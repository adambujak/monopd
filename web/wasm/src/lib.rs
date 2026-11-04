//! WebAssembly bindings for the browser UI.
//!
//! Dependency-free C ABI: functions take numbers, and anything structured is
//! returned as JSON written into a shared buffer (`out_ptr` / `out_len`).

use monopd_engine::bots::RandomBot;
use monopd_engine::cards::*;
use monopd_engine::game::{Effect, Game, Phase, TargetChoice};
use monopd_engine::Action;
use std::cell::RefCell;
use std::fmt::Write;

struct App {
    game: Game,
    bot: RandomBot,
    log: Vec<(i32, String)>,
}

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
    static OUT: RefCell<String> = const { RefCell::new(String::new()) };
}

fn emit(s: String) {
    OUT.with(|o| *o.borrow_mut() = s);
}

#[no_mangle]
pub extern "C" fn out_ptr() -> *const u8 {
    OUT.with(|o| o.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn out_len() -> u32 {
    OUT.with(|o| o.borrow().len() as u32)
}

#[no_mangle]
pub extern "C" fn new_game(players: u32, seed: u32) {
    let game = Game::new(players as usize, seed as u64);
    let mut app = App { game, bot: RandomBot::new(seed as u64 ^ 0x5EED), log: Vec::new() };
    app.log.push((-1, format!("New {players}-player game (seed {seed}). P0 starts.")));
    APP.with(|a| *a.borrow_mut() = Some(app));
}

/// Apply the action with flat index `idx` for the current player. Returns 1 on success.
#[no_mangle]
pub extern "C" fn step(idx: u32) -> i32 {
    with_app(|app| match Action::from_index(idx as usize) {
        Some(a) if app.game.legal_actions().contains(&a) => {
            apply_logged(app, a);
            1
        }
        _ => 0,
    })
}

/// Let the random bot act for the current player. Returns 1 if it acted.
#[no_mangle]
pub extern "C" fn bot_step() -> i32 {
    with_app(|app| {
        if app.game.current_player().is_none() {
            return 0;
        }
        let a = app.bot.choose(&app.game);
        apply_logged(app, a);
        1
    })
}

/// Write the game state as JSON. `seat` = whose hand is visible and whose
/// legal actions are listed; -1 shows every hand and the current player's actions.
#[no_mangle]
pub extern "C" fn state(seat: i32) {
    let s = with_app(|app| state_json(app, seat));
    emit(s);
}

/// Static card/colour data as JSON.
#[no_mangle]
pub extern "C" fn catalog() {
    let mut s = String::from("{\"kinds\":[");
    for (i, k) in ALL_KINDS.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let ty = match k {
            Kind::Property(_) => "property",
            Kind::Wild(_) => "wild",
            Kind::Action(_) => "action",
            Kind::Rent(_) => "rent",
            Kind::Money(_) => "money",
        };
        let colors = match k {
            Kind::Rent(r) => r.colors(),
            _ => k.colors(),
        };
        let short = match k {
            Kind::Property(c) => c.name().to_string(),
            Kind::Wild(WildType::Rainbow) => "Wild".to_string(),
            Kind::Wild(_) => "Wild".to_string(),
            Kind::Action(a) => a.name().to_string(),
            Kind::Rent(RentType::Wild) => "Rent (any)".to_string(),
            Kind::Rent(_) => "Rent".to_string(),
            Kind::Money(m) => format!("${}M", MONEY_VALUES[*m as usize]),
        };
        let _ = write!(
            s,
            "{{\"name\":{},\"short\":{},\"type\":\"{ty}\",\"value\":{},\"colors\":{}}}",
            jstr(&k.to_string()),
            jstr(&short),
            k.value(),
            color_list(colors)
        );
    }
    s.push_str("],\"colors\":[");
    for (i, c) in COLORS.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let rent: Vec<String> = (1..=c.set_size()).map(|n| c.rent(n).to_string()).collect();
        let _ = write!(
            s,
            "{{\"name\":\"{}\",\"size\":{},\"rent\":[{}],\"buildings\":{}}}",
            c.name(),
            c.set_size(),
            rent.join(","),
            c.allows_buildings()
        );
    }
    s.push_str("]}");
    emit(s);
}

