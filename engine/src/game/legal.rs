//! Which actions are legal right now.

use super::player::distinct;
use super::{DOUBLE, Game, JSN, Phase, TargetChoice};
use crate::action::{Action, NUM_ACTIONS};
use crate::cards::*;

impl Game {
    pub fn legal_actions(&self) -> Vec<Action> {
        let mut out = Vec::with_capacity(32);
        self.legal_actions_into(&mut out);
        out
    }

    /// Legal actions as a mask over `0..NUM_ACTIONS`.
    pub fn legal_mask(&self) -> Vec<bool> {
        let mut mask = vec![false; NUM_ACTIONS];
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
            Phase::Discard => out.extend(distinct(&self.players[self.active].hand).map(Action::Discard)),
            Phase::ChooseTarget(choice) => self.target_actions(choice, out),
            // These two questions are asked whether or not the player holds the
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
        out.push(Action::EndTurn);
        if self.plays_left == 0 {
            return;
        }
        let a = self.active;
        let p = &self.players[a];
        for kind in distinct(&p.hand) {
            match kind {
                Kind::Money(_) => out.push(Action::Bank(kind)),
                Kind::Property(_) | Kind::Wild(_) => {
                    out.extend(iter_colors(kind.colors()).map(|color| Action::PlaceProperty { kind, color }));
                }
                Kind::Action(t) => {
                    out.push(Action::Bank(kind));
                    if self.can_play(a, t) {
                        out.push(Action::PlayAction(t));
                    }
                }
                Kind::Rent(r) => {
                    out.push(Action::Bank(kind));
                    let wild = r == RentType::Wild;
                    let colors = iter_colors(r.colors()).filter(|&c| p.has_real(c));
                    out.extend(colors.map(|color| Action::PlayRent { color, wild }));
                }
            }
        }
        for from in COLORS {
            for kind in distinct(&p.pile(from).cards).filter(|k| k.is_wild()) {
                let targets = iter_colors(kind.colors()).filter(|&to| to != from);
                out.extend(targets.map(|to| Action::MoveWild { kind, from, to }));
            }
        }
    }

    /// Can action card `t` be played (not banked) by player `a` right now?
    fn can_play(&self, a: usize, t: ActionType) -> bool {
        let p = &self.players[a];
        let any_opponent = |f: fn(&super::Player) -> bool| self.opponents(a).any(|o| f(&self.players[o]));
        match t {
            ActionType::JustSayNo | ActionType::DoubleRent => false, // only as responses
            ActionType::PassGo | ActionType::Birthday | ActionType::DebtCollector => true,
            ActionType::DealBreaker => any_opponent(super::Player::has_complete_set),
            ActionType::SlyDeal => any_opponent(super::Player::has_stealable),
            ActionType::ForcedDeal => p.has_stealable() && any_opponent(super::Player::has_stealable),
            ActionType::House | ActionType::Hotel => {
                COLORS.iter().any(|&c| p.can_build(c, t == ActionType::Hotel))
            }
        }
    }

    fn target_actions(&self, choice: TargetChoice, out: &mut Vec<Action>) {
        let a = self.active;
        let opponents = 1..self.num_players as u8;
        match choice {
            TargetChoice::RentPlayer(_) | TargetChoice::Debt => out.extend(opponents.map(Action::TargetPlayer)),
            TargetChoice::Sly | TargetChoice::ForcedTake => {
                for rel in opponents {
                    self.push_stealable(rel, out);
                }
            }
            TargetChoice::ForcedGive { .. } => self.push_stealable(0, out),
            TargetChoice::DealBreaker => {
                for rel in opponents {
                    let owner = &self.players[self.seat(a, rel)];
                    let sets = COLORS.into_iter().filter(|&c| owner.is_complete(c));
                    out.extend(sets.map(|color| Action::TargetSet { rel, color }));
                }
            }
            TargetChoice::House { .. } | TargetChoice::Hotel { .. } => {
                let hotel = matches!(choice, TargetChoice::Hotel { .. });
                let sets = COLORS.into_iter().filter(|&c| self.players[a].can_build(c, hotel));
                out.extend(sets.map(|color| Action::TargetSet { rel: 0, color }));
            }
        }
    }

    /// Every property Sly/Forced Deal could take from the player at `rel`.
    fn push_stealable(&self, rel: u8, out: &mut Vec<Action>) {
        let owner = &self.players[self.seat(self.active, rel)];
        for color in COLORS {
            let pile = owner.pile(color);
            if pile.is_stealable(color) {
                out.extend(distinct(&pile.cards).map(|kind| Action::TargetProperty { rel, kind, color }));
            }
        }
    }

    fn pay_actions(&self, out: &mut Vec<Action>) {
        let p = &self.players[self.pending.as_ref().expect("paying without a debt").victim()];
        out.extend(distinct(&p.bank).map(Action::PayBank));
        for color in COLORS {
            let pile = p.pile(color);
            let payable = distinct(&pile.cards).filter(|k| k.value() > 0); // rainbow wilds are worth $0
            out.extend(payable.map(|kind| Action::PayProperty { kind, color }));
            // A hotel sits on the house, so it has to go first.
            if pile.hotel.is_some() {
                out.push(Action::PayBuilding { color, hotel: true });
            } else if pile.house.is_some() {
                out.push(Action::PayBuilding { color, hotel: false });
            }
        }
    }
}
