use crate::common::*;

#[test]
fn properties_go_on_the_table_and_cannot_be_banked() {
    let mut g = Game::new_test(2);
    g.give_hand(0, RED);
    assert_eq!(g.legal_actions(), vec![EndTurn, PlaceProperty { kind: RED, color: Color::Red }]);
    step(&mut g, PlaceProperty { kind: RED, color: Color::Red });
    assert_eq!(g.players[0].pile(Color::Red).len(), 1);
    assert_eq!(g.plays_left, 2);
}

#[test]
fn money_action_and_rent_cards_can_be_banked() {
    let mut g = Game::new_test(2);
    g.give_hand(0, M3);
    g.give_hand(0, act(ActionType::DealBreaker));
    g.give_hand(0, Kind::Rent(RentType::Wild));
    step(&mut g, Bank(M3));
    step(&mut g, Bank(act(ActionType::DealBreaker)));
    assert_eq!(bank(&g, 0), 3 + 5);
    assert!(g.legal_actions().contains(&Bank(Kind::Rent(RentType::Wild))));
}

#[test]
fn identical_cards_give_one_action() {
    let mut g = Game::new_test(2);
    g.give_table(0, RED, Color::Red);
    g.give_hand(0, Kind::Rent(RentType::RedYellow));
    g.give_hand(0, Kind::Rent(RentType::RedYellow));
    g.give_hand(0, M1);
    g.give_hand(0, M1);
    assert_eq!(legal_of(&g, |a| matches!(a, PlayRent { .. })), vec![PlayRent { color: Color::Red, wild: false }]);
    assert_eq!(legal_of(&g, |a| matches!(a, Bank(_))).len(), 2, "one Bank for the rent kind, one for $1M");
}

#[test]
fn a_wild_can_be_placed_only_as_one_of_its_colours() {
    let mut g = Game::new_test(2);
    let ry = Kind::Wild(WildType::RedYellow);
    g.give_hand(0, ry);
    assert_eq!(
        legal_of(&g, |a| matches!(a, PlaceProperty { .. })),
        vec![PlaceProperty { kind: ry, color: Color::Red }, PlaceProperty { kind: ry, color: Color::Yellow }]
    );
}

#[test]
fn moving_a_wild_uses_a_play() {
    let mut g = Game::new_test(2);
    let ry = Kind::Wild(WildType::RedYellow);
    g.give_hand(0, ry);
    step(&mut g, PlaceProperty { kind: ry, color: Color::Red });
    step(&mut g, MoveWild { kind: ry, from: Color::Red, to: Color::Yellow });
    assert_eq!(g.plays_left, 1);
    assert_eq!(g.players[0].pile(Color::Yellow).len(), 1);
    step(&mut g, MoveWild { kind: ry, from: Color::Yellow, to: Color::Red });
    assert_eq!(g.active, 1, "third play (a move) ends the turn");
}

#[test]
fn moving_a_wild_out_of_a_built_set_sends_the_buildings_to_the_bank() {
    let mut g = Game::new_test(2);
    let bg = Kind::Wild(WildType::DarkBlueGreen);
    g.give_table(0, DARKBLUE, Color::DarkBlue);
    g.give_table(0, bg, Color::DarkBlue);
    g.give_building(0, Color::DarkBlue, false);
    step(&mut g, MoveWild { kind: bg, from: Color::DarkBlue, to: Color::Green });
    assert_eq!(g.players[0].pile(Color::DarkBlue).house, None);
    assert_eq!(bank(&g, 0), 3);
}

#[test]
fn extra_cards_of_a_colour_stay_in_the_same_set() {
    let mut g = Game::new_test(2);
    complete_set(&mut g, 0, Color::Brown);
    g.give_table(0, Kind::Wild(WildType::LightBlueBrown), Color::Brown);
    assert_eq!(g.players[0].complete_sets(), 1);
    assert_eq!(g.players[0].rent(Color::Brown), 2, "rent caps at a full set");
}
