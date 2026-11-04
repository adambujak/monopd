//! The game state machine.
//!
//! The engine never asks for input. Instead:
//! - [`Game::current_player`] says who must decide next. This can be someone
//!   other than the active player, e.g. a player paying rent or deciding
//!   whether to play Just Say No.
//! - [`Game::legal_actions`] lists every move allowed right now.
//! - [`Game::step`] applies one of them and advances the game.
//!
//! The state is plain data, so `clone()` gives a full copy for search.

use crate::action::{Action, MAX_PLAYERS};
use crate::cards::*;
use crate::rng::Rng;
use std::fmt::Write as _;

pub const MIN_PLAYERS: usize = 2;
pub const STARTING_HAND: usize = 5;
pub const HAND_LIMIT: usize = 7;
pub const PLAYS_PER_TURN: u8 = 3;
pub const SETS_TO_WIN: usize = 3;
pub const DEFAULT_MAX_TURNS: u32 = 500;
pub const DEBT_COLLECTOR_AMOUNT: u8 = 5;
pub const BIRTHDAY_AMOUNT: u8 = 2;

const JSN: Kind = Kind::Action(ActionType::JustSayNo);
const DOUBLE: Kind = Kind::Action(ActionType::DoubleRent);
const RAINBOW: Kind = Kind::Wild(WildType::Rainbow);

/// All properties of one colour on a player's table, plus buildings.
///
/// Each colour is a single pile. Any count >= the set size counts as one
/// complete set (extra cards are protected along with it).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pile {
    pub cards: Vec<CardId>,
    pub house: Option<CardId>,
    pub hotel: Option<CardId>,
}

impl Pile {
    pub fn len(&self) -> usize {
        self.cards.len()
    }
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }
    pub fn position(&self, kind: Kind) -> Option<usize> {
        self.cards.iter().position(|&c| kind_of(c) == kind)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Player {
    pub hand: Vec<CardId>,
    pub bank: Vec<CardId>,
    pub table: [Pile; NUM_COLORS],
}

impl Player {
    pub fn pile(&self, color: Color) -> &Pile {
        &self.table[color.idx()]
    }
    pub fn is_complete(&self, color: Color) -> bool {
        pile_complete(self.pile(color), color)
    }
    /// Does this colour hold at least one card that really is that colour?
    /// A rainbow wild has no colour on its own, so a pile of only rainbows
    /// can't charge rent or count as a set.
    pub fn has_real(&self, color: Color) -> bool {
        pile_has_real(self.pile(color))
    }
    pub fn complete_sets(&self) -> usize {
        COLORS.iter().filter(|&&c| self.is_complete(c)).count()
    }
    pub fn has_complete_set(&self) -> bool {
        COLORS.iter().any(|&c| self.is_complete(c))
    }
    /// Can Sly Deal / Forced Deal take something from this player?
    pub fn has_stealable(&self) -> bool {
        COLORS.iter().any(|&c| is_stealable(self.pile(c), c))
    }
    /// Rent this player would charge for `color`.
    pub fn rent(&self, color: Color) -> u8 {
        let pile = self.pile(color);
        if !pile_has_real(pile) {
            return 0;
        }
        let mut r = color.rent(pile.len());
        if self.is_complete(color) {
            if pile.house.is_some() {
                r += HOUSE_RENT;
            }
            if pile.hotel.is_some() {
                r += HOTEL_RENT;
            }
        }
        r
    }
    pub fn bank_value(&self) -> u32 {
        self.bank.iter().map(|&c| kind_of(c).value() as u32).sum()
    }
    /// Total value this player could hand over as payment.
    pub fn payable_value(&self) -> u32 {
        let mut v = self.bank_value();
        for pile in &self.table {
            v += pile.cards.iter().map(|&c| kind_of(c).value() as u32).sum::<u32>();
            v += pile.house.map_or(0, |c| kind_of(c).value() as u32);
            v += pile.hotel.map_or(0, |c| kind_of(c).value() as u32);
        }
        v
    }
    pub fn holds(&self, kind: Kind) -> bool {
        self.hand.iter().any(|&c| kind_of(c) == kind)
    }
    pub fn count_in_hand(&self, kind: Kind) -> usize {
        self.hand.iter().filter(|&&c| kind_of(c) == kind).count()
    }
    fn take_from_hand(&mut self, kind: Kind) -> CardId {
        let i = self.hand.iter().position(|&c| kind_of(c) == kind).expect("card kind not in hand");
        self.hand.swap_remove(i)
    }
}

fn is_stealable(pile: &Pile, color: Color) -> bool {
    !pile.is_empty() && !pile_complete(pile, color)
}

fn pile_has_real(pile: &Pile) -> bool {
    pile.cards.iter().any(|&c| kind_of(c) != RAINBOW)
}

fn pile_complete(pile: &Pile, color: Color) -> bool {
    pile.len() >= color.set_size() && pile_has_real(pile)
}

/// Which target the active player is picking.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TargetChoice {
    RentPlayer(Color),
    Debt,
    Sly,
    ForcedTake,
    ForcedGive { target: usize, card: CardId, color: Color },
    DealBreaker,
    House { card: CardId },
    Hotel { card: CardId },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Active player plays cards / ends the turn.
    Main,
    /// Active player is over the hand limit at end of turn.
    Discard,
    /// Active player picks a target for the card just played.
    ChooseTarget(TargetChoice),
    /// Active player may add a Double The Rent to the rent just played.
    OfferDouble { color: Color, target: Option<usize>, multiplier: u8 },
    /// Just Say No chain against the current victim. Even count: the victim
    /// decides; odd: the actor decides whether to counter.
    Respond { jsn_count: u8 },
    /// The current victim is choosing cards to pay with.
    Pay { owed: u8 },
    GameOver,
}

