//! Test helpers, consistency checks and a text view of the table.

use super::{Effect, Game, HAND_LIMIT, Phase, SETS_TO_WIN, TargetChoice};
use crate::cards::*;
use std::fmt::Write as _;

impl Game {
    // ------------------------------------------------------------------
    // Scenario setup for tests: move a card of some kind out of the deck.
    // ------------------------------------------------------------------

    fn take_from_deck(&mut self, kind: Kind) -> CardId {
        let i = self.deck.iter().position(|&c| kind_of(c) == kind).unwrap_or_else(|| panic!("no {kind} left in deck"));
        self.deck.remove(i)
    }

    #[doc(hidden)]
    pub fn give_hand(&mut self, p: usize, kind: Kind) -> CardId {
        let card = self.take_from_deck(kind);
        self.players[p].hand.push(card);
        card
    }

    #[doc(hidden)]
    pub fn give_bank(&mut self, p: usize, kind: Kind) -> CardId {
        let card = self.take_from_deck(kind);
        self.players[p].bank.push(card);
        card
    }

    #[doc(hidden)]
    pub fn give_table(&mut self, p: usize, kind: Kind, color: Color) -> CardId {
        assert!(kind.colors() & color.bit() != 0, "{kind} can't be {color}");
        let card = self.take_from_deck(kind);
        self.players[p].pile_mut(color).cards.push(card);
        card
    }

    #[doc(hidden)]
    pub fn give_building(&mut self, p: usize, color: Color, hotel: bool) -> CardId {
        let card = self.take_from_deck(Kind::Action(if hotel { ActionType::Hotel } else { ActionType::House }));
        let pile = self.players[p].pile_mut(color);
        *(if hotel { &mut pile.hotel } else { &mut pile.house }) = Some(card);
        card
    }

    // ------------------------------------------------------------------
    // Consistency checks
    // ------------------------------------------------------------------

    /// Panics unless the state is consistent: every card in exactly one place,
    /// cards where they may be, and the phase matching the rest of the state.
    /// Cheap enough to run after every step in fuzz tests.
    pub fn check_invariants(&self) {
        self.check_cards();
        self.check_phase();
        if !self.is_over() {
            assert!(!self.legal_actions().is_empty(), "no legal actions in {:?}", self.phase);
        }
    }

    fn check_cards(&self) {
        let mut seen = [0u8; DECK_SIZE];
        let mut see = |c: CardId| seen[c as usize] += 1;
        self.deck.iter().chain(&self.discard).for_each(|&c| see(c));
        if let Phase::ChooseTarget(TargetChoice::House { card } | TargetChoice::Hotel { card }) = self.phase {
            see(card); // held by the phase until its set is chosen
        }
        for p in &self.players {
            p.hand.iter().chain(&p.bank).for_each(|&c| see(c));
            assert!(p.bank.iter().all(|&c| !kind_of(c).is_property()), "property in bank");
            for color in COLORS {
                let pile = p.pile(color);
                for &c in &pile.cards {
                    see(c);
                    assert!(kind_of(c).colors() & color.bit() != 0, "card {c} misplaced in {color}");
                }
                if let Some(h) = pile.house {
                    see(h);
                    assert_eq!(kind_of(h), Kind::Action(ActionType::House));
                    assert!(p.is_complete(color) && color.allows_buildings(), "house on a bad set");
                }
                if let Some(h) = pile.hotel {
                    see(h);
                    assert_eq!(kind_of(h), Kind::Action(ActionType::Hotel));
                    assert!(pile.house.is_some(), "hotel without a house");
                }
            }
        }
        if let Some(bad) = seen.iter().position(|&n| n != 1) {
            panic!("card {bad} appears {} times", seen[bad]);
        }
    }

    fn check_phase(&self) {
        let someone_won = self.players.iter().any(|p| p.complete_sets() >= SETS_TO_WIN);
        assert_eq!(self.winner.is_some(), someone_won, "winner doesn't match the table");
        if let Some(w) = self.winner {
            assert!(self.is_over() && self.players[w].complete_sets() >= SETS_TO_WIN);
        }
        let resolving = matches!(self.phase, Phase::Respond { .. } | Phase::Pay { .. });
        assert_eq!(self.pending.is_some(), resolving, "pending resolution outside Respond/Pay");
        match self.phase {
            Phase::GameOver => {
                assert!(self.winner.is_some() || self.turn >= self.max_turns, "game over too early");
            }
            Phase::Main => assert!(self.plays_left > 0, "no plays left: the turn should have ended"),
            Phase::Discard => assert!(self.players[self.active].hand.len() > HAND_LIMIT),
            Phase::ChooseTarget(_) => {}
            Phase::OfferDouble { multiplier, .. } => {
                assert!(self.plays_left > 0, "Double offered with no plays left");
                assert!(matches!(multiplier, 1 | 2), "more doubles than plays allow");
            }
            Phase::Respond { jsn_count } => {
                let r = self.pending.as_ref().unwrap();
                assert!(r.idx < r.victims.len());
                assert!(jsn_count <= KIND_COUNTS[Kind::Action(ActionType::JustSayNo).index()]);
            }
            Phase::Pay { owed } => {
                let r = self.pending.as_ref().unwrap();
                assert!(matches!(r.effect, Effect::Charge(_)) && owed > 0);
                let assets = self.players[r.victim()].payable_value();
                assert!(assets > owed as u32, "payer should have paid everything automatically");
            }
        }
    }

    // ------------------------------------------------------------------
    // Text view
    // ------------------------------------------------------------------

    /// Human-readable view of the table from `seat`'s perspective.
    pub fn describe(&self, seat: usize) -> String {
        let names = |cards: &[CardId]| cards.iter().map(|&c| kind_of(c).to_string()).collect::<Vec<_>>().join(", ");
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
                if pile.is_empty() {
                    continue;
                }
                let house = if pile.house.is_some() { " +house" } else { "" };
                let hotel = if pile.hotel.is_some() { " +hotel" } else { "" };
                let _ = writeln!(s, "    {color:<9} {}/{}{house}{hotel}: {}", pile.len(), color.set_size(), names(&pile.cards));
            }
        }
        let _ = writeln!(s, "Your hand: {}", names(&self.players[seat].hand));
        if let Some(r) = &self.pending {
            let _ = writeln!(s, "Resolving {:?} from P{} against P{}", r.effect, r.actor, r.victim());
        }
        s
    }
}
