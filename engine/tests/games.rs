//! Whole games: invariants hold, games end, and play is reproducible.

use monopd_engine::bots::RandomBot;
use monopd_engine::{Action, Game, OBS_SIZE};

#[test]
fn random_games_keep_every_invariant_and_end() {
    let mut winners = 0;
    for players in 2..=5 {
        for seed in 0..150u64 {
            let mut g = Game::new(players, seed);
            let mut bot = RandomBot::new(seed * 31 + players as u64);
            while let Some(p) = g.current_player() {
                let a = bot.choose(&g);
                assert_eq!(Action::from_index(a.index()), Some(a));
                let obs = g.observe(p);
                assert_eq!(obs.len(), OBS_SIZE);
                assert!(obs.iter().all(|x| x.is_finite()));
                let mask = g.legal_mask();
                assert!(mask[a.index()]);
                assert_eq!(mask.iter().filter(|&&m| m).count(), g.legal_actions().len());
                g.step(a).unwrap();
                g.check_invariants();
            }
            winners += g.winner.is_some() as u32;
        }
    }
    assert!(winners > 0, "random play should sometimes produce a winner");
}

#[test]
fn the_same_seed_plays_the_same_game() {
    let play = |seed| {
        let mut g = Game::new(4, seed);
        let mut bot = RandomBot::new(99);
        let mut trace = Vec::new();
        while g.current_player().is_some() {
            let a = bot.choose(&g);
            trace.push(a.index());
            g.step(a).unwrap();
        }
        (trace, g.winner, g.turn)
    };
    assert_eq!(play(5), play(5));
    assert_ne!(play(5).0, play(6).0);
}