/// What happens to each victim once any Just Say No chain is settled.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    Charge(u8),
    Sly { card: CardId, color: Color },
    Forced { theirs: CardId, their_color: Color, mine: CardId, my_color: Color },
    DealBreaker(Color),
}

/// An action card being resolved against one or more victims, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolution {
    pub actor: usize,
    pub effect: Effect,
    pub victims: Vec<usize>,
    pub idx: usize,
}

impl Resolution {
    pub fn victim(&self) -> usize {
        self.victims[self.idx]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IllegalAction(pub Action);

impl std::fmt::Display for IllegalAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "illegal action: {}", self.0)
    }
}
impl std::error::Error for IllegalAction {}

#[derive(Clone, Debug)]
pub struct Game {
    pub num_players: usize,
    pub players: Vec<Player>,
    /// Draw pile; cards are drawn from the end.
    pub deck: Vec<CardId>,
    pub discard: Vec<CardId>,
    pub active: usize,
    pub plays_left: u8,
    pub phase: Phase,
    pub pending: Option<Resolution>,
    /// Number of turns started so far (0-based index of the current turn).
    pub turn: u32,
    /// The game ends in a draw when this many turns have been played.
    pub max_turns: u32,
    pub winner: Option<usize>,
    pub rng: Rng,
}

#[inline]
fn mark(seen: &mut u64, k: Kind) -> bool {
    let b = 1u64 << k.index();
    let fresh = *seen & b == 0;
    *seen |= b;
    fresh
}

impl Game {
    /// A new game: shuffled deck, 5 cards each, player 0's turn has begun.
    pub fn new(num_players: usize, seed: u64) -> Game {
        let mut g = Game::blank(num_players, seed);
        let mut deck = std::mem::take(&mut g.deck);
        g.rng.shuffle(&mut deck);
        g.deck = deck;
        for _ in 0..STARTING_HAND {
            for p in 0..num_players {
                g.draw(p, 1);
            }
        }
        g.start_turn(0);
        g
    }

    /// Unshuffled deck, empty hands, player 0 in the Main phase with 3 plays
    /// and nothing drawn. For setting up scenarios in tests.
    #[doc(hidden)]
    pub fn new_test(num_players: usize) -> Game {
        Game::blank(num_players, 0)
    }

    fn blank(num_players: usize, seed: u64) -> Game {
        assert!(
            (MIN_PLAYERS..=MAX_PLAYERS).contains(&num_players),
            "Monopoly Deal supports 2-5 players"
        );
        Game {
            num_players,
            players: vec![Player::default(); num_players],
            deck: (0..DECK_SIZE as CardId).collect(),
            discard: Vec::new(),
            active: 0,
            plays_left: PLAYS_PER_TURN,
            phase: Phase::Main,
            pending: None,
            turn: 0,
            max_turns: DEFAULT_MAX_TURNS,
            winner: None,
            rng: Rng::new(seed),
        }
    }

    // ------------------------------------------------------------------
    // Queries
    // ------------------------------------------------------------------

    pub fn is_over(&self) -> bool {
        self.phase == Phase::GameOver
    }

    /// The player who must act next, or `None` when the game is over.
    pub fn current_player(&self) -> Option<usize> {
        match self.phase {
            Phase::GameOver => None,
            Phase::Respond { jsn_count } => Some(self.responder(jsn_count)),
            Phase::Pay { .. } => Some(self.pending.as_ref().unwrap().victim()),
            _ => Some(self.active),
        }
    }

    /// Final reward per player: +1 winner, -1 others, 0 for everyone on a draw.
    pub fn returns(&self) -> Vec<f32> {
        match self.winner {
            Some(w) => (0..self.num_players).map(|p| if p == w { 1.0 } else { -1.0 }).collect(),
            None => vec![0.0; self.num_players],
        }
    }

    /// Seat of player `rel` places after `from`.
    pub fn seat(&self, from: usize, rel: u8) -> usize {
        (from + rel as usize) % self.num_players
    }

    /// Relative position of `to` as seen from `from`.
    pub fn rel(&self, from: usize, to: usize) -> u8 {
        ((to + self.num_players - from) % self.num_players) as u8
    }

    fn opponents(&self, of: usize) -> impl Iterator<Item = usize> {
        let n = self.num_players;
        (1..n).map(move |r| (of + r) % n)
    }

    fn responder(&self, jsn_count: u8) -> usize {
        let r = self.pending.as_ref().unwrap();
        if jsn_count % 2 == 0 { r.victim() } else { r.actor }
    }

