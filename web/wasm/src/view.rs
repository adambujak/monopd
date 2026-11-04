//! What the page receives: static card data, and the game state for a seat.

use crate::json::{obj, Raw, ToJson};
use crate::text;
use crate::App;
use monopd_engine::cards::*;
use monopd_engine::game::{Game, Player};
use monopd_engine::Action;

/// How many log lines the page gets.
const LOG_LINES: usize = 300;

/// Every card kind and colour, indexed like the engine (`Kind::index`, `Color::idx`).
pub fn catalog() -> Raw {
    let kinds: Vec<Raw> = ALL_KINDS
        .iter()
        .map(|&k| {
            let (kind_type, colors) = match k {
                Kind::Property(_) => ("property", k.colors()),
                Kind::Wild(_) => ("wild", k.colors()),
                Kind::Action(_) => ("action", 0),
                Kind::Rent(r) => ("rent", r.colors()),
                Kind::Money(_) => ("money", 0),
            };
            let short = match k {
                Kind::Property(c) => text::color_label(c),
                Kind::Wild(_) => "Wild".to_string(),
                Kind::Action(a) => a.name().to_string(),
                Kind::Rent(RentType::Wild) => "Wild Rent".to_string(),
                Kind::Rent(_) => "Rent".to_string(),
                Kind::Money(_) => format!("${}M", k.value()),
            };
            obj! {
                "name": k.to_string(),
                "short": short,
                "slug": text::kind_slug(k),
                "type": kind_type,
                "value": k.value(),
                "colors": color_list(colors),
            }
        })
        .collect();
    let colors: Vec<Raw> = COLORS
        .iter()
        .map(|&c| {
            let rent: Vec<u8> = (1..=c.set_size()).map(|n| c.rent(n)).collect();
            obj! { "label": text::color_label(c), "size": c.set_size(), "rent": rent }
        })
        .collect();
    obj! { "kinds": kinds, "colors": colors }
}

/// The game as `seat` sees it. `seat` = -1 shows every hand and lists the
/// current player's legal actions, whoever that is.
pub fn state(app: &App, seat: i32) -> Raw {
    let g = &app.game;
    let sees = |p: usize| seat < 0 || seat as usize == p;
    let current = g.current_player();
    let (phase, phase_text) = text::phase(g);
    let players: Vec<Raw> = g.players.iter().enumerate().map(|(i, p)| player(p, sees(i))).collect();
    let legal: Vec<Raw> = match current {
        Some(cp) if sees(cp) => g.legal_actions().into_iter().map(|a| action(g, cp, a)).collect(),
        _ => Vec::new(),
    };
    let log: Vec<Raw> = app.log[app.log.len().saturating_sub(LOG_LINES)..]
        .iter()
        .map(|(p, t)| obj! { "p": *p, "t": t })
        .collect();
    obj! {
        "n": g.num_players,
        "turn": g.turn,
        "active": g.active,
        "playsLeft": g.plays_left,
        "current": current,
        "winner": g.winner,
        "phase": phase,
        "phaseText": phase_text,
        "deck": g.deck.len(),
        "discard": g.discard.len(),
        "discardTop": g.discard.last().map(|&c| kind_of(c).index()),
        "players": players,
        "legal": legal,
        "log": log,
    }
}

fn player(p: &Player, show_hand: bool) -> Raw {
    let table: Vec<Raw> = COLORS
        .iter()
        .filter(|&&c| !p.pile(c).is_empty())
        .map(|&c| {
            let pile = p.pile(c);
            obj! {
                "color": c.idx(),
                "cards": cards(&pile.cards),
                "house": pile.house.is_some(),
                "hotel": pile.hotel.is_some(),
                "complete": p.is_complete(c),
                "rent": p.rent(c),
            }
        })
        .collect();
    obj! {
        "handCount": p.hand.len(),
        "hand": show_hand.then(|| cards(&p.hand)),
        "bank": cards(&p.bank),
        "bankValue": p.bank_value(),
        "sets": p.complete_sets(),
        "table": table,
    }
}

/// One legal action: its index, a label, and what it refers to on the table
/// (with absolute seats) so the page can attach it to the right card.
fn action(g: &Game, who: usize, a: Action) -> Raw {
    #[derive(Default)]
    struct Refs {
        verb: &'static str,
        kind: Option<usize>,
        color: Option<usize>,
        seat: Option<usize>,
        to: Option<usize>,
        hotel: bool,
    }
    let k = |kind: Kind| Some(kind.index());
    let c = |color: Color| Some(color.idx());
    let mine = Some(who);
    let r = match a {
        Action::EndTurn => Refs { verb: "end", ..Refs::default() },
        Action::Bank(kind) => Refs { verb: "bank", kind: k(kind), ..Refs::default() },
        Action::PlaceProperty { kind, color } => Refs { verb: "place", kind: k(kind), color: c(color), ..Refs::default() },
        Action::MoveWild { kind, from, to } => {
            Refs { verb: "move", kind: k(kind), color: c(from), to: c(to), seat: mine, ..Refs::default() }
        }
        Action::PlayAction(t) => Refs { verb: "play", kind: k(Kind::Action(t)), ..Refs::default() },
        Action::PlayRent { color, wild } => {
            let rent = if wild { RentType::Wild } else { RentType::for_color(color) };
            Refs { verb: "rent", kind: k(Kind::Rent(rent)), color: c(color), ..Refs::default() }
        }
        Action::TargetPlayer(rel) => Refs { verb: "targetPlayer", seat: Some(g.seat(who, rel)), ..Refs::default() },
        Action::TargetProperty { rel, kind, color } => Refs {
            verb: "targetProperty",
            kind: k(kind),
            color: c(color),
            seat: Some(g.seat(who, rel)),
            ..Refs::default()
        },
        Action::TargetSet { rel, color } => {
            Refs { verb: "targetSet", color: c(color), seat: Some(g.seat(who, rel)), ..Refs::default() }
        }
        Action::PlayDouble => Refs { verb: "double", kind: k(Kind::Action(ActionType::DoubleRent)), ..Refs::default() },
        Action::DeclineDouble => Refs { verb: "noDouble", ..Refs::default() },
        Action::JustSayNo => Refs { verb: "jsn", kind: k(Kind::Action(ActionType::JustSayNo)), ..Refs::default() },
        Action::Accept => Refs { verb: "accept", ..Refs::default() },
        Action::PayBank(kind) => Refs { verb: "payBank", kind: k(kind), seat: mine, ..Refs::default() },
        Action::PayProperty { kind, color } => {
            Refs { verb: "payProperty", kind: k(kind), color: c(color), seat: mine, ..Refs::default() }
        }
        Action::PayBuilding { color, hotel } => {
            Refs { verb: "payBuilding", color: c(color), seat: mine, hotel, ..Refs::default() }
        }
        Action::Discard(kind) => Refs { verb: "discard", kind: k(kind), ..Refs::default() },
    };
    obj! {
        "i": a.index(),
        "label": text::label(g, who, a),
        "verb": r.verb,
        "kind": r.kind,
        "color": r.color,
        "seat": r.seat,
        "to": r.to,
        "hotel": r.hotel,
    }
}

fn cards(ids: &[CardId]) -> Raw {
    let list: Vec<Raw> = ids.iter().map(|&c| obj! { "id": c, "k": kind_of(c).index() }).collect();
    Raw(list.to_json())
}

fn color_list(mask: u16) -> Vec<usize> {
    iter_colors(mask).map(Color::idx).collect()
}
