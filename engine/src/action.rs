//! The action space.
//!
//! Actions name *what* is done, never *which physical card*: cards of the
//! same [`Kind`] are interchangeable, so "charge rent on Red" is the same
//! action whichever Red/Yellow rent card you hold. Players are named by seat
//! *relative to the decider* (`rel` 1 = next player, 0 = yourself), so the
//! same move looks the same from every seat.
//!
//! Complex moves are split into a sequence of small decisions. Sly Deal, for
//! example, is `PlayAction(SlyDeal)` followed by `TargetProperty {..}`. The
//! engine's phase decides which kinds of action are legal at each point.
//!
//! Every action has a fixed index in `0..NUM_ACTIONS`, so an RL policy can
//! output one logit per index and mask out the illegal ones.

use crate::cards::*;
use std::fmt;

pub const MAX_PLAYERS: usize = 5;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    // --- Main phase (active player) -----------------------------------
    /// End the turn (also legal before using all 3 plays).
    EndTurn,
    /// Put a money, action or rent card from hand into the bank.
    Bank(Kind),
    /// Place a property or wild from hand as `color`.
    PlaceProperty { kind: Kind, color: Color },
    /// Re-assign a wild already on your table. Uses one of your 3 plays.
    MoveWild { kind: Kind, from: Color, to: Color },
    /// Play an action card. Targets, if any, are picked next.
    PlayAction(ActionType),
    /// Charge rent on `color`. `wild` = use the wild rent card (one target)
    /// rather than the two-colour card (charges everyone).
    PlayRent { color: Color, wild: bool },

    // --- Target selection (active player) -------------------------------
    TargetPlayer(u8),
    /// A property on the table of player `rel` (0 = your own, for Forced Deal).
    TargetProperty { rel: u8, kind: Kind, color: Color },
    /// A whole colour on the table of player `rel` (Deal Breaker, House, Hotel).
    TargetSet { rel: u8, color: Color },

    // --- Yes/no decisions ------------------------------------------------
    PlayDouble,
    DeclineDouble,
    JustSayNo,
    Accept,

    // --- Paying a debt (the payer) --------------------------------------
    PayBank(Kind),
    PayProperty { kind: Kind, color: Color },
    PayBuilding { color: Color, hotel: bool },

    // --- End of turn hand limit -----------------------------------------
    Discard(Kind),
}

const NC: usize = NUM_COLORS;
const PK: usize = NUM_PROPERTY_KINDS;
const NUM_WILD_KINDS: usize = 8;

const O_END: usize = 0;
const O_BANK: usize = O_END + 1;
const O_PLACE: usize = O_BANK + NUM_KINDS;
const O_MOVE: usize = O_PLACE + PK * NC;
const O_PLAY: usize = O_MOVE + NUM_WILD_KINDS * NC * NC;
const O_RENT: usize = O_PLAY + NUM_ACTION_TYPES;
const O_TPLAYER: usize = O_RENT + 2 * NC;
const O_TPROP: usize = O_TPLAYER + MAX_PLAYERS;
const O_TSET: usize = O_TPROP + MAX_PLAYERS * PK * NC;
const O_MISC: usize = O_TSET + MAX_PLAYERS * NC;
const O_PAYBANK: usize = O_MISC + 4;
const O_PAYPROP: usize = O_PAYBANK + NUM_KINDS;
const O_PAYBLD: usize = O_PAYPROP + PK * NC;
const O_DISCARD: usize = O_PAYBLD + 2 * NC;

/// Size of the flat action space.
pub const NUM_ACTIONS: usize = O_DISCARD + NUM_KINDS;

#[inline]
fn slot(kind: Kind, color: Color) -> usize {
    debug_assert!(kind.is_property());
    kind.index() * NC + color.idx()
}

#[inline]
fn unslot(i: usize) -> (Kind, Color) {
    (Kind::from_index(i / NC), Color::from_idx(i % NC))
}

impl Action {
    /// Flat index in `0..NUM_ACTIONS`.
    pub fn index(self) -> usize {
        match self {
            Action::EndTurn => O_END,
            Action::Bank(k) => O_BANK + k.index(),
            Action::PlaceProperty { kind, color } => O_PLACE + slot(kind, color),
            Action::MoveWild { kind, from, to } => {
                debug_assert!(kind.is_wild());
                O_MOVE + ((kind.index() - 10) * NC + from.idx()) * NC + to.idx()
            }
            Action::PlayAction(t) => O_PLAY + t as usize,
            Action::PlayRent { color, wild } => O_RENT + (wild as usize) * NC + color.idx(),
            Action::TargetPlayer(r) => O_TPLAYER + r as usize,
            Action::TargetProperty { rel, kind, color } => {
                O_TPROP + rel as usize * PK * NC + slot(kind, color)
            }
            Action::TargetSet { rel, color } => O_TSET + rel as usize * NC + color.idx(),
            Action::PlayDouble => O_MISC,
            Action::DeclineDouble => O_MISC + 1,
            Action::JustSayNo => O_MISC + 2,
            Action::Accept => O_MISC + 3,
            Action::PayBank(k) => O_PAYBANK + k.index(),
            Action::PayProperty { kind, color } => O_PAYPROP + slot(kind, color),
            Action::PayBuilding { color, hotel } => O_PAYBLD + (hotel as usize) * NC + color.idx(),
            Action::Discard(k) => O_DISCARD + k.index(),
        }
    }