    // ------------------------------------------------------------------
    // Legal actions
    // ------------------------------------------------------------------

    pub fn legal_actions(&self) -> Vec<Action> {
        let mut out = Vec::with_capacity(32);
        self.legal_actions_into(&mut out);
        out
    }

    /// Legal actions as a mask over `0..NUM_ACTIONS`.
    pub fn legal_mask(&self) -> Vec<bool> {
        let mut mask = vec![false; crate::action::NUM_ACTIONS];
        for a in self.legal_actions() {
            mask[a.index()] = true;
        }
        mask
    }

    /// Like [`Game::legal_actions`] but reuses a buffer.
    pub fn legal_actions_into(&self, out: &mut Vec<Action>) {
        out.clear();
        match self.phase {
            Phase::GameOver => {}
            Phase::Main => self.main_actions(out),
            Phase::Discard => {
                let mut seen = 0u64;
                for &c in &self.players[self.active].hand {
                    let k = kind_of(c);
                    if mark(&mut seen, k) {
                        out.push(Action::Discard(k));
                    }
                }
            }
            Phase::ChooseTarget(choice) => self.target_actions(choice, out),
            // These questions are asked whether or not the player holds the
            // card (asking only holders would reveal their hand); without the
            // card the only answer is no.
            Phase::OfferDouble { .. } => {
                if self.players[self.active].holds(DOUBLE) {
                    out.push(Action::PlayDouble);
                }
                out.push(Action::DeclineDouble);
            }
            Phase::Respond { jsn_count } => {
                if self.players[self.responder(jsn_count)].holds(JSN) {
                    out.push(Action::JustSayNo);
                }
                out.push(Action::Accept);
            }
            Phase::Pay { .. } => self.pay_actions(out),
        }
    }

    fn main_actions(&self, out: &mut Vec<Action>) {
        let a = self.active;
        let p = &self.players[a];
        out.push(Action::EndTurn);
        if self.plays_left > 0 {
            let mut seen = 0u64;
            for &c in &p.hand {
                let k = kind_of(c);
                if !mark(&mut seen, k) {
                    continue;
                }
                match k {
                    Kind::Money(_) => out.push(Action::Bank(k)),
                    Kind::Property(_) | Kind::Wild(_) => {
                        for color in iter_colors(k.colors()) {
                            out.push(Action::PlaceProperty { kind: k, color });
                        }
                    }
                    Kind::Action(t) => {
                        out.push(Action::Bank(k));
                        if self.can_play(a, t) {
                            out.push(Action::PlayAction(t));
                        }
                    }
                    Kind::Rent(r) => {
                        out.push(Action::Bank(k));
                        for color in iter_colors(r.colors()) {
                            if p.has_real(color) {
                                out.push(Action::PlayRent { color, wild: r == RentType::Wild });
                            }
                        }
                    }
                }
            }
        }
        if self.plays_left > 0 {
            for from in COLORS {
                let mut seen = 0u64;
                for &c in &p.pile(from).cards {
                    let k = kind_of(c);
                    if k.is_wild() && mark(&mut seen, k) {
                        for to in iter_colors(k.colors()) {
                            if to != from {
                                out.push(Action::MoveWild { kind: k, from, to });
                            }
                        }
                    }
                }
            }
        }
    }

    fn can_play(&self, a: usize, t: ActionType) -> bool {
        let p = &self.players[a];
        match t {
            ActionType::JustSayNo | ActionType::DoubleRent => false,
            ActionType::PassGo | ActionType::Birthday | ActionType::DebtCollector => true,
            ActionType::DealBreaker => self.opponents(a).any(|o| self.players[o].has_complete_set()),
            ActionType::SlyDeal => self.opponents(a).any(|o| self.players[o].has_stealable()),
            ActionType::ForcedDeal => {
                p.has_stealable() && self.opponents(a).any(|o| self.players[o].has_stealable())
            }
            ActionType::House => COLORS.iter().any(|&c| can_build_house(p, c)),
            ActionType::Hotel => COLORS.iter().any(|&c| can_build_hotel(p, c)),
        }
    }

    fn push_stealable(&self, owner: usize, rel: u8, out: &mut Vec<Action>) {
        for color in COLORS {
            let pile = self.players[owner].pile(color);
            if !is_stealable(pile, color) {
                continue;
            }
            let mut seen = 0u64;
            for &c in &pile.cards {
                let kind = kind_of(c);
                if mark(&mut seen, kind) {
                    out.push(Action::TargetProperty { rel, kind, color });
                }
            }
        }
    }

