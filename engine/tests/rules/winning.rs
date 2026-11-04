use crate::common::*;

#[test]
fn three_complete_sets_of_different_colours_win() {
    let mut g = Game::new_test(2);
    complete_set(&mut g, 0, Color::Brown);
    complete_set(&mut g, 0, Color::DarkBlue);
    // An overflowing Brown pile doesn't count twice.
    g.give_table(0, Kind::Wild(WildType::LightBlueBrown), Color::Brown);
    g.give_table(0, RAINBOW, Color::Brown);
    g.give_hand(0, prop(Color::Utility));
    g.give_hand(0, prop(Color::Utility));
    step(&mut g, PlaceProperty { kind: prop(Color::Utility), color: Color::Utility });
    assert!(!g.is_over());
    step(&mut g, PlaceProperty { kind: prop(Color::Utility), color: Color::Utility });
    assert_eq!(g.winner, Some(0));
    assert!(g.is_over());
    assert_eq!(g.current_player(), None);
    assert!(g.legal_actions().is_empty());
    assert_eq!(g.returns(), vec![1.0, -1.0]);
}

#[test]
fn a_payment_can_win_the_game_on_someone_elses_turn() {
    let mut g = Game::new_test(2);
    g.active = 1;
    complete_set(&mut g, 1, Color::Brown);
    complete_set(&mut g, 1, Color::DarkBlue);
    g.give_table(1, prop(Color::Utility), Color::Utility);
    g.give_hand(1, Kind::Rent(RentType::LightBlueBrown));
    g.give_table(0, prop(Color::Utility), Color::Utility);
    g.give_table(0, prop(Color::Pink), Color::Pink);
    g.give_table(0, prop(Color::Pink), Color::Pink);
    step(&mut g, PlayRent { color: Color::Brown, wild: false }); // $2M
    assert_eq!(g.current_player(), Some(0));
    step(&mut g, PayProperty { kind: prop(Color::Utility), color: Color::Utility });
    assert_eq!(g.winner, Some(1), "P0's Utility completed P1's third set");
}

#[test]
fn a_forced_deal_that_completes_both_players_goes_to_the_mover() {
    // P1 takes P0's Railroad (completing P1's Railroads) and gives P1's Utility
    // (completing P0's Utilities): both reach 3 sets at once.
    let mut g = Game::new_test(2);
    g.active = 1;
    complete_set(&mut g, 0, Color::Green);
    complete_set(&mut g, 0, Color::Red);
    complete_set(&mut g, 1, Color::Yellow);
    complete_set(&mut g, 1, Color::Orange);
    g.give_table(0, prop(Color::Utility), Color::Utility);
    g.give_table(0, prop(Color::Railroad), Color::Railroad);
    g.give_table(1, prop(Color::Utility), Color::Utility);
    g.give_table(1, Kind::Wild(WildType::LightBlueRailroad), Color::Railroad);
    g.give_table(1, RAINBOW, Color::Railroad);
    g.give_table(1, RAINBOW, Color::Railroad);
    g.give_hand(1, act(ActionType::ForcedDeal));
    step(&mut g, PlayAction(ActionType::ForcedDeal));
    step(&mut g, TargetProperty { rel: 1, kind: prop(Color::Railroad), color: Color::Railroad });
    step(&mut g, TargetProperty { rel: 0, kind: prop(Color::Utility), color: Color::Utility });
    assert_eq!((g.players[0].complete_sets(), g.players[1].complete_sets()), (3, 3));
    assert_eq!(g.winner, Some(1));
}

#[test]
fn a_deal_breaker_can_win() {
    let mut g = Game::new_test(2);
    complete_set(&mut g, 0, Color::Brown);
    complete_set(&mut g, 0, Color::Utility);
    complete_set(&mut g, 1, Color::DarkBlue);
    g.give_hand(0, act(ActionType::DealBreaker));
    step(&mut g, PlayAction(ActionType::DealBreaker));
    step(&mut g, TargetSet { rel: 1, color: Color::DarkBlue });
    assert_eq!(g.winner, Some(0));
}