// ---------------------------------------------------------------------------

fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> R {
    APP.with(|a| f(a.borrow_mut().as_mut().expect("call new_game first")))
}

fn apply_logged(app: &mut App, a: Action) {
    let g = &app.game;
    let who = g.current_player().unwrap();
    let text = describe_action(g, who, a);
    let before: Vec<u32> = g.players.iter().map(|p| p.payable_value()).collect();
    let (turn_before, active_before) = (g.turn, g.active);
    app.game.apply_unchecked(a);
    // Declining a Just Say No or a Double is only asked of players who hold
    // one, so logging it would reveal their hand. Plays are logged.
    if !matches!(a, Action::Accept | Action::DeclineDouble) {
        app.log.push((who as i32, text));
    }
    if app.game.turn != turn_before && !matches!(a, Action::EndTurn | Action::Discard(_)) {
        app.log.push((active_before as i32, "is out of plays: turn ends".to_string()));
    } else if app.game.phase == Phase::Discard && !matches!(a, Action::EndTurn | Action::Discard(_)) {
        app.log.push((active_before as i32, "is out of plays and must discard down to 7".to_string()));
    }

    // Summarise value that changed hands (payments and steals).
    let after: Vec<u32> = app.game.players.iter().map(|p| p.payable_value()).collect();
    if before.iter().zip(&after).any(|(b, a)| a < b) {
        let parts: Vec<String> = before
            .iter()
            .zip(&after)
            .enumerate()
            .filter(|(_, (b, a))| a != b)
            .map(|(p, (b, a))| {
                let d = *a as i64 - *b as i64;
                format!("P{p} {}${}M", if d > 0 { "+" } else { "−" }, d.abs())
            })
            .collect();
        app.log.push((-1, parts.join(", ")));
    }
    if app.game.is_over() {
        let msg = match app.game.winner {
            Some(w) => format!("P{w} wins with 3 complete sets!"),
            None => "Draw: turn limit reached.".to_string(),
        };
        app.log.push((-1, msg));
    }
}

/// Like `Action`'s Display, but with absolute seats ("P2" not "player+1").
fn describe_action(g: &Game, who: usize, a: Action) -> String {
    let seat = |rel: u8| g.seat(who, rel);
    match a {
        Action::TargetPlayer(r) => format!("targets P{}", seat(r)),
        Action::TargetProperty { rel, kind, color } => {
            let owner = seat(rel);
            if owner == who {
                format!("gives their {kind} ({color})")
            } else {
                format!("targets P{owner}'s {kind} ({color})")
            }
        }
        Action::TargetSet { rel, color } => {
            let owner = seat(rel);
            if owner == who { format!("builds on {color}") } else { format!("targets P{owner}'s {color} set") }
        }
        Action::PlayRent { color, wild } => {
            let amount = g.players[who].rent(color);
            if wild {
                format!("charges {color} rent (${amount}M) to one player")
            } else {
                format!("charges {color} rent (${amount}M) to everyone")
            }
        }
        _ => {
            let s = a.to_string();
            let mut c = s.chars();
            match c.next() {
                Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
                None => s,
            }
        }
    }
}

fn effect_text(e: &Effect) -> String {
    match e {
        Effect::Charge(x) => format!("${x}M charge"),
        Effect::Sly { card, color } => format!("Sly Deal on {} ({color})", kind_of(*card)),
        Effect::Forced { theirs, their_color, .. } => {
            format!("Forced Deal for {} ({their_color})", kind_of(*theirs))
        }
        Effect::DealBreaker(c) => format!("Deal Breaker on {c}"),
    }
}