    fn target_actions(&self, choice: TargetChoice, out: &mut Vec<Action>) {
        let a = self.active;
        let n = self.num_players;
        match choice {
            TargetChoice::RentPlayer(_) | TargetChoice::Debt => {
                for r in 1..n {
                    out.push(Action::TargetPlayer(r as u8));
                }
            }
            TargetChoice::Sly | TargetChoice::ForcedTake => {
                for r in 1..n {
                    self.push_stealable(self.seat(a, r as u8), r as u8, out);
                }
            }
            TargetChoice::ForcedGive { .. } => self.push_stealable(a, 0, out),
            TargetChoice::DealBreaker => {
                for r in 1..n {
                    let o = self.seat(a, r as u8);
                    for color in COLORS {
                        if self.players[o].is_complete(color) {
                            out.push(Action::TargetSet { rel: r as u8, color });
                        }
                    }
                }
            }
            TargetChoice::House { .. } => {
                for color in COLORS {
                    if can_build_house(&self.players[a], color) {
                        out.push(Action::TargetSet { rel: 0, color });
                    }
                }
            }
            TargetChoice::Hotel { .. } => {
                for color in COLORS {
                    if can_build_hotel(&self.players[a], color) {
                        out.push(Action::TargetSet { rel: 0, color });
                    }
                }
            }
        }
    }

    fn pay_actions(&self, out: &mut Vec<Action>) {
        let payer = self.pending.as_ref().unwrap().victim();
        let p = &self.players[payer];
        let mut seen = 0u64;
        for &c in &p.bank {
            let k = kind_of(c);
            if mark(&mut seen, k) {
                out.push(Action::PayBank(k));
            }
        }
        for color in COLORS {
            let pile = p.pile(color);
            let mut seen = 0u64;
            for &c in &pile.cards {
                let k = kind_of(c);
                if k.value() > 0 && mark(&mut seen, k) {
                    out.push(Action::PayProperty { kind: k, color });
                }
            }
            if pile.hotel.is_some() {
                out.push(Action::PayBuilding { color, hotel: true });
            } else if pile.house.is_some() {
                out.push(Action::PayBuilding { color, hotel: false });
            }
        }
    }

    // ------------------------------------------------------------------
    // Applying actions
    // ------------------------------------------------------------------

    /// Apply `action` if it is legal.
    pub fn step(&mut self, action: Action) -> Result<(), IllegalAction> {
        if !self.legal_actions().contains(&action) {
            return Err(IllegalAction(action));
        }
        self.apply(action);
        Ok(())
    }

    /// Apply the action with flat index `idx` if it is legal.
    pub fn step_index(&mut self, idx: usize) -> Result<(), IllegalAction> {
        match Action::from_index(idx) {
            Some(a) => self.step(a),
            None => Err(IllegalAction(Action::EndTurn)),
        }
    }

    /// Apply an action the caller *knows* is legal (taken from
    /// `legal_actions`). Skips the legality check; an illegal action may panic
    /// or corrupt the state.
    pub fn apply_unchecked(&mut self, action: Action) {
        self.apply(action);
    }

    fn apply(&mut self, action: Action) {
        let a = self.active;
        match action {
            Action::EndTurn => self.end_turn(),
            Action::Bank(k) => {
                let c = self.players[a].take_from_hand(k);
                self.players[a].bank.push(c);
                self.plays_left -= 1;
            }
            Action::PlaceProperty { kind, color } => {
                let c = self.players[a].take_from_hand(kind);
                self.players[a].table[color.idx()].cards.push(c);
                self.plays_left -= 1;
                self.check_win();
            }
            Action::MoveWild { kind, from, to } => {
                let pile = &mut self.players[a].table[from.idx()];
                let i = pile.position(kind).expect("wild not on table");
                let c = pile.cards.remove(i);
                self.fix_buildings(a, from);
                self.players[a].table[to.idx()].cards.push(c);
                self.plays_left -= 1;
                self.check_win();
            }
            Action::PlayAction(t) => self.play_action(t),
            Action::PlayRent { color, wild } => {
                let rent = if wild { RentType::Wild } else { RentType::for_color(color) };
                let c = self.players[a].take_from_hand(Kind::Rent(rent));
                self.discard.push(c);
                self.plays_left -= 1;
                if wild {
                    self.phase = Phase::ChooseTarget(TargetChoice::RentPlayer(color));
                } else {
                    self.offer_double(color, None, 1);
                }
            }
            Action::TargetPlayer(rel) => {
                let target = self.seat(a, rel);
                match self.phase {
                    Phase::ChooseTarget(TargetChoice::RentPlayer(color)) => {
                        self.offer_double(color, Some(target), 1)
                    }
                    Phase::ChooseTarget(TargetChoice::Debt) => {
                        self.start_resolution(Effect::Charge(DEBT_COLLECTOR_AMOUNT), vec![target])
                    }
                    _ => unreachable!("TargetPlayer in {:?}", self.phase),
                }
            }
            Action::TargetProperty { rel, kind, color } => {
                let owner = self.seat(a, rel);
                let pile = self.players[owner].pile(color);
                let card = pile.cards[pile.position(kind).expect("target property missing")];
                match self.phase {
                    Phase::ChooseTarget(TargetChoice::Sly) => {
                        self.start_resolution(Effect::Sly { card, color }, vec![owner])
                    }
                    Phase::ChooseTarget(TargetChoice::ForcedTake) => {
                        self.phase = Phase::ChooseTarget(TargetChoice::ForcedGive {
                            target: owner,
                            card,
                            color,
                        })
                    }
                    Phase::ChooseTarget(TargetChoice::ForcedGive { target, card: theirs, color: their_color }) => {
                        let effect = Effect::Forced { theirs, their_color, mine: card, my_color: color };
                        self.start_resolution(effect, vec![target])
                    }
                    _ => unreachable!("TargetProperty in {:?}", self.phase),
                }
            }
            Action::TargetSet { rel, color } => match self.phase {
                Phase::ChooseTarget(TargetChoice::DealBreaker) => {
                    let owner = self.seat(a, rel);
                    self.start_resolution(Effect::DealBreaker(color), vec![owner])
                }
                Phase::ChooseTarget(TargetChoice::House { card }) => {
                    self.players[a].table[color.idx()].house = Some(card);
                    self.phase = Phase::Main;
                }
                Phase::ChooseTarget(TargetChoice::Hotel { card }) => {
                    self.players[a].table[color.idx()].hotel = Some(card);
                    self.phase = Phase::Main;
                }
                _ => unreachable!("TargetSet in {:?}", self.phase),
            },
            Action::PlayDouble => {
                let Phase::OfferDouble { color, target, multiplier } = self.phase else { unreachable!() };
                let c = self.players[a].take_from_hand(DOUBLE);
                self.discard.push(c);
                self.plays_left -= 1;
                self.offer_double(color, target, multiplier * 2);
            }
            Action::DeclineDouble => {
                let Phase::OfferDouble { color, target, multiplier } = self.phase else { unreachable!() };
                self.resolve_rent(color, target, multiplier);
            }
            Action::JustSayNo => {
                let Phase::Respond { jsn_count } = self.phase else { unreachable!() };
                let responder = self.responder(jsn_count);
                let c = self.players[responder].take_from_hand(JSN);
                self.discard.push(c);
                // The other side always gets the chance to answer it.
                self.phase = Phase::Respond { jsn_count: jsn_count + 1 };
            }
            Action::Accept => {
                let Phase::Respond { jsn_count } = self.phase else { unreachable!() };
                self.finish_chain(jsn_count);
            }
            Action::PayBank(_) | Action::PayProperty { .. } | Action::PayBuilding { .. } => self.pay(action),
            Action::Discard(k) => {
                let c = self.players[a].take_from_hand(k);
                self.discard.push(c);
                if self.players[a].hand.len() <= HAND_LIMIT {
                    self.next_turn();
                }
            }
        }
        // With no plays left the only legal move would be EndTurn, so take it.
        // This also covers the moment a rent/steal finishes resolving.
        if self.phase == Phase::Main && self.plays_left == 0 {
            self.end_turn();
        }
    }