    /// Inverse of [`Action::index`]. Every index decodes to *some* action;
    /// whether it is legal depends on the game state.
    pub fn from_index(i: usize) -> Option<Action> {
        Some(match i {
            O_END => Action::EndTurn,
            _ if i < O_PLACE => Action::Bank(Kind::from_index(i - O_BANK)),
            _ if i < O_MOVE => {
                let (kind, color) = unslot(i - O_PLACE);
                Action::PlaceProperty { kind, color }
            }
            _ if i < O_PLAY => {
                let j = i - O_MOVE;
                Action::MoveWild {
                    kind: Kind::from_index(10 + j / (NC * NC)),
                    from: Color::from_idx((j / NC) % NC),
                    to: Color::from_idx(j % NC),
                }
            }
            _ if i < O_RENT => Action::PlayAction(ACTION_TYPES[i - O_PLAY]),
            _ if i < O_TPLAYER => {
                let j = i - O_RENT;
                Action::PlayRent { color: Color::from_idx(j % NC), wild: j >= NC }
            }
            _ if i < O_TPROP => Action::TargetPlayer((i - O_TPLAYER) as u8),
            _ if i < O_TSET => {
                let j = i - O_TPROP;
                let (kind, color) = unslot(j % (PK * NC));
                Action::TargetProperty { rel: (j / (PK * NC)) as u8, kind, color }
            }
            _ if i < O_MISC => {
                let j = i - O_TSET;
                Action::TargetSet { rel: (j / NC) as u8, color: Color::from_idx(j % NC) }
            }
            _ if i < O_PAYBANK => match i - O_MISC {
                0 => Action::PlayDouble,
                1 => Action::DeclineDouble,
                2 => Action::JustSayNo,
                _ => Action::Accept,
            },
            _ if i < O_PAYPROP => Action::PayBank(Kind::from_index(i - O_PAYBANK)),
            _ if i < O_PAYBLD => {
                let (kind, color) = unslot(i - O_PAYPROP);
                Action::PayProperty { kind, color }
            }
            _ if i < O_DISCARD => {
                let j = i - O_PAYBLD;
                Action::PayBuilding { color: Color::from_idx(j % NC), hotel: j >= NC }
            }
            _ if i < NUM_ACTIONS => Action::Discard(Kind::from_index(i - O_DISCARD)),
            _ => return None,
        })
    }
}

fn rel_name(rel: u8) -> String {
    if rel == 0 { "self".to_string() } else { format!("player+{rel}") }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Action::EndTurn => write!(f, "End turn"),
            Action::Bank(k) => write!(f, "Bank {k}"),
            Action::PlaceProperty { kind, color } => write!(f, "Place {kind} as {color}"),
            Action::MoveWild { kind, from, to } => write!(f, "Move {kind} from {from} to {to}"),
            Action::PlayAction(t) => write!(f, "Play {}", t.name()),
            Action::PlayRent { color, wild } => {
                write!(f, "Charge {color} rent{}", if wild { " (wild, one player)" } else { " (everyone)" })
            }
            Action::TargetPlayer(r) => write!(f, "Target {}", rel_name(r)),
            Action::TargetProperty { rel, kind, color } => {
                write!(f, "Target {kind} in {color} of {}", rel_name(rel))
            }
            Action::TargetSet { rel, color } => write!(f, "Target {color} set of {}", rel_name(rel)),
            Action::PlayDouble => write!(f, "Play Double The Rent"),
            Action::DeclineDouble => write!(f, "Don't double"),
            Action::JustSayNo => write!(f, "Play Just Say No"),
            Action::Accept => write!(f, "Accept"),
            Action::PayBank(k) => write!(f, "Pay with {k} from bank"),
            Action::PayProperty { kind, color } => write!(f, "Pay with {kind} from {color}"),
            Action::PayBuilding { color, hotel } => {
                write!(f, "Pay with {} on {color}", if hotel { "hotel" } else { "house" })
            }
            Action::Discard(k) => write!(f, "Discard {k}"),
        }
    }
}