fn phase_text(g: &Game) -> (&'static str, String) {
    let a = g.active;
    match g.phase {
        Phase::Main => ("main", format!("P{a}'s turn: {} play(s) left", g.plays_left)),
        Phase::Discard => (
            "discard",
            format!("P{a} must discard down to 7 ({} in hand)", g.players[a].hand.len()),
        ),
        Phase::ChooseTarget(t) => {
            let what = match t {
                TargetChoice::RentPlayer(_) => "who pays the wild rent",
                TargetChoice::Debt => "who pays the Debt Collector",
                TargetChoice::Sly => "a property to Sly Deal",
                TargetChoice::ForcedTake => "a property to take (Forced Deal)",
                TargetChoice::ForcedGive { .. } => "a property to give away (Forced Deal)",
                TargetChoice::DealBreaker => "a set to Deal Break",
                TargetChoice::House { .. } => "a set for the House",
                TargetChoice::Hotel { .. } => "a set for the Hotel",
            };
            ("target", format!("P{a} is choosing {what}"))
        }
        Phase::OfferDouble { color, multiplier, .. } => (
            "double",
            format!(
                "P{a} may double the {color} rent (now ${}M)",
                g.players[a].rent(color) as u32 * multiplier as u32
            ),
        ),
        Phase::Respond { jsn_count } => {
            let r = g.pending.as_ref().unwrap();
            let who = g.current_player().unwrap();
            let text = if jsn_count % 2 == 0 {
                format!("P{who} may Just Say No to P{}'s {}", r.actor, effect_text(&r.effect))
            } else {
                format!("P{who} may Just Say No back to keep their {}", effect_text(&r.effect))
            };
            ("respond", text)
        }
        Phase::Pay { owed } => {
            let r = g.pending.as_ref().unwrap();
            ("pay", format!("P{} owes P{} ${owed}M: choose cards to pay with", r.victim(), r.actor))
        }
        Phase::GameOver => (
            "over",
            match g.winner {
                Some(w) => format!("P{w} wins!"),
                None => "Draw (turn limit)".to_string(),
            },
        ),
    }
}

fn state_json(app: &App, seat: i32) -> String {
    let g = &app.game;
    let mut s = String::with_capacity(8192);
    let current = g.current_player();
    let (phase, text) = phase_text(g);
    let _ = write!(
        s,
        "{{\"n\":{},\"turn\":{},\"active\":{},\"playsLeft\":{},\"current\":{},\"winner\":{},\
         \"phase\":\"{phase}\",\"phaseText\":{},\"deck\":{},\"discard\":{},\"discardTop\":{},",
        g.num_players,
        g.turn,
        g.active,
        g.plays_left,
        opt(current),
        opt(g.winner),
        jstr(&text),
        g.deck.len(),
        g.discard.len(),
        g.discard.last().map_or("null".to_string(), |&c| kind_of(c).index().to_string()),
    );
    let pend = g.pending.as_ref().map(|r| (r.actor, r.victim()));
    let _ = write!(
        s,
        "\"pendingActor\":{},\"pendingVictim\":{},",
        opt(pend.map(|p| p.0)),
        opt(pend.map(|p| p.1))
    );

    s.push_str("\"players\":[");
    for (i, p) in g.players.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let show_hand = seat < 0 || seat as usize == i;
        let _ = write!(
            s,
            "{{\"handCount\":{},\"hand\":{},\"bank\":{},\"bankValue\":{},\"sets\":{},\"table\":[",
            p.hand.len(),
            if show_hand { cards(&p.hand) } else { "null".to_string() },
            cards(&p.bank),
            p.bank_value(),
            p.complete_sets()
        );
        let mut first = true;
        for color in COLORS {
            let pile = p.pile(color);
            if pile.is_empty() && pile.house.is_none() {
                continue;
            }
            if !first {
                s.push(',');
            }
            first = false;
            let _ = write!(
                s,
                "{{\"color\":{},\"cards\":{},\"house\":{},\"hotel\":{},\"complete\":{},\"rent\":{}}}",
                color.idx(),
                cards(&pile.cards),
                opt(pile.house),
                opt(pile.hotel),
                p.is_complete(color),
                p.rent(color)
            );
        }
        s.push_str("]}");
    }
    s.push_str("],\"legal\":[");
    if let Some(cp) = current {
        if seat < 0 || seat as usize == cp {
            for (j, a) in g.legal_actions().into_iter().enumerate() {
                if j > 0 {
                    s.push(',');
                }
                action_json(&mut s, g, cp, a);
            }
        }
    }
    s.push_str("],\"log\":[");
    let start = app.log.len().saturating_sub(300);
    for (j, (p, t)) in app.log[start..].iter().enumerate() {
        if j > 0 {
            s.push(',');
        }
        let _ = write!(s, "{{\"p\":{p},\"t\":{}}}", jstr(t));
    }
    s.push_str("]}");
    s
}