    fn play_action(&mut self, t: ActionType) {
        let a = self.active;
        let c = self.players[a].take_from_hand(Kind::Action(t));
        self.plays_left -= 1;
        match t {
            ActionType::House => {
                self.phase = Phase::ChooseTarget(TargetChoice::House { card: c });
                return;
            }
            ActionType::Hotel => {
                self.phase = Phase::ChooseTarget(TargetChoice::Hotel { card: c });
                return;
            }
            _ => self.discard.push(c),
        }
        match t {
            ActionType::PassGo => self.draw(a, 2),
            ActionType::Birthday => {
                let victims = self.opponents(a).collect();
                self.start_resolution(Effect::Charge(BIRTHDAY_AMOUNT), victims);
            }
            ActionType::DebtCollector => self.phase = Phase::ChooseTarget(TargetChoice::Debt),
            ActionType::SlyDeal => self.phase = Phase::ChooseTarget(TargetChoice::Sly),
            ActionType::ForcedDeal => self.phase = Phase::ChooseTarget(TargetChoice::ForcedTake),
            ActionType::DealBreaker => self.phase = Phase::ChooseTarget(TargetChoice::DealBreaker),
            _ => unreachable!("{t:?} is not playable"),
        }
    }

    fn offer_double(&mut self, color: Color, target: Option<usize>, multiplier: u8) {
        if self.plays_left > 0 {
            self.phase = Phase::OfferDouble { color, target, multiplier };
        } else {
            self.resolve_rent(color, target, multiplier);
        }
    }

    fn resolve_rent(&mut self, color: Color, target: Option<usize>, multiplier: u8) {
        let a = self.active;
        let amount = (self.players[a].rent(color) as u32 * multiplier as u32).min(u8::MAX as u32) as u8;
        let victims = match target {
            Some(t) => vec![t],
            None => self.opponents(a).collect(),
        };
        self.start_resolution(Effect::Charge(amount), victims);
    }

    fn start_resolution(&mut self, effect: Effect, victims: Vec<usize>) {
        self.pending = Some(Resolution { actor: self.active, effect, victims, idx: 0 });
        self.process_victims();
    }

    /// Work through victims until someone has to make a decision.
    fn process_victims(&mut self) {
        if self.is_over() {
            return;
        }
        let r = self.pending.as_ref().expect("no pending resolution");
        if r.idx >= r.victims.len() {
            self.pending = None;
            self.phase = Phase::Main;
        } else {
            // Every victim is asked about Just Say No, holder or not.
            self.phase = Phase::Respond { jsn_count: 0 };
        }
    }

