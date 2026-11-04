//! Fixed-size observation vector for one seat.
//!
//! It holds only what that player could really see: their own hand, every
//! table and bank, the discard pile, and hand *sizes* of opponents. Players
//! are listed in seat order starting from the observer (slot 0 = me, slot 1 =
//! next player, ...) and padded to `MAX_PLAYERS`, so one network handles any
//! player count and any seat.

use crate::action::MAX_PLAYERS;
use crate::cards::*;
use crate::game::{Effect, Game, Phase, TargetChoice, PLAYS_PER_TURN};

const NUM_PHASES: usize = 14;

const GLOBAL_SIZE: usize = (MAX_PLAYERS - 1) // player count one-hot (2..=5)
    + 1                                       // deck size
    + NUM_KINDS                               // discard pile composition
    + 1                                       // turn progress
    + (PLAYS_PER_TURN as usize + 1)           // plays left one-hot
    + MAX_PLAYERS                             // active player (relative)
    + NUM_PHASES                              // decision type
    + MAX_PLAYERS                             // pending actor (relative)
    + MAX_PLAYERS                             // pending victim (relative)
    + 4                                       // pending effect type
    + 1                                       // amount owed / charged
    + 1                                       // rent multiplier
    + 1                                       // JSN chain length
    + NUM_COLORS                              // colour involved
    + NUM_PROPERTY_KINDS; // property kind involved

pub const PER_PLAYER_SIZE: usize = 1 // present
    + 1                              // hand size
    + 1                              // bank value
    + NUM_KINDS                      // bank composition
    + NUM_PROPERTY_KINDS * NUM_COLORS // table: count of each (kind, colour)
    + 4 * NUM_COLORS                 // per colour: fill ratio, complete, house, hotel
    + 1                              // complete sets
    + 1; // unattached rainbow wilds

pub const OBS_SIZE: usize = GLOBAL_SIZE + NUM_KINDS + MAX_PLAYERS * PER_PLAYER_SIZE;

struct Writer {
    v: Vec<f32>,
}

impl Writer {
    fn push(&mut self, x: f32) {
        self.v.push(x);
    }
    fn one_hot(&mut self, n: usize, i: Option<usize>) {
        let start = self.v.len();
        self.v.resize(start + n, 0.0);
        if let Some(i) = i {
            self.v[start + i] = 1.0;
        }
    }
    fn counts(&mut self, n: usize, items: impl Iterator<Item = usize>, scale: f32) {
        let start = self.v.len();
        self.v.resize(start + n, 0.0);
        for i in items {
            self.v[start + i] += scale;
        }
    }
}

fn phase_index(p: &Phase) -> usize {
    match p {
        Phase::Main => 0,
        Phase::Discard => 1,
        Phase::ChooseTarget(t) => match t {
            TargetChoice::RentPlayer(_) => 2,
            TargetChoice::Debt => 3,
            TargetChoice::Sly => 4,
            TargetChoice::ForcedTake => 5,
            TargetChoice::ForcedGive { .. } => 6,
            TargetChoice::DealBreaker => 7,
            TargetChoice::House { .. } => 8,
            TargetChoice::Hotel { .. } => 9,
        },
        Phase::OfferDouble { .. } => 10,
        Phase::Respond { .. } => 11,
        Phase::Pay { .. } => 12,
        Phase::GameOver => 13,
    }
}

