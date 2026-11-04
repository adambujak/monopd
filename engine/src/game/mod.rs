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
//!
//! Layout: `player` (piles and players), `legal` (what may be done), `apply`
//! (doing it, turn flow), `resolve` (rent, steals, payments, Just Say No),
//! `search` (determinize), `debug` (test helpers, invariants, text view).

mod apply;
mod debug;
mod legal;
mod player;
mod resolve;
mod search;

pub use player::{Pile, Player};

use crate::action::{Action, MAX_PLAYERS};
use crate::cards::*;
use crate::rng::Rng;

pub const MIN_PLAYERS: usize = 2;
pub const STARTING_HAND: usize = 5;
pub const EMPTY_HAND_DRAW: usize = 5;
pub const TURN_DRAW: usize = 2;
pub const HAND_LIMIT: usize = 7;
pub const PLAYS_PER_TURN: u8 = 3;
pub const SETS_TO_WIN: usize = 3;
pub const DEFAULT_MAX_TURNS: u32 = 500;
pub const DEBT_COLLECTOR_AMOUNT: u8 = 5;
pub const BIRTHDAY_AMOUNT: u8 = 2;

const JSN: Kind = Kind::Action(ActionType::JustSayNo);
const DOUBLE: Kind = Kind::Action(ActionType::DoubleRent);
const RAINBOW: Kind = Kind::Wild(WildType::Rainbow);

/// Which target the active player is picking.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TargetChoice {
    RentPlayer(Color),
    Debt,
    Sly,
    ForcedTake,
    /// Second half of a Forced Deal: `card` (in `color`) is what we take from `target`.
    ForcedGive { target: usize, card: CardId, color: Color },
    DealBreaker,
    /// The House/Hotel card is held here until its set is picked.
    House { card: CardId },
    Hotel { card: CardId },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Active player plays cards or ends the turn.
    Main,
    /// Active player is over the hand limit at the end of their turn.
    Discard,
    /// Active player picks a target for the card just played.
    ChooseTarget(TargetChoice),
    /// Active player may add a Double The Rent to the rent just played.
    OfferDouble { color: Color, target: Option<usize>, multiplier: u8 },
    /// Just Say No chain against the current victim. Even count: the victim
    /// answers; odd: the actor answers (to counter).
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
pub enum StepError {
    /// The action isn't legal in the current state.
    Illegal(Action),
    /// The index isn't in `0..NUM_ACTIONS`.
    OutOfRange(usize),
}

impl std::fmt::Display for StepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StepError::Illegal(a) => write!(f, "illegal action: {a}"),
            StepError::OutOfRange(i) => write!(f, "action index {i} out of range"),
        }
    }
}

impl std::error::Error for StepError {}

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
        assert!((MIN_PLAYERS..=MAX_PLAYERS).contains(&num_players), "Monopoly Deal supports 2-5 players");
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

    pub fn is_over(&self) -> bool {
        self.phase == Phase::GameOver
    }

    /// The player who must act next, or `None` when the game is over.
    pub fn current_player(&self) -> Option<usize> {
        match self.phase {
            Phase::GameOver => None,
            Phase::Respond { jsn_count } => Some(self.responder(jsn_count)),
            Phase::Pay { .. } => Some(self.pending.as_ref()?.victim()),
            _ => Some(self.active),
        }
    }

    /// Final reward per player: +1 winner, -1 others, 0 for everyone on a draw.
    pub fn returns(&self) -> Vec<f32> {
        (0..self.num_players)
            .map(|p| match self.winner {
                Some(w) if w == p => 1.0,
                Some(_) => -1.0,
                None => 0.0,
            })
            .collect()
    }

    /// Seat of the player `rel` places after `from`.
    pub fn seat(&self, from: usize, rel: u8) -> usize {
        (from + rel as usize) % self.num_players
    }

    /// Position of `to` relative to `from` (0 = same player).
    pub fn rel(&self, from: usize, to: usize) -> u8 {
        ((to + self.num_players - from) % self.num_players) as u8
    }

    /// Everyone but `of`, in seat order starting after them.
    fn opponents(&self, of: usize) -> impl Iterator<Item = usize> + use<> {
        let n = self.num_players;
        (1..n).map(move |r| (of + r) % n)
    }

    /// Who answers at this point of a Just Say No chain.
    fn responder(&self, jsn_count: u8) -> usize {
        let r = self.pending.as_ref().expect("no pending resolution");
        if jsn_count.is_multiple_of(2) { r.victim() } else { r.actor }
    }

    /// Apply `action` if it is legal.
    pub fn step(&mut self, action: Action) -> Result<(), StepError> {
        if !self.legal_actions().contains(&action) {
            return Err(StepError::Illegal(action));
        }
        self.apply(action);
        Ok(())
    }

    /// Apply the action with flat index `idx` if it is legal.
    pub fn step_index(&mut self, idx: usize) -> Result<(), StepError> {
        let action = Action::from_index(idx).ok_or(StepError::OutOfRange(idx))?;
        self.step(action)
    }

    /// Apply an action the caller *knows* is legal (taken from
    /// `legal_actions`). Skips the legality check; an illegal action may panic
    /// or corrupt the state.
    pub fn apply_unchecked(&mut self, action: Action) {
        self.apply(action);
    }
}