    fn finish_chain(&mut self, jsn_count: u8) {
        if jsn_count % 2 == 0 && (self.apply_effect() || self.is_over()) {
            return;
        }
        self.next_victim();
    }

    fn next_victim(&mut self) {
        self.pending.as_mut().unwrap().idx += 1;
        self.process_victims();
    }

    /// Apply the pending effect to the current victim. Returns true if we now
    /// wait for that victim to choose how to pay.
    fn apply_effect(&mut self) -> bool {
        let r = self.pending.as_ref().unwrap();
        let (actor, victim, effect) = (r.actor, r.victim(), r.effect);
        match effect {
            Effect::Charge(owed) => return self.begin_payment(victim, actor, owed),
            Effect::Sly { card, color } => self.move_property(victim, actor, card, color),
            Effect::Forced { theirs, their_color, mine, my_color } => {
                self.move_property(victim, actor, theirs, their_color);
                self.move_property(actor, victim, mine, my_color);
            }
            Effect::DealBreaker(color) => {
                let stolen = std::mem::take(&mut self.players[victim].table[color.idx()]);
                let dst = &mut self.players[actor].table[color.idx()];
                dst.cards.extend(stolen.cards);
                let mut extra = Vec::new();
                match (dst.house, stolen.house) {
                    (None, h) => dst.house = h,
                    (Some(_), Some(h)) => extra.push(h),
                    _ => {}
                }
                match (dst.hotel, stolen.hotel) {
                    (None, h) => dst.hotel = h,
                    (Some(_), Some(h)) => extra.push(h),
                    _ => {}
                }
                self.players[actor].bank.extend(extra);
            }
        }
        self.check_win();
        false
    }

    /// Move one property card between tables, keeping its colour.
    fn move_property(&mut self, from: usize, to: usize, card: CardId, color: Color) {
        let pile = &mut self.players[from].table[color.idx()];
        let i = pile.cards.iter().position(|&c| c == card).expect("property moved away");
        pile.cards.remove(i);
        self.fix_buildings(from, color);
        self.players[to].table[color.idx()].cards.push(card);
    }

    /// Buildings on a set that is no longer complete go to the owner's bank.
    fn fix_buildings(&mut self, p: usize, color: Color) {
        let player = &mut self.players[p];
        if player.is_complete(color) {
            return;
        }
        let pile = &mut player.table[color.idx()];
        let (house, hotel) = (pile.house.take(), pile.hotel.take());
        player.bank.extend(house);
        player.bank.extend(hotel);
    }

    fn begin_payment(&mut self, payer: usize, receiver: usize, owed: u8) -> bool {
        let total = self.players[payer].payable_value();
        if owed == 0 || total == 0 {
            return false;
        }
        if total <= owed as u32 {
            self.pay_everything(payer, receiver);
            return false;
        }
        self.phase = Phase::Pay { owed };
        true
    }

    fn pay(&mut self, action: Action) {
        let Phase::Pay { owed } = self.phase else { unreachable!() };
        let r = self.pending.as_ref().unwrap();
        let (payer, receiver) = (r.victim(), r.actor);
        let value = match action {
            Action::PayBank(k) => {
                let bank = &mut self.players[payer].bank;
                // Pay with the most recently banked card of this kind, keeping the
                // bank in deposit order (the UI shows the newest card on top).
                let i = bank.iter().rposition(|&c| kind_of(c) == k).expect("not in bank");
                let c = bank.remove(i);
                self.players[receiver].bank.push(c);
                k.value()
            }
            Action::PayProperty { kind, color } => {
                let pile = self.players[payer].pile(color);
                let card = pile.cards[pile.position(kind).expect("not on table")];
                self.move_property(payer, receiver, card, color);
                kind.value()
            }
            Action::PayBuilding { color, hotel } => {
                let pile = &mut self.players[payer].table[color.idx()];
                let c = if hotel { pile.hotel.take() } else { pile.house.take() }.expect("no building");
                self.players[receiver].bank.push(c);
                kind_of(c).value()
            }
            _ => unreachable!(),
        };
        if self.check_win() {
            return;
        }
        let remaining = owed.saturating_sub(value);
        let total = self.players[payer].payable_value();
        if remaining > 0 && total > remaining as u32 {
            self.phase = Phase::Pay { owed: remaining };
            return;
        }
        if remaining > 0 && total > 0 {
            self.pay_everything(payer, receiver);
            if self.is_over() {
                return;
            }
        }
        self.next_victim();
    }

    /// Hand over everything of value (rainbow wilds are worth $0 and stay).
    fn pay_everything(&mut self, payer: usize, receiver: usize) {
        let bank = std::mem::take(&mut self.players[payer].bank);
        self.players[receiver].bank.extend(bank);
        for color in COLORS {
            let pile = &mut self.players[payer].table[color.idx()];
            let buildings: Vec<CardId> = pile.house.take().into_iter().chain(pile.hotel.take()).collect();
            let (give, keep): (Vec<CardId>, Vec<CardId>) =
                pile.cards.iter().partition(|&&c| kind_of(c).value() > 0);
            pile.cards = keep;
            self.players[receiver].bank.extend(buildings);
            self.players[receiver].table[color.idx()].cards.extend(give);
        }
        self.check_win();
    }

