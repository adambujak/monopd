use crate::common::*;

fn debt_collector_on_p1(g: &mut Game) {
    g.give_hand(0, act(ActionType::DebtCollector));
    g.give_bank(1, M5);
    step(g, PlayAction(ActionType::DebtCollector));
    step(g, TargetPlayer(1));
}

#[test]
fn just_say_no_cancels_the_action() {
    let mut g = Game::new_test(2);
    g.give_hand(1, JSN);
    debt_collector_on_p1(&mut g);
    assert_eq!((g.phase, g.current_player()), (Phase::Respond { jsn_count: 0 }, Some(1)));
    step(&mut g, JustSayNo);
    assert_eq!(g.phase, Phase::Main);
    assert_eq!(bank(&g, 1), 5);
}

#[test]
fn a_just_say_no_can_be_countered() {
    let mut g = Game::new_test(2);
    g.give_hand(0, JSN);
    g.give_hand(1, JSN);
    debt_collector_on_p1(&mut g);
    step(&mut g, JustSayNo);
    assert_eq!((g.phase, g.current_player()), (Phase::Respond { jsn_count: 1 }, Some(0)));
    step(&mut g, JustSayNo); // the debt stands again
    assert_eq!(bank(&g, 0), 5);
    assert_eq!(g.plays_left, 2, "a Just Say No in response doesn't use a play");
}

#[test]
fn a_chain_of_three_cancels_again() {
    let mut g = Game::new_test(2);
    g.give_hand(0, JSN);
    g.give_hand(1, JSN);
    g.give_hand(1, JSN);
    debt_collector_on_p1(&mut g);
    step(&mut g, JustSayNo); // P1: no
    step(&mut g, JustSayNo); // P0: no to that
    assert_eq!((g.phase, g.current_player()), (Phase::Respond { jsn_count: 2 }, Some(1)));
    step(&mut g, JustSayNo); // P1: no again
    assert_eq!(g.phase, Phase::Main);
    assert_eq!(bank(&g, 1), 5, "three Just Say Nos: cancelled");
    assert_eq!(g.discard.len(), 4, "the Debt Collector and three Just Say Nos");
}

#[test]
fn it_protects_only_the_player_who_plays_it() {
    let mut g = Game::new_test(3);
    g.give_hand(0, act(ActionType::Birthday));
    g.give_hand(1, JSN);
    g.give_bank(1, M2);
    g.give_bank(2, M2);
    step(&mut g, PlayAction(ActionType::Birthday));
    step(&mut g, JustSayNo);
    assert_eq!((bank(&g, 0), bank(&g, 1), bank(&g, 2)), (2, 2, 0));
}

#[test]
fn it_stops_a_steal() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::SlyDeal));
    g.give_hand(1, JSN);
    g.give_table(1, GREEN, Color::Green);
    step(&mut g, PlayAction(ActionType::SlyDeal));
    step(&mut g, TargetProperty { rel: 1, kind: GREEN, color: Color::Green });
    step(&mut g, JustSayNo);
    assert_eq!(g.players[1].pile(Color::Green).len(), 1);
    assert!(g.players[0].pile(Color::Green).is_empty());
}

#[test]
fn holding_one_but_accepting_still_pays() {
    let mut g = Game::new_test(2);
    g.give_hand(1, JSN);
    debt_collector_on_p1(&mut g);
    step(&mut g, Accept);
    assert_eq!(bank(&g, 0), 5);
    assert!(g.players[1].holds(JSN));
}

// Who gets asked must not depend on hidden hands, or it would leak them.

#[test]
fn every_victim_is_asked_even_without_one() {
    let mut g = Game::new_test(3);
    g.give_hand(0, act(ActionType::Birthday));
    g.give_hand(2, JSN);
    g.step(PlayAction(ActionType::Birthday)).unwrap();
    assert_eq!((g.phase, g.current_player()), (Phase::Respond { jsn_count: 0 }, Some(1)));
    assert_eq!(g.legal_actions(), vec![Accept], "P1 has none: accepting is the only answer");
    g.step(Accept).unwrap();
    assert_eq!(g.current_player(), Some(2));
    assert_eq!(g.legal_actions(), vec![JustSayNo, Accept]);
}

#[test]
fn the_actor_is_always_asked_to_counter() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::DebtCollector));
    g.give_hand(1, JSN);
    g.give_bank(1, M5);
    g.step(PlayAction(ActionType::DebtCollector)).unwrap();
    g.step(TargetPlayer(1)).unwrap();
    g.step(JustSayNo).unwrap();
    assert_eq!((g.phase, g.current_player()), (Phase::Respond { jsn_count: 1 }, Some(0)));
    assert_eq!(g.legal_actions(), vec![Accept]);
    g.step(Accept).unwrap();
    assert_eq!(bank(&g, 1), 5, "cancelled");
}

#[test]
fn double_the_rent_is_offered_even_without_one() {
    let mut g = Game::new_test(2);
    g.give_table(0, RED, Color::Red);
    g.give_hand(0, Kind::Rent(RentType::RedYellow));
    g.step(PlayRent { color: Color::Red, wild: false }).unwrap();
    assert!(matches!(g.phase, Phase::OfferDouble { .. }));
    assert_eq!(g.legal_actions(), vec![DeclineDouble]);
}