#[allow(unused_assignments)]
fn action_json(s: &mut String, g: &Game, who: usize, a: Action) {
    let mut verb = "";
    let mut kind: Option<usize> = None;
    let mut color: Option<usize> = None;
    let mut seat: Option<usize> = None;
    let mut to: Option<usize> = None;
    let mut hotel = false;
    match a {
        Action::EndTurn => verb = "end",
        Action::Bank(k) => (verb, kind) = ("bank", Some(k.index())),
        Action::PlaceProperty { kind: k, color: c } => {
            (verb, kind, color) = ("place", Some(k.index()), Some(c.idx()))
        }
        Action::MoveWild { kind: k, from, to: t } => {
            (verb, kind, color, to, seat) = ("move", Some(k.index()), Some(from.idx()), Some(t.idx()), Some(who))
        }
        Action::PlayAction(t) => (verb, kind) = ("play", Some(Kind::Action(t).index())),
        Action::PlayRent { color: c, wild } => {
            let r = if wild { RentType::Wild } else { RentType::for_color(c) };
            (verb, kind, color) = ("rent", Some(Kind::Rent(r).index()), Some(c.idx()))
        }
        Action::TargetPlayer(r) => (verb, seat) = ("targetPlayer", Some(g.seat(who, r))),
        Action::TargetProperty { rel, kind: k, color: c } => {
            (verb, kind, color, seat) = ("targetProperty", Some(k.index()), Some(c.idx()), Some(g.seat(who, rel)))
        }
        Action::TargetSet { rel, color: c } => {
            (verb, color, seat) = ("targetSet", Some(c.idx()), Some(g.seat(who, rel)))
        }
        Action::PlayDouble => verb = "double",
        Action::DeclineDouble => verb = "noDouble",
        Action::JustSayNo => verb = "jsn",
        Action::Accept => verb = "accept",
        Action::PayBank(k) => (verb, kind, seat) = ("payBank", Some(k.index()), Some(who)),
        Action::PayProperty { kind: k, color: c } => {
            (verb, kind, color, seat) = ("payProperty", Some(k.index()), Some(c.idx()), Some(who))
        }
        Action::PayBuilding { color: c, hotel: h } => {
            (verb, color, seat, hotel) = ("payBuilding", Some(c.idx()), Some(who), h)
        }
        Action::Discard(k) => (verb, kind) = ("discard", Some(k.index())),
    }
    let label = {
        let d = describe_action(g, who, a);
        let mut c = d.chars();
        match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            None => d,
        }
    };
    let _ = write!(
        s,
        "{{\"i\":{},\"label\":{},\"verb\":\"{verb}\",\"kind\":{},\"color\":{},\"seat\":{},\"to\":{},\"hotel\":{hotel}}}",
        a.index(),
        jstr(&label),
        opt(kind),
        opt(color),
        opt(seat),
        opt(to)
    );
}

fn cards(ids: &[CardId]) -> String {
    let parts: Vec<String> =
        ids.iter().map(|&c| format!("{{\"id\":{c},\"k\":{}}}", kind_of(c).index())).collect();
    format!("[{}]", parts.join(","))
}

fn color_list(mask: u16) -> String {
    let v: Vec<String> = iter_colors(mask).map(|c| c.idx().to_string()).collect();
    format!("[{}]", v.join(","))
}

fn opt<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map_or("null".to_string(), |x| x.to_string())
}

fn jstr(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The log must not reveal that a player held a Just Say No or a Double
    /// they chose not to use.
    #[test]
    fn log_never_reveals_declined_responses() {
        let mut saw_jsn = false;
        for seed in 0..300 {
            new_game(3, seed);
            for _ in 0..5000 {
                if bot_step() == 0 {
                    break;
                }
            }
            APP.with(|a| {
                let app = a.borrow();
                for (_, t) in &app.as_ref().unwrap().log {
                    assert!(!t.starts_with("accept"), "log reveals a held JSN: {t}");
                    assert!(!t.starts_with("don't double"), "log reveals a held Double: {t}");
                    saw_jsn |= t.contains("Just Say No");
                }
            });
            state(-1); // the JSON builder runs on every state
        }
        assert!(saw_jsn, "test should see some Just Say No plays");
    }
}
