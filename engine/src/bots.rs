//! Baseline bots.

use crate::action::Action;
use crate::game::Game;
use crate::rng::Rng;

/// Picks uniformly among legal actions.
pub struct RandomBot {
    rng: Rng,
    buf: Vec<Action>,
}

impl RandomBot {
    pub fn new(seed: u64) -> Self {
        RandomBot { rng: Rng::new(seed), buf: Vec::with_capacity(64) }
    }

    pub fn choose(&mut self, game: &Game) -> Action {
        game.legal_actions_into(&mut self.buf);
        self.buf[self.rng.below(self.buf.len())]
    }
}
