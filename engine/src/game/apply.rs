//! Applying an action, and the turn flow (draw, end turn, discard).

use super::{
    BIRTHDAY_AMOUNT, DEBT_COLLECTOR_AMOUNT, DOUBLE, EMPTY_HAND_DRAW, Effect, Game, HAND_LIMIT, PLAYS_PER_TURN,
    Phase, Player, TURN_DRAW, TargetChoice,
};
use crate::action::Action;
use crate::cards::*;

impl Game {
    pub(super) fn apply(&mut self, action: Action) {
        match action {
            Action::EndTurn => self.end_turn(),
            Action::Bank(kind) => {
                let card = self.me().take_from_hand(kind);
                self.me().bank.push(card);
                self.use_play();
            }
            Action::PlaceProperty { kind, color } => {
                let card = self.me().take_from_hand(kind);
                self.me().pile_mut(color).cards.push(card);
                self.use_play();
                self.check_win();
            }
            Action::MoveWild { kind, from, to } => {
                let p = self.me();
                let card = p.pile(from).find(kind).expect("wild not on table");
                p.pile_mut(from).remove(card);
                p.fix_buildings(from);
                p.pile_mut(to).cards.push(card);
                self.use_play();
                self.check_win();
            }
            Action::PlayAction(t) => self.play_action(t),
            Action::PlayRent { color, wild } => {
                let rent = if wild { RentType::Wild } else { RentType::for_color(color) };
                let card = self.me().take_from_hand(Kind::Rent(rent));
                self.discard.push(card);
                self.use_play();
                if wild {
                    self.phase = Phase::ChooseTarget(TargetChoice::RentPlayer(color));
                } else {
                    self.offer_double(color, None, 1);
                }
            }
            Action::TargetPlayer(_) | Action::TargetProperty { .. } | Action::TargetSet { .. } => {
                self.choose_target(action)
            }
            Action::PlayDouble | Action::DeclineDouble => {
                let Phase::OfferDouble { color, target, multiplier } = self.phase else { unreachable!() };
                if action == Action::PlayDouble {
                    let card = self.me().take_from_hand(DOUBLE);
                    self.discard.push(card);
                    self.use_play();
                    self.offer_double(color, target, multiplier * 2);
                } else {
                    self.resolve_rent(color, target, multiplier);
                }
            }
            Action::JustSayNo => self.respond(true),
            Action::Accept => self.respond(false),
            Action::PayBank(_) | Action::PayProperty { .. } | Action::PayBuilding { .. } => self.pay(action),
            Action::Discard(kind) => {
                let card = self.me().take_from_hand(kind);
                self.discard.push(card);
                if self.players[self.active].hand.len() <= HAND_LIMIT {
                    self.next_turn();
                }
            }
        }
        // With no plays left the only legal move would be EndTurn, so take it.
        // This also covers the moment a rent or steal finishes resolving.
        if self.phase == Phase::Main && self.plays_left == 0 {
            self.end_turn();
        }
    }

    /// The active player.
    fn me(&mut self) -> &mut Player {
        &mut self.players[self.active]
    }

    fn use_play(&mut self) {
        self.plays_left -= 1;
    }

    fn play_action(&mut self, t: ActionType) {
        let a = self.active;
        let card = self.me().take_from_hand(Kind::Action(t));
        self.use_play();
        if matches!(t, ActionType::House | ActionType::Hotel) {
            // Held by the phase until its set is chosen.
            let choice = if t == ActionType::House { TargetChoice::House { card } } else { TargetChoice::Hotel { card } };
            self.phase = Phase::ChooseTarget(choice);
            return;
        }
        self.discard.push(card);
        let choose = |choice| Phase::ChooseTarget(choice);
        match t {
            ActionType::PassGo => self.draw(a, 2),
            ActionType::Birthday => {
                let everyone = self.opponents(a).collect();
                self.start_resolution(Effect::Charge(BIRTHDAY_AMOUNT), everyone);
            }
            ActionType::DebtCollector => self.phase = choose(TargetChoice::Debt),
            ActionType::SlyDeal => self.phase = choose(TargetChoice::Sly),
            ActionType::ForcedDeal => self.phase = choose(TargetChoice::ForcedTake),
            ActionType::DealBreaker => self.phase = choose(TargetChoice::DealBreaker),
            ActionType::JustSayNo | ActionType::DoubleRent | ActionType::House | ActionType::Hotel => {
                unreachable!("{t:?} is not played this way")
            }
        }
    }

