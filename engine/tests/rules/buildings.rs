use crate::common::*;

#[test]
fn a_house_needs_a_complete_set_and_not_a_railroad_or_utility() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::House));
    complete_set(&mut g, 0, Color::Utility);
    g.give_table(0, RED, Color::Red);
    g.give_table(0, RED, Color::Red);
    assert!(!g.legal_actions().contains(&PlayAction(ActionType::House)));
    g.give_table(0, RED, Color::Red);
    step(&mut g, PlayAction(ActionType::House));
    assert_eq!(g.legal_actions(), vec![TargetSet { rel: 0, color: Color::Red }]);
    step(&mut g, TargetSet { rel: 0, color: Color::Red });
    assert!(g.players[0].pile(Color::Red).house.is_some());
    assert_eq!(g.plays_left, 2);
}

#[test]
fn a_hotel_needs_a_house_first() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::House));
    g.give_hand(0, act(ActionType::Hotel));
    complete_set(&mut g, 0, Color::Red);
    assert!(!g.legal_actions().contains(&PlayAction(ActionType::Hotel)));
    step(&mut g, PlayAction(ActionType::House));
    step(&mut g, TargetSet { rel: 0, color: Color::Red });
    step(&mut g, PlayAction(ActionType::Hotel));
    step(&mut g, TargetSet { rel: 0, color: Color::Red });
    assert_eq!(g.players[0].rent(Color::Red), 6 + 3 + 4);
}

#[test]
fn only_one_house_per_set() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::House));
    complete_set(&mut g, 0, Color::Red);
    g.give_building(0, Color::Red, false);
    assert!(!g.legal_actions().contains(&PlayAction(ActionType::House)));
}
