//! Human-readable wording for actions, phases and effects.

use monopd_engine::cards::*;
use monopd_engine::game::{Effect, Game, Phase, TargetChoice};
use monopd_engine::Action;

/// What `who` does, as a log line ("charges Red rent ($3M) to everyone").
/// Like `Action`'s Display, but with absolute seats ("P2", not "player+1").
pub fn describe(g: &Game, who: usize, a: Action) -> String {
    let seat = |rel: u8| g.seat(who, rel);
    match a {
        Action::TargetPlayer(rel) => format!("targets P{}", seat(rel)),
        Action::TargetProperty { rel, kind, color } if seat(rel) == who => format!("gives their {kind} ({color})"),
        Action::TargetProperty { rel, kind, color } => format!("targets P{}'s {kind} ({color})", seat(rel)),
        Action::TargetSet { rel, color } if seat(rel) == who => format!("builds on {color}"),
        Action::TargetSet { rel, color } => format!("targets P{}'s {color} set", seat(rel)),
        Action::PlayRent { color, wild } => {
            let amount = g.players[who].rent(color);
            let whom = if wild { "one player" } else { "everyone" };
            format!("charges {color} rent (${amount}M) to {whom}")
        }
        _ => lowercase_first(&a.to_string()),
    }
}

/// `describe` as a button label ("Charges Red rent ...").
pub fn label(g: &Game, who: usize, a: Action) -> String {
    let d = describe(g, who, a);
    let mut chars = d.chars();
    chars.next().map_or(d.clone(), |c| c.to_uppercase().chain(chars).collect())
}

fn lowercase_first(s: &str) -> String {
    let mut chars = s.chars();
    chars.next().map_or_else(String::new, |c| c.to_lowercase().chain(chars).collect())
}

/// A short phase name for the page and a sentence describing the moment.
pub fn phase(g: &Game) -> (&'static str, String) {
    let a = g.active;
    match g.phase {
        Phase::Main => ("main", format!("P{a}'s turn: {} play(s) left", g.plays_left)),
        Phase::Discard => ("discard", format!("P{a} must discard down to 7 ({} in hand)", g.players[a].hand.len())),
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
        Phase::OfferDouble { color, multiplier, .. } => {
            let now = g.players[a].rent(color) as u32 * multiplier as u32;
            ("double", format!("P{a} may double the {color} rent (now ${now}M)"))
        }
        Phase::Respond { jsn_count } => {
            let r = g.pending.as_ref().expect("responding without a pending action");
            let who = g.current_player().unwrap_or(r.victim());
            let text = if jsn_count % 2 == 0 {
                format!("P{who} may Just Say No to P{}'s {}", r.actor, effect(&r.effect))
            } else {
                format!("P{who} may Just Say No back to keep their {}", effect(&r.effect))
            };
            ("respond", text)
        }
        Phase::Pay { owed } => {
            let r = g.pending.as_ref().expect("paying without a debt");
            ("pay", format!("P{} owes P{} ${owed}M: choose cards to pay with", r.victim(), r.actor))
        }
        Phase::GameOver => {
            let text = g.winner.map_or_else(|| "Draw (turn limit)".to_string(), |w| format!("P{w} wins!"));
            ("over", text)
        }
    }
}

fn effect(e: &Effect) -> String {
    match *e {
        Effect::Charge(x) => format!("${x}M charge"),
        Effect::Sly { card, color } => format!("Sly Deal on {} ({color})", kind_of(card)),
        Effect::Forced { theirs, their_color, .. } => format!("Forced Deal for {} ({their_color})", kind_of(theirs)),
        Effect::DealBreaker(c) => format!("Deal Breaker on {c}"),
    }
}

/// "LightBlue" -> "Light Blue".
pub fn color_label(c: Color) -> String {
    let mut out = String::new();
    for (i, ch) in c.name().chars().enumerate() {
        if i > 0 && ch.is_uppercase() {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

/// Stable name for a card kind: the key used in `web/cards/skin.json`.
pub fn kind_slug(k: Kind) -> String {
    let colors = |mask: u16| iter_colors(mask).map(|c| c.name().to_lowercase()).collect::<Vec<_>>().join("-");
    match k {
        Kind::Property(c) => format!("property-{}", c.name().to_lowercase()),
        Kind::Wild(WildType::Rainbow) => "wild-rainbow".to_string(),
        Kind::Wild(w) => format!("wild-{}", colors(w.colors())),
        Kind::Rent(RentType::Wild) => "rent-wild".to_string(),
        Kind::Rent(r) => format!("rent-{}", colors(r.colors())),
        Kind::Money(_) => format!("money-{}", k.value()),
        Kind::Action(a) => {
            let name: String = a.name().to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
            format!("action-{}", name.trim_matches('-'))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_match_the_skin_readme() {
        assert_eq!(kind_slug(Kind::Property(Color::LightBlue)), "property-lightblue");
        assert_eq!(kind_slug(Kind::Wild(WildType::DarkBlueGreen)), "wild-green-darkblue");
        assert_eq!(kind_slug(Kind::Rent(RentType::RailroadUtility)), "rent-railroad-utility");
        assert_eq!(kind_slug(Kind::Action(ActionType::Birthday)), "action-it-s-my-birthday");
        assert_eq!(kind_slug(Kind::Money(5)), "money-10");
        let mut slugs: Vec<_> = ALL_KINDS.iter().map(|&k| kind_slug(k)).collect();
        slugs.sort();
        slugs.dedup();
        assert_eq!(slugs.len(), NUM_KINDS, "slugs are unique");
    }

    #[test]
    fn colour_labels_have_spaces() {
        assert_eq!(color_label(Color::DarkBlue), "Dark Blue");
        assert_eq!(color_label(Color::Red), "Red");
    }
}