    fn choose_target(&mut self, action: Action) {
        let Phase::ChooseTarget(choice) = self.phase else { unreachable!() };
        let a = self.active;
        match (choice, action) {
            (TargetChoice::RentPlayer(color), Action::TargetPlayer(rel)) => {
                self.offer_double(color, Some(self.seat(a, rel)), 1)
            }
            (TargetChoice::Debt, Action::TargetPlayer(rel)) => {
                self.start_resolution(Effect::Charge(DEBT_COLLECTOR_AMOUNT), vec![self.seat(a, rel)])
            }
            (TargetChoice::Sly, Action::TargetProperty { rel, kind, color }) => {
                let owner = self.seat(a, rel);
                let card = self.card_on_table(owner, kind, color);
                self.start_resolution(Effect::Sly { card, color }, vec![owner])
            }
            (TargetChoice::ForcedTake, Action::TargetProperty { rel, kind, color }) => {
                let target = self.seat(a, rel);
                let card = self.card_on_table(target, kind, color);
                self.phase = Phase::ChooseTarget(TargetChoice::ForcedGive { target, card, color });
            }
            (TargetChoice::ForcedGive { target, card: theirs, color: their_color }, Action::TargetProperty { kind, color, .. }) => {
                let mine = self.card_on_table(a, kind, color);
                self.start_resolution(Effect::Forced { theirs, their_color, mine, my_color: color }, vec![target])
            }
            (TargetChoice::DealBreaker, Action::TargetSet { rel, color }) => {
                self.start_resolution(Effect::DealBreaker(color), vec![self.seat(a, rel)])
            }
            (TargetChoice::House { card }, Action::TargetSet { color, .. }) => {
                self.me().pile_mut(color).house = Some(card);
                self.phase = Phase::Main;
            }
            (TargetChoice::Hotel { card }, Action::TargetSet { color, .. }) => {
                self.me().pile_mut(color).hotel = Some(card);
                self.phase = Phase::Main;
            }
            _ => unreachable!("{action:?} doesn't answer {choice:?}"),
        }
    }

    pub(super) fn card_on_table(&self, owner: usize, kind: Kind, color: Color) -> CardId {
        self.players[owner].pile(color).find(kind).expect("no such property")
    }

    // ------------------------------------------------------------------
    // Turn flow
    // ------------------------------------------------------------------

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
            self.phase = Phase::GameOver; // a draw
        } else {
            self.start_turn((self.active + 1) % self.num_players);
        }
    }

    pub(super) fn start_turn(&mut self, p: usize) {
        self.active = p;
        self.plays_left = PLAYS_PER_TURN;
        self.phase = Phase::Main;
        let n = if self.players[p].hand.is_empty() { EMPTY_HAND_DRAW } else { TURN_DRAW };
        self.draw(p, n);
    }

    /// Draw `n` cards, reshuffling the discard pile into the deck if needed.
    pub(super) fn draw(&mut self, p: usize, n: usize) {
        for _ in 0..n {
            if self.deck.is_empty() {
                if self.discard.is_empty() {
                    return;
                }
                self.deck = std::mem::take(&mut self.discard);
                self.rng.shuffle(&mut self.deck);
            }
            let card = self.deck.pop().unwrap();
            self.players[p].hand.push(card);
        }
    }
}
