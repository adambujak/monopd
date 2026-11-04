//! Shared helpers and names for the rule tests.

pub use monopd_engine::action::Action::*;
pub use monopd_engine::cards::*;
pub use monopd_engine::game::{Phase, TargetChoice};
pub use monopd_engine::{Action, Game, StepError};

pub const RED: Kind = Kind::Property(Color::Red);
pub const GREEN: Kind = Kind::Property(Color::Green);
pub const BROWN: Kind = Kind::Property(Color::Brown);
pub const DARKBLUE: Kind = Kind::Property(Color::DarkBlue);
pub const RAINBOW: Kind = Kind::Wild(WildType::Rainbow);
pub const M1: Kind = Kind::Money(0);
pub const M2: Kind = Kind::Money(1);
pub const M3: Kind = Kind::Money(2);
pub const M5: Kind = Kind::Money(4);
pub const M10: Kind = Kind::Money(5);
pub const JSN: Kind = Kind::Action(ActionType::JustSayNo);
pub const DOUBLE: Kind = Kind::Action(ActionType::DoubleRent);

pub fn act(t: ActionType) -> Kind {
    Kind::Action(t)
}

pub fn prop(c: Color) -> Kind {
    Kind::Property(c)
}

/// Apply a legal action, check the state, then answer any forced "no".
///
/// Everyone targeted is asked about Just Say No (and the renter about Double
/// The Rent) whether or not they hold one. When "no" is the only possible
/// answer, take it, so tests read as the real decisions. To see those forced
/// questions, call `g.step` directly.
pub fn step(g: &mut Game, a: Action) {
    g.step(a).unwrap_or_else(|e| panic!("{e}; legal: {:?}", g.legal_actions()));
    g.check_invariants();
    loop {
        let legal = g.legal_actions();
        if legal != [Accept] && legal != [DeclineDouble] {
            return;
        }
        g.step(legal[0]).unwrap();
        g.check_invariants();
    }
}

pub fn bank(g: &Game, p: usize) -> u32 {
    g.players[p].bank_value()
}

/// Fill `p`'s `color` pile with plain properties up to a complete set.
pub fn complete_set(g: &mut Game, p: usize, color: Color) {
    for _ in 0..color.set_size() {
        g.give_table(p, prop(color), color);
    }
}

pub fn legal_of<F: Fn(&Action) -> bool>(g: &Game, f: F) -> Vec<Action> {
    g.legal_actions().into_iter().filter(|a| f(a)).collect()
}