    fn check_win(&mut self) -> bool {
        if self.winner.is_some() {
            return true;
        }
        // A Forced Deal can complete a third set for both players at once; the
        // player whose turn it is gets the win, then seat order after them.
        let n = self.num_players;
        let order = (0..n).map(|r| (self.active + r) % n);
        if let Some(w) = order.clone().find(|&p| self.players[p].complete_sets() >= SETS_TO_WIN) {
            self.winner = Some(w);
            self.phase = Phase::GameOver;
            self.pending = None;
            return true;
        }
        false
    }

    fn end_turn(&mut self) {
        if self.players[self.active].hand.len() > HAND_LIMIT {
            self.phase = Phase::Discard;
        } else {
            self.next_turn();
        }
    }

    fn next_turn(&mut self) {
        self.turn += 1;
        if self.turn >= self.max_turns {
            self.phase = Phase::GameOver;
            return;
        }
        self.start_turn((self.active + 1) % self.num_players);
    }

    fn start_turn(&mut self, p: usize) {
        self.active = p;
        self.plays_left = PLAYS_PER_TURN;
        self.phase = Phase::Main;
        let n = if self.players[p].hand.is_empty() { 5 } else { 2 };
        self.draw(p, n);
    }

    /// Draw `n` cards, reshuffling the discard pile into the deck if needed.
    fn draw(&mut self, p: usize, n: usize) {
        for _ in 0..n {
            if self.deck.is_empty() {
                if self.discard.is_empty() {
                    return;
                }
                let mut deck = std::mem::take(&mut self.discard);
                self.rng.shuffle(&mut deck);
                self.deck = deck;
            }
            let c = self.deck.pop().unwrap();
            self.players[p].hand.push(c);
        }
    }

    // ------------------------------------------------------------------
    // Search support
    // ------------------------------------------------------------------

    /// A copy of the game where everything `seat` can't see (other players'
    /// hands and the deck order) is re-dealt at random, keeping hand sizes.
    /// Used by Information-Set MCTS.
    pub fn determinize(&self, seat: usize, seed: u64) -> Game {
        let mut g = self.clone();
        let mut rng = Rng::new(seed);
        // Nothing about the phase reveals a hand: Just Say No and Double The
        // Rent questions are asked whether or not the player holds the card.
        let mut pool = std::mem::take(&mut g.deck);
        let mut sizes = vec![0; g.num_players];
        for p in 0..g.num_players {
            if p != seat {
                sizes[p] = g.players[p].hand.len();
                pool.append(&mut g.players[p].hand);
            }
        }
        rng.shuffle(&mut pool);
        for p in 0..g.num_players {
            if p != seat {
                let at = pool.len() - sizes[p];
                g.players[p].hand = pool.split_off(at);
            }
        }
        g.deck = pool;
        g.rng = Rng::new(rng.next_u64());
        g
    }

    // ------------------------------------------------------------------
    // Test / debug helpers
    // ------------------------------------------------------------------

    fn take_from_deck(&mut self, kind: Kind) -> CardId {
        let i = self
            .deck
            .iter()
            .position(|&c| kind_of(c) == kind)
            .unwrap_or_else(|| panic!("no {kind} left in deck"));
        self.deck.remove(i)
    }

    #[doc(hidden)]
    pub fn give_hand(&mut self, p: usize, kind: Kind) -> CardId {
        let c = self.take_from_deck(kind);
        self.players[p].hand.push(c);
        c
    }

    #[doc(hidden)]
    pub fn give_bank(&mut self, p: usize, kind: Kind) -> CardId {
        let c = self.take_from_deck(kind);
        self.players[p].bank.push(c);
        c
    }

    #[doc(hidden)]
    pub fn give_table(&mut self, p: usize, kind: Kind, color: Color) -> CardId {
        assert!(kind.colors() & color.bit() != 0, "{kind} can't be {color}");
        let c = self.take_from_deck(kind);
        self.players[p].table[color.idx()].cards.push(c);
        c
    }

    #[doc(hidden)]
    pub fn give_building(&mut self, p: usize, color: Color, hotel: bool) -> CardId {
        let kind = Kind::Action(if hotel { ActionType::Hotel } else { ActionType::House });
        let c = self.take_from_deck(kind);
        let pile = &mut self.players[p].table[color.idx()];
        if hotel { pile.hotel = Some(c) } else { pile.house = Some(c) }
        c
    }

