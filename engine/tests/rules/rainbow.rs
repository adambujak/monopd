//! The rainbow (multi-colour) wild has no colour of its own.

use crate::common::*;

#[test]
fn it_can_be_laid_down_alone_as_any_colour() {
    let mut g = Game::new_test(2);
    g.give_hand(0, RAINBOW);
    assert_eq!(legal_of(&g, |a| matches!(a, PlaceProperty { .. })).len(), NUM_COLORS);
    step(&mut g, PlaceProperty { kind: RAINBOW, color: Color::Red });
    assert_eq!(g.players[0].pile(Color::Red).len(), 1);
}

#[test]
fn alone_it_collects_no_rent() {
    let mut g = Game::new_test(2);
    g.give_table(0, RAINBOW, Color::Red);
    g.give_hand(0, Kind::Rent(RentType::RedYellow));
    assert_eq!(g.players[0].rent(Color::Red), 0);
    assert!(legal_of(&g, |a| matches!(a, PlayRent { .. })).is_empty());
    g.give_table(0, RED, Color::Red);
    assert_eq!(g.players[0].rent(Color::Red), 3, "with a real Red, both count: 2 Red = $3M");
    assert!(g.legal_actions().contains(&PlayRent { color: Color::Red, wild: false }));
}

#[test]
fn rainbows_alone_are_not_a_set_or_protected() {
    let mut g = Game::new_test(2);
    g.give_table(0, RAINBOW, Color::Brown);
    g.give_table(0, RAINBOW, Color::Brown);
    assert_eq!(g.players[0].complete_sets(), 0);
    g.active = 1;
    g.give_hand(1, act(ActionType::SlyDeal));
    assert!(g.legal_actions().contains(&PlayAction(ActionType::SlyDeal)));
}

#[test]
fn with_a_two_colour_wild_it_completes_a_set() {
    let mut g = Game::new_test(2);
    g.give_table(0, Kind::Wild(WildType::LightBlueBrown), Color::Brown);
    g.give_table(0, RAINBOW, Color::Brown);
    assert!(g.players[0].is_complete(Color::Brown));
}

#[test]
fn it_is_worth_nothing_as_payment() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::DebtCollector));
    g.give_table(1, RAINBOW, Color::Red);
    step(&mut g, PlayAction(ActionType::DebtCollector));
    step(&mut g, TargetPlayer(1));
    assert_eq!(g.phase, Phase::Main, "nothing payable, so nothing paid");
    assert_eq!(g.players[1].pile(Color::Red).len(), 1);
}