impl Game {
    /// Observation for `seat`, length [`OBS_SIZE`].
    pub fn observe(&self, seat: usize) -> Vec<f32> {
        let mut w = Writer { v: Vec::with_capacity(OBS_SIZE) };
        let n = self.num_players;
        let rel = |p: usize| self.rel(seat, p) as usize;

        // --- global ---
        w.one_hot(MAX_PLAYERS - 1, Some(n - 2));
        w.push(self.deck.len() as f32 / DECK_SIZE as f32);
        w.counts(NUM_KINDS, self.discard.iter().map(|&c| kind_of(c).index()), 1.0);
        let start = w.v.len() - NUM_KINDS;
        for k in 0..NUM_KINDS {
            w.v[start + k] /= KIND_COUNTS[k] as f32;
        }
        w.push(self.turn as f32 / self.max_turns as f32);
        w.one_hot(PLAYS_PER_TURN as usize + 1, Some(self.plays_left as usize));
        w.one_hot(MAX_PLAYERS, Some(rel(self.active)));
        w.one_hot(NUM_PHASES, Some(phase_index(&self.phase)));

        // --- what is being resolved / chosen ---
        let (actor, victim) = match &self.pending {
            Some(r) => (Some(rel(r.actor)), Some(rel(r.victim()))),
            None => (None, None),
        };
        w.one_hot(MAX_PLAYERS, actor);
        w.one_hot(MAX_PLAYERS, victim);
        let mut effect = None;
        let mut amount = 0.0;
        let mut multiplier = 0.0;
        let mut color = None;
        let mut prop_kind = None;
        if let Some(r) = &self.pending {
            match r.effect {
                Effect::Charge(x) => {
                    effect = Some(0);
                    amount = x as f32;
                }
                Effect::Sly { card, color: c } => {
                    effect = Some(1);
                    color = Some(c.idx());
                    prop_kind = Some(kind_of(card).index());
                }
                Effect::Forced { theirs, their_color, .. } => {
                    effect = Some(2);
                    color = Some(their_color.idx());
                    prop_kind = Some(kind_of(theirs).index());
                }
                Effect::DealBreaker(c) => {
                    effect = Some(3);
                    color = Some(c.idx());
                }
            }
        }
        let mut jsn = 0.0;
        match self.phase {
            Phase::Pay { owed } => amount = owed as f32,
            Phase::Respond { jsn_count } => jsn = jsn_count as f32,
            Phase::OfferDouble { color: c, multiplier: m, .. } => {
                color = Some(c.idx());
                multiplier = m as f32;
                amount = self.players[self.active].rent(c) as f32 * m as f32;
            }
            Phase::ChooseTarget(TargetChoice::RentPlayer(c)) => color = Some(c.idx()),
            Phase::ChooseTarget(TargetChoice::ForcedGive { card, color: c, .. }) => {
                color = Some(c.idx());
                prop_kind = Some(kind_of(card).index());
            }
            _ => {}
        }
        w.one_hot(4, effect);
        w.push(amount / 10.0);
        w.push(multiplier / 4.0);
        w.push(jsn / 4.0);
        w.one_hot(NUM_COLORS, color);
        w.one_hot(NUM_PROPERTY_KINDS, prop_kind);

        // --- my hand ---
        w.counts(NUM_KINDS, self.players[seat].hand.iter().map(|&c| kind_of(c).index()), 1.0);

        // --- every player, starting with me ---
        for r in 0..MAX_PLAYERS {
            if r >= n {
                w.v.resize(w.v.len() + PER_PLAYER_SIZE, 0.0);
                continue;
            }
            let p = &self.players[self.seat(seat, r as u8)];
            w.push(1.0);
            w.push(p.hand.len() as f32 / 10.0);
            w.push(p.bank_value() as f32 / 20.0);
            w.counts(NUM_KINDS, p.bank.iter().map(|&c| kind_of(c).index()), 1.0);
            w.counts(
                NUM_PROPERTY_KINDS * NUM_COLORS,
                COLORS.iter().flat_map(|&col| {
                    p.pile(col).cards.iter().map(move |&c| kind_of(c).index() * NUM_COLORS + col.idx())
                }),
                1.0,
            );
            for col in COLORS {
                let pile = p.pile(col);
                w.push(pile.len().min(col.set_size()) as f32 / col.set_size() as f32);
                w.push(p.is_complete(col) as u8 as f32);
                w.push(pile.house.is_some() as u8 as f32);
                w.push(pile.hotel.is_some() as u8 as f32);
            }
            w.push(p.complete_sets() as f32 / 3.0);
            w.push(p.loose.len() as f32);
        }

        debug_assert_eq!(w.v.len(), OBS_SIZE);
        w.v
    }
}
