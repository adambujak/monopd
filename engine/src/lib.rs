//! Monopoly Deal game engine, built for self-play reinforcement learning.
//!
//! ```
//! use monopd_engine::{Game, bots::RandomBot};
//!
//! let mut game = Game::new(3, 42);
//! let mut bot = RandomBot::new(7);
//! while let Some(_player) = game.current_player() {
//!     let action = bot.choose(&game);
//!     game.step(action).unwrap();
//! }
//! println!("winner: {:?}", game.winner);
//! ```

pub mod action;
pub mod bots;
pub mod cards;
pub mod game;
pub mod obs;
pub mod rng;

pub use action::{Action, MAX_PLAYERS, NUM_ACTIONS};
pub use cards::{ActionType, Color, Kind, RentType, WildType};
pub use game::{Game, Phase, StepError};
pub use obs::OBS_SIZE;
