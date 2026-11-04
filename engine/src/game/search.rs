//! Support for search over hidden information (Information-Set MCTS).

use super::Game;
use crate::rng::Rng;

impl Game {
    /// A copy of the game where everything `seat` can't see (other players'
    /// hands and the deck order) is re-dealt at random, keeping hand sizes.
    ///
    /// Nothing about the phase reveals a hand: Just Say No and Double The Rent
    /// questions are asked whether or not the player holds the card.
    pub fn determinize(&self, seat: usize, seed: u64) -> Game {
        let mut g = self.clone();
        let mut rng = Rng::new(seed);
        let others: Vec<usize> = (0..g.num_players).filter(|&p| p != seat).collect();

        let mut unseen = std::mem::take(&mut g.deck);
        let sizes: Vec<usize> = others.iter().map(|&p| g.players[p].hand.len()).collect();
        for &p in &others {
            unseen.append(&mut g.players[p].hand);
        }
        rng.shuffle(&mut unseen);
        for (&p, &size) in others.iter().zip(&sizes) {
            g.players[p].hand = unseen.split_off(unseen.len() - size);
        }
        g.deck = unseen;
        g.rng = Rng::new(rng.next_u64());
        g
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bots::RandomBot;
    use crate::cards::kind_of;

    fn sorted_kinds(cards: impl Iterator<Item = u8>) -> Vec<usize> {
        let mut v: Vec<usize> = cards.map(|c| kind_of(c).index()).collect();
        v.sort();
        v
    }

    #[test]
    fn keeps_everything_the_player_can_see() {
        let mut g = Game::new(4, 3);
        let mut bot = RandomBot::new(1);
        for _ in 0..60 {
            let a = bot.choose(&g);
            g.step(a).unwrap();
        }
        let d = g.determinize(2, 99);
        d.check_invariants();
        assert_eq!(d.players[2].hand, g.players[2].hand);
        assert_eq!(d.discard, g.discard);
        for p in 0..4 {
            assert_eq!(d.players[p].hand.len(), g.players[p].hand.len());
            assert_eq!(d.players[p].bank, g.players[p].bank);
            assert_eq!(d.players[p].table, g.players[p].table);
        }
        assert_eq!(d.deck.len(), g.deck.len());
        assert_eq!((d.phase, d.active, d.plays_left, d.turn), (g.phase, g.active, g.plays_left, g.turn));
    }

    #[test]
    fn only_reshuffles_the_unseen_cards() {
        let g = Game::new(3, 8);
        let unseen = |g: &Game| sorted_kinds(g.deck.iter().chain(&g.players[1].hand).chain(&g.players[2].hand).copied());
        let d = g.determinize(0, 5);
        assert_eq!(unseen(&d), unseen(&g));
        assert_ne!(d.players[1].hand, g.players[1].hand, "should actually re-deal (seeded, so stable)");
    }

    #[test]
    fn same_seed_same_result() {
        let g = Game::new(3, 8);
        let (a, b) = (g.determinize(1, 42), g.determinize(1, 42));
        assert_eq!(a.deck, b.deck);
        assert_eq!(a.players[0].hand, b.players[0].hand);
    }
}