    /// Panics unless every card is in exactly one place and the state is
    /// consistent. Cheap enough to run after every step in fuzz tests.
    pub fn check_invariants(&self) {
        let mut seen = [0u8; DECK_SIZE];
        let mut see = |c: CardId| seen[c as usize] += 1;
        self.deck.iter().for_each(|&c| see(c));
        self.discard.iter().for_each(|&c| see(c));
        // A House/Hotel being placed is held by the phase until its target is picked.
        if let Phase::ChooseTarget(TargetChoice::House { card } | TargetChoice::Hotel { card }) = self.phase {
            see(card);
        }
        for p in &self.players {
            p.hand.iter().for_each(|&c| see(c));
            p.bank.iter().for_each(|&c| see(c));
            for (i, pile) in p.table.iter().enumerate() {
                let color = COLORS[i];
                for &c in &pile.cards {
                    see(c);
                    assert!(kind_of(c).colors() & color.bit() != 0, "card {c} misplaced in {color}");
                }
                if let Some(h) = pile.house {
                    see(h);
                    assert_eq!(kind_of(h), Kind::Action(ActionType::House));
                    assert!(p.is_complete(color) && color.allows_buildings(), "house on bad set");
                }
                if let Some(h) = pile.hotel {
                    see(h);
                    assert_eq!(kind_of(h), Kind::Action(ActionType::Hotel));
                    assert!(pile.house.is_some(), "hotel without house");
                }
            }
            for &c in &p.bank {
                assert!(!kind_of(c).is_property(), "property in bank");
            }
        }
        if let Some(bad) = seen.iter().position(|&n| n != 1) {
            panic!("card {bad} appears {} times", seen[bad]);
        }
        // Phase-specific consistency.
        match self.phase {
            Phase::GameOver => {
                assert!(self.pending.is_none());
                if let Some(w) = self.winner {
                    assert!(self.players[w].complete_sets() >= SETS_TO_WIN, "winner without 3 sets");
                } else {
                    assert!(self.turn >= self.max_turns, "game over without a winner before the turn limit");
                }
            }
            Phase::Main => {
                assert!(self.pending.is_none(), "Main phase with a pending resolution");
                assert!(self.plays_left > 0, "Main phase with no plays left should have ended the turn");
            }
            Phase::Discard => {
                assert!(self.players[self.active].hand.len() > HAND_LIMIT, "Discard phase at or under the limit");
            }
            Phase::ChooseTarget(_) => assert!(self.pending.is_none()),
            Phase::OfferDouble { multiplier, .. } => {
                assert!(self.plays_left > 0, "Double offered with no plays left");
                assert!(matches!(multiplier, 1 | 2), "more doubles than plays allow");
            }
            Phase::Respond { jsn_count } => {
                let r = self.pending.as_ref().expect("Respond without a pending resolution");
                assert!(r.idx < r.victims.len());
                let _ = jsn_count;
            }
            Phase::Pay { owed } => {
                let r = self.pending.as_ref().expect("Pay without a pending resolution");
                assert!(matches!(r.effect, Effect::Charge(_)));
                assert!(owed > 0);
                assert!(
                    self.players[r.victim()].payable_value() > owed as u32,
                    "payer should have paid everything automatically"
                );
            }
        }
        if let Some(w) = self.winner {
            assert!(self.is_over());
            let _ = w;
        } else {
            for p in &self.players {
                assert!(p.complete_sets() < SETS_TO_WIN || self.is_over(), "unnoticed winner");
            }
        }
        assert!(self.players.iter().all(|p| p.hand.len() <= DECK_SIZE));
        if !self.is_over() {
            assert!(!self.legal_actions().is_empty(), "no legal actions in {:?}", self.phase);
        }
    }

    /// Human-readable view of the table from `seat`'s perspective.
    pub fn describe(&self, seat: usize) -> String {
        let mut s = String::new();
        let _ = writeln!(
            s,
            "Turn {}  |  deck {}  discard {}  |  P{} to act, plays left {}",
            self.turn,
            self.deck.len(),
            self.discard.len(),
            self.active,
            self.plays_left
        );
        for (i, p) in self.players.iter().enumerate() {
            let me = if i == seat { " (you)" } else { "" };
            let _ = writeln!(
                s,
                "P{i}{me}: hand {} cards, bank ${}M, {} complete set(s)",
                p.hand.len(),
                p.bank_value(),
                p.complete_sets()
            );
            for color in COLORS {
                let pile = p.pile(color);
                if pile.is_empty() && pile.house.is_none() {
                    continue;
                }
                let cards: Vec<String> = pile.cards.iter().map(|&c| kind_of(c).to_string()).collect();
                let mut extra = String::new();
                if pile.house.is_some() {
                    extra.push_str(" +house");
                }
                if pile.hotel.is_some() {
                    extra.push_str(" +hotel");
                }
                let _ = writeln!(
                    s,
                    "    {color:<9} {}/{}{extra}: {}",
                    pile.len(),
                    color.set_size(),
                    cards.join(", ")
                );
            }
        }
        let hand: Vec<String> = self.players[seat].hand.iter().map(|&c| kind_of(c).to_string()).collect();
        let _ = writeln!(s, "Your hand: {}", hand.join(", "));
        if let Some(r) = &self.pending {
            let _ = writeln!(s, "Resolving {:?} from P{} against P{}", r.effect, r.actor, r.victim());
        }
        s
    }
}

fn can_build_house(p: &Player, c: Color) -> bool {
    c.allows_buildings() && p.is_complete(c) && p.pile(c).house.is_none()
}

fn can_build_hotel(p: &Player, c: Color) -> bool {
    c.allows_buildings() && p.is_complete(c) && p.pile(c).house.is_some() && p.pile(c).hotel.is_none()
}
