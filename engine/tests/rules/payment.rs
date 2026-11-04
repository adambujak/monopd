use crate::common::*;

fn debt_collector_on_p1(g: &mut Game) {
    g.give_hand(0, act(ActionType::DebtCollector));
    step(g, PlayAction(ActionType::DebtCollector));
    step(g, TargetPlayer(1));
}

#[test]
fn a_short_payer_hands_over_everything_of_value() {
    let mut g = Game::new_test(2);
    g.give_bank(1, M1);
    g.give_table(1, GREEN, Color::Green);
    g.give_table(1, RAINBOW, Color::Green); // worth $0, stays
    debt_collector_on_p1(&mut g);
    assert_eq!(g.phase, Phase::Main);
    assert_eq!(bank(&g, 0), 1);
    assert_eq!(g.players[0].pile(Color::Green).len(), 1, "the property goes to the receiver's table");
    assert_eq!(g.players[1].pile(Color::Green).len(), 1, "the rainbow wild can't be paid");
}

#[test]
fn handing_over_everything_includes_buildings() {
    let mut g = Game::new_test(2);
    complete_set(&mut g, 1, Color::Brown);
    g.give_building(1, Color::Brown, false); // 1 + 1 + 3 = $5M: exactly enough
    debt_collector_on_p1(&mut g);
    assert!(g.players[1].pile(Color::Brown).is_empty());
    assert_eq!(g.players[0].pile(Color::Brown).len(), 2);
    assert_eq!(g.players[0].pile(Color::Brown).house, None, "received buildings go to the bank");
    assert_eq!(bank(&g, 0), 3);
}

#[test]
fn a_payer_with_nothing_pays_nothing() {
    let mut g = Game::new_test(2);
    debt_collector_on_p1(&mut g);
    assert_eq!(g.phase, Phase::Main);
}

#[test]
fn paying_from_a_built_set_moves_its_buildings_to_the_bank() {
    let mut g = Game::new_test(2);
    complete_set(&mut g, 1, Color::Brown);
    g.give_building(1, Color::Brown, false);
    g.give_bank(1, M10);
    debt_collector_on_p1(&mut g);
    let legal = g.legal_actions();
    assert!(legal.contains(&PayBuilding { color: Color::Brown, hotel: false }));
    assert!(legal.contains(&PayProperty { kind: BROWN, color: Color::Brown }));
    step(&mut g, PayProperty { kind: BROWN, color: Color::Brown }); // $1M paid
    assert_eq!(g.players[1].pile(Color::Brown).house, None);
    assert_eq!(bank(&g, 1), 10 + 3, "the house moved to the payer's bank");
    assert_eq!(g.phase, Phase::Pay { owed: 4 });
    step(&mut g, PayBank(act(ActionType::House)));
    assert_eq!(g.phase, Phase::Pay { owed: 1 });
    step(&mut g, PayProperty { kind: BROWN, color: Color::Brown });
    assert_eq!(g.phase, Phase::Main);
    assert_eq!(g.players[0].pile(Color::Brown).len(), 2);
    assert_eq!(bank(&g, 1), 10);
}

#[test]
fn a_hotel_must_be_paid_before_its_house() {
    let mut g = Game::new_test(2);
    complete_set(&mut g, 1, Color::Brown);
    g.give_building(1, Color::Brown, false);
    g.give_building(1, Color::Brown, true);
    g.give_bank(1, M10);
    debt_collector_on_p1(&mut g);
    let buildings = legal_of(&g, |a| matches!(a, PayBuilding { .. }));
    assert_eq!(buildings, vec![PayBuilding { color: Color::Brown, hotel: true }]);
}

#[test]
fn birthday_collects_from_everyone() {
    let mut g = Game::new_test(4);
    g.give_hand(0, act(ActionType::Birthday));
    for p in 1..4 {
        g.give_bank(p, M2);
    }
    step(&mut g, PlayAction(ActionType::Birthday));
    assert_eq!(bank(&g, 0), 6);
}

#[test]
fn debt_collector_charges_only_its_target() {
    let mut g = Game::new_test(3);
    g.give_bank(1, M5);
    g.give_bank(2, M5);
    debt_collector_on_p1(&mut g);
    assert_eq!((bank(&g, 0), bank(&g, 1), bank(&g, 2)), (5, 0, 5));
}
