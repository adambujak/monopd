//! Resolving action cards against victims: Just Say No chains, rent and
//! payments, steals, and the win check.

use super::{Effect, Game, JSN, Phase, Resolution, SETS_TO_WIN};
use crate::action::Action;
use crate::cards::*;

impl Game {
    /// After rent is played, offer a Double The Rent while plays remain. The
    /// offer is made whether or not the player holds one (see `legal`).
    pub(super) fn offer_double(&mut self, color: Color, target: Option<usize>, multiplier: u8) {
        if self.plays_left > 0 {
            self.phase = Phase::OfferDouble { color, target, multiplier };
        } else {
            self.resolve_rent(color, target, multiplier);
        }
    }

    pub(super) fn resolve_rent(&mut self, color: Color, target: Option<usize>, multiplier: u8) {
        let a = self.active;
        let amount = (self.players[a].rent(color) as u32 * multiplier as u32).min(u8::MAX as u32) as u8;
        let victims = match target {
            Some(t) => vec![t],
            None => self.opponents(a).collect(),
        };
        self.start_resolution(Effect::Charge(amount), victims);
    }

    pub(super) fn start_resolution(&mut self, effect: Effect, victims: Vec<usize>) {
        self.pending = Some(Resolution { actor: self.active, effect, victims, idx: 0 });
        self.ask_victim();
    }

    /// Ask the current victim whether to Just Say No (everyone is asked, card
    /// or not), or finish once every victim has been dealt with.
    fn ask_victim(&mut self) {
        let r = self.pending.as_ref().expect("no pending resolution");
        if r.idx < r.victims.len() {
            self.phase = Phase::Respond { jsn_count: 0 };
        } else {
            self.pending = None;
            self.phase = Phase::Main;
        }
    }

    /// The current responder plays a Just Say No, or lets the chain end.
    pub(super) fn respond(&mut self, say_no: bool) {
        let Phase::Respond { jsn_count } = self.phase else { unreachable!() };
        if say_no {
            let responder = self.responder(jsn_count);
            let card = self.players[responder].take_from_hand(JSN);
            self.discard.push(card);
            // The other side always gets the chance to answer it.
            self.phase = Phase::Respond { jsn_count: jsn_count + 1 };
            return;
        }
        // An odd number of Just Say Nos cancels the action for this victim.
        let cancelled = jsn_count % 2 == 1;
        if !cancelled && (self.apply_effect() || self.is_over()) {
            return;
        }
        self.next_victim();
    }

    fn next_victim(&mut self) {
        self.pending.as_mut().expect("no pending resolution").idx += 1;
        self.ask_victim();
    }

    /// Apply the effect to the current victim. Returns true if we now wait
    /// for them to choose how to pay.
    fn apply_effect(&mut self) -> bool {
        let r = self.pending.as_ref().expect("no pending resolution");
        let (actor, victim, effect) = (r.actor, r.victim(), r.effect);
        match effect {
            Effect::Charge(owed) => return self.collect(victim, actor, owed),
            Effect::Sly { card, color } => self.move_property(victim, actor, card, color),
            Effect::Forced { theirs, their_color, mine, my_color } => {
                self.move_property(victim, actor, theirs, their_color);
                self.move_property(actor, victim, mine, my_color);
            }
            Effect::DealBreaker(color) => {
                let stolen = std::mem::take(self.players[victim].pile_mut(color));
                let extra = self.players[actor].pile_mut(color).absorb(stolen);
                self.players[actor].bank.extend(extra);
            }
        }
        self.check_win();
        false
    }

    /// Start (or continue) collecting `owed` from `payer`. Pays automatically
    /// when there's no choice to make; returns true if the payer must choose.
    fn collect(&mut self, payer: usize, receiver: usize, owed: u8) -> bool {
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

    /// The payer hands over one card. No change is given.
    pub(super) fn pay(&mut self, action: Action) {
        let Phase::Pay { owed } = self.phase else { unreachable!() };
        let r = self.pending.as_ref().expect("paying without a debt");
        let (payer, receiver) = (r.victim(), r.actor);
        let value = match action {
            Action::PayBank(kind) => {
                let card = self.players[payer].take_from_bank(kind);
                self.players[receiver].bank.push(card);
                kind.value()
            }
            Action::PayProperty { kind, color } => {
                let card = self.card_on_table(payer, kind, color);
                self.move_property(payer, receiver, card, color);
                kind.value()
            }
            Action::PayBuilding { color, hotel } => {
                let card = self.players[payer].take_building(color, hotel);
                self.players[receiver].bank.push(card);
                kind_of(card).value()
            }
            _ => unreachable!(),
        };
        if self.check_win() {
            return;
        }
        if !self.collect(payer, receiver, owed.saturating_sub(value)) && !self.is_over() {
            self.next_victim();
        }
    }

    /// Hand over everything of value. Rainbow wilds are worth $0 and stay.
    fn pay_everything(&mut self, payer: usize, receiver: usize) {
        let mut to_bank = std::mem::take(&mut self.players[payer].bank);
        for color in COLORS {
            let pile = self.players[payer].pile_mut(color);
            to_bank.extend(pile.take_buildings().into_iter().flatten());
            let (give, keep) = pile.cards.iter().partition(|&&c| kind_of(c).value() > 0);
            pile.cards = keep;
            self.players[receiver].pile_mut(color).cards.extend::<Vec<CardId>>(give);
        }
        self.players[receiver].bank.extend(to_bank);
        self.check_win();
    }

    /// Move one property card between tables, keeping its colour.
    fn move_property(&mut self, from: usize, to: usize, card: CardId, color: Color) {
        let p = &mut self.players[from];
        p.pile_mut(color).remove(card);
        p.fix_buildings(color);
        self.players[to].pile_mut(color).cards.push(card);
    }

    /// End the game if someone has 3 complete sets. A Forced Deal can complete
    /// a third set for two players at once; the player whose turn it is wins.
    pub(super) fn check_win(&mut self) -> bool {
        if self.winner.is_none() {
            let n = self.num_players;
            self.winner = (0..n).map(|r| (self.active + r) % n).find(|&p| self.players[p].complete_sets() >= SETS_TO_WIN);
            if self.winner.is_some() {
                self.phase = Phase::GameOver;
                self.pending = None;
            }
        }
        self.winner.is_some()
    }
}
