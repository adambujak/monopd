//! WebAssembly bindings for the browser viewer.
//!
//! Dependency-free C ABI: functions take numbers, and anything structured is
//! returned as JSON in a shared buffer (read it with `out_ptr` / `out_len`).
//!
//! - `json`: tiny JSON writer
//! - `text`: wording for actions, phases, colours and card names
//! - `view`: the catalog and per-seat game state the page renders

mod json;
mod text;
mod view;

use monopd_engine::bots::RandomBot;
use monopd_engine::game::{Game, Phase};
use monopd_engine::Action;
use std::cell::RefCell;

/// One game in progress, with its bot and log. Log lines are
/// `(player, text)`; player -1 is a game event.
struct App {
    game: Game,
    bot: RandomBot,
    log: Vec<(i32, String)>,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    static OUT: RefCell<String> = const { RefCell::new(String::new()) };
}

#[no_mangle]
pub extern "C" fn out_ptr() -> *const u8 {
    OUT.with(|o| o.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn out_len() -> u32 {
    OUT.with(|o| o.borrow().len() as u32)
}

/// Static card and colour data, as JSON.
#[no_mangle]
pub extern "C" fn catalog() {
    emit(view::catalog());
}

#[no_mangle]
pub extern "C" fn new_game(players: u32, seed: u32) {
    let app = App {
        game: Game::new(players as usize, seed as u64),
        bot: RandomBot::new(seed as u64 ^ 0x5EED),
        log: vec![(-1, format!("New {players}-player game (seed {seed}). P0 starts."))],
    };
    APP.with(|a| *a.borrow_mut() = Some(app));
}

/// Apply the action with flat index `idx` for the current player. Returns 1
/// if it was legal and applied.
#[no_mangle]
pub extern "C" fn step(idx: u32) -> i32 {
    with_app(|app| match Action::from_index(idx as usize) {
        Some(a) if app.game.legal_actions().contains(&a) => {
            app.apply(a);
            1
        }
        _ => 0,
    })
}

/// Let the random bot act for the current player. Returns 1 if it acted.
#[no_mangle]
pub extern "C" fn bot_step() -> i32 {
    with_app(|app| {
        if app.game.is_over() {
            return 0;
        }
        let a = app.bot.choose(&app.game);
        app.apply(a);
        1
    })
}

/// The game state as JSON. `seat` is whose hand is shown and whose legal
/// actions are listed; -1 shows every hand and the current player's actions.
#[no_mangle]
pub extern "C" fn state(seat: i32) {
    let json = with_app(|app| view::state(app, seat));
    emit(json);
}

fn emit(json: json::Raw) {
    OUT.with(|o| *o.borrow_mut() = json.0);
}

fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> R {
    APP.with(|a| f(a.borrow_mut().as_mut().expect("call new_game first")))
}

impl App {
    /// Apply a legal action and log what happened.
    fn apply(&mut self, a: Action) {
        let g = &self.game;
        let who = g.current_player().expect("game is over");
        let line = text::describe(g, who, a);
        let (turn, active) = (g.turn, g.active);
        let worth_before: Vec<u32> = g.players.iter().map(|p| p.payable_value()).collect();

        self.game.apply_unchecked(a);

        // Everyone targeted is asked about Just Say No (and every renter about
        // Double The Rent), so "no" answers are routine: don't log them.
        if !matches!(a, Action::Accept | Action::DeclineDouble) {
            self.log.push((who as i32, line));
        }
        let ended_by_player = matches!(a, Action::EndTurn | Action::Discard(_));
        if !ended_by_player && self.game.turn != turn {
            self.log.push((active as i32, "is out of plays: turn ends".to_string()));
        } else if !ended_by_player && self.game.phase == Phase::Discard {
            self.log.push((active as i32, "is out of plays and must discard down to 7".to_string()));
        }
        self.log_value_moved(&worth_before);
        if self.game.is_over() {
            let msg = match self.game.winner {
                Some(w) => format!("P{w} wins with 3 complete sets!"),
                None => "Draw: turn limit reached.".to_string(),
            };
            self.log.push((-1, msg));
        }
    }

    /// Summarise value that changed hands (payments and steals), e.g.
    /// "P0 +$3M, P2 −$3M".
    fn log_value_moved(&mut self, before: &[u32]) {
        let after: Vec<u32> = self.game.players.iter().map(|p| p.payable_value()).collect();
        if !before.iter().zip(&after).any(|(b, a)| a < b) {
            return;
        }
        let parts: Vec<String> = before
            .iter()
            .zip(&after)
            .enumerate()
            .filter(|(_, (b, a))| a != b)
            .map(|(p, (&b, &a))| {
                let sign = if a > b { "+" } else { "−" };
                format!("P{p} {sign}${}M", a.abs_diff(b))
            })
            .collect();
        self.log.push((-1, parts.join(", ")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn play_out(seed: u32) {
        new_game(3, seed);
        while bot_step() == 1 {}
    }

    /// The log must not reveal that a player held a Just Say No or a Double
    /// they chose not to use.
    #[test]
    fn log_never_reveals_declined_responses() {
        let mut saw_jsn = false;
        for seed in 0..300 {
            play_out(seed);
            APP.with(|a| {
                for (_, t) in &a.borrow().as_ref().unwrap().log {
                    assert!(!t.starts_with("accept"), "log reveals a held JSN: {t}");
                    assert!(!t.starts_with("don't double"), "log reveals a held Double: {t}");
                    saw_jsn |= t.contains("Just Say No");
                }
            });
        }
        assert!(saw_jsn, "test should see some Just Say No plays");
    }

    /// Every state the page could ask for is valid-looking JSON with the
    /// fields it reads.
    #[test]
    fn state_json_has_the_fields_the_page_reads() {
        catalog();
        let cat = OUT.with(|o| o.borrow().clone());
        assert!(cat.starts_with("{\"kinds\":[") && cat.contains("\"slug\":\"action-just-say-no\""));
        new_game(4, 7);
        for _ in 0..200 {
            for seat in [-1, 0, 2] {
                state(seat);
                let s = OUT.with(|o| o.borrow().clone());
                for field in ["\"phase\":", "\"players\":[", "\"legal\":[", "\"log\":[", "\"current\":"] {
                    assert!(s.contains(field), "missing {field}");
                }
                assert_eq!(s.matches('{').count(), s.matches('}').count());
            }
            if bot_step() == 0 {
                break;
            }
        }
    }
}
