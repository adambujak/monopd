//! Run many random-vs-random games and report throughput and outcome stats.
//!
//!     cargo run --release --bin simulate -- [games] [players] [seed]

use monopd_engine::bots::RandomBot;
use monopd_engine::Game;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let games: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(10_000);
    let players: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(3);
    let seed: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);

    let mut wins = vec![0u64; players];
    let (mut draws, mut total_steps, mut total_turns) = (0u64, 0u64, 0u64);
    let mut bot = RandomBot::new(seed ^ 0xABCD);
    let start = Instant::now();
    for g in 0..games {
        let mut game = Game::new(players, seed.wrapping_add(g));
        while game.current_player().is_some() {
            let a = bot.choose(&game);
            game.apply_unchecked(a);
            total_steps += 1;
        }
        total_turns += game.turn as u64;
        match game.winner {
            Some(w) => wins[w] += 1,
            None => draws += 1,
        }
    }
    let secs = start.elapsed().as_secs_f64();
    println!("{games} games, {players} players, random bots");
    println!(
        "{:.0} games/s, {:.2}M decisions/s ({:.2}s total)",
        games as f64 / secs,
        total_steps as f64 / secs / 1e6,
        secs
    );
    println!(
        "avg {:.1} turns, {:.1} decisions per game",
        total_turns as f64 / games as f64,
        total_steps as f64 / games as f64
    );
    for (p, w) in wins.iter().enumerate() {
        println!("  P{p} won {:.1}%", 100.0 * *w as f64 / games as f64);
    }
    println!("  draws (turn limit) {:.1}%", 100.0 * draws as f64 / games as f64);
}
