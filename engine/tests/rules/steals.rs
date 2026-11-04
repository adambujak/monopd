use crate::common::*;

#[test]
fn sly_deal_cannot_take_from_a_complete_set() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::SlyDeal));
    complete_set(&mut g, 1, Color::Red);
    assert!(!g.legal_actions().contains(&PlayAction(ActionType::SlyDeal)), "nothing to take");
    g.give_table(1, GREEN, Color::Green);
    step(&mut g, PlayAction(ActionType::SlyDeal));
    assert_eq!(g.legal_actions(), vec![TargetProperty { rel: 1, kind: GREEN, color: Color::Green }]);
    step(&mut g, TargetProperty { rel: 1, kind: GREEN, color: Color::Green });
    assert_eq!(g.players[0].pile(Color::Green).len(), 1);
    assert!(g.players[1].pile(Color::Green).is_empty());
}

#[test]
fn a_stolen_wild_keeps_its_colour() {
    let mut g = Game::new_test(2);
    let ry = Kind::Wild(WildType::RedYellow);
    g.give_hand(0, act(ActionType::SlyDeal));
    g.give_table(1, ry, Color::Yellow);
    step(&mut g, PlayAction(ActionType::SlyDeal));
    step(&mut g, TargetProperty { rel: 1, kind: ry, color: Color::Yellow });
    assert_eq!(g.players[0].pile(Color::Yellow).len(), 1);
}

#[test]
fn forced_deal_swaps_one_property_each_way() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::ForcedDeal));
    g.give_table(0, BROWN, Color::Brown);
    g.give_table(1, GREEN, Color::Green);
    step(&mut g, PlayAction(ActionType::ForcedDeal));
    step(&mut g, TargetProperty { rel: 1, kind: GREEN, color: Color::Green });
    assert_eq!(g.legal_actions(), vec![TargetProperty { rel: 0, kind: BROWN, color: Color::Brown }]);
    step(&mut g, TargetProperty { rel: 0, kind: BROWN, color: Color::Brown });
    assert_eq!(g.players[0].pile(Color::Green).len(), 1);
    assert_eq!(g.players[1].pile(Color::Brown).len(), 1);
    assert!(g.players[0].pile(Color::Brown).is_empty());
}

#[test]
fn forced_deal_needs_something_of_yours_to_give() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::ForcedDeal));
    g.give_table(1, GREEN, Color::Green);
    assert!(!g.legal_actions().contains(&PlayAction(ActionType::ForcedDeal)));
    complete_set(&mut g, 0, Color::Brown);
    assert!(!g.legal_actions().contains(&PlayAction(ActionType::ForcedDeal)), "can't give from a complete set");
}

#[test]
fn deal_breaker_takes_a_whole_set_with_its_buildings() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::DealBreaker));
    complete_set(&mut g, 1, Color::DarkBlue);
    g.give_building(1, Color::DarkBlue, false);
    step(&mut g, PlayAction(ActionType::DealBreaker));
    step(&mut g, TargetSet { rel: 1, color: Color::DarkBlue });
    assert!(g.players[0].is_complete(Color::DarkBlue));
    assert!(g.players[0].pile(Color::DarkBlue).house.is_some());
    assert!(g.players[1].pile(Color::DarkBlue).is_empty());
}

#[test]
fn deal_breaker_needs_a_complete_set_to_take() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::DealBreaker));
    g.give_table(1, RED, Color::Red);
    assert!(!g.legal_actions().contains(&PlayAction(ActionType::DealBreaker)));
}

#[test]
fn a_deal_broken_set_merging_into_a_built_set_banks_the_extra_house() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::DealBreaker));
    g.give_table(0, Kind::Wild(WildType::DarkBlueGreen), Color::DarkBlue);
    g.give_table(0, RAINBOW, Color::DarkBlue);
    g.give_building(0, Color::DarkBlue, false);
    complete_set(&mut g, 1, Color::DarkBlue);
    g.give_building(1, Color::DarkBlue, false);
    step(&mut g, PlayAction(ActionType::DealBreaker));
    step(&mut g, TargetSet { rel: 1, color: Color::DarkBlue });
    assert_eq!(g.players[0].pile(Color::DarkBlue).len(), 4);
    assert!(g.players[0].pile(Color::DarkBlue).house.is_some());
    assert_eq!(bank(&g, 0), 3, "only one house fits; the other is banked");
}
