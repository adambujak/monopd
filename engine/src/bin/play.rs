//! Play against random bots in the terminal. You are P0.
//!
//!     cargo run --release --bin play -- [players] [seed]

use monopd_engine::bots::RandomBot;
use monopd_engine::Game;
use std::io::{self, BufRead, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let players: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(2);
    let seed: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or_else(|| {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
    });

    let mut game = Game::new(players, seed);
    let mut bot = RandomBot::new(seed.wrapping_mul(7));
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    while let Some(p) = game.current_player() {
        if p != 0 {
            let a = bot.choose(&game);
            println!("  P{p}: {a}");
            game.step(a).unwrap();
            continue;
        }
        println!("\n{}", game.describe(0));
        let legal = game.legal_actions();
        for (i, a) in legal.iter().enumerate() {
            println!("  [{i}] {a}");
        }
        loop {
            print!("> ");
            io::stdout().flush().unwrap();
            let Some(Ok(line)) = lines.next() else { return };
            match line.trim().parse::<usize>() {
                Ok(i) if i < legal.len() => {
                    game.step(legal[i]).unwrap();
                    break;
                }
                _ => println!("enter a number 0-{}", legal.len() - 1),
            }
        }
    }
    println!("\n{}", game.describe(0));
    match game.winner {
        Some(0) => println!("You win!"),
        Some(w) => println!("P{w} wins."),
        None => println!("Draw (turn limit)."),
    }
}
