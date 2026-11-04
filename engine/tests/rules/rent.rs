use crate::common::*;

#[test]
fn two_colour_rent_charges_every_opponent_and_the_payer_chooses() {
    let mut g = Game::new_test(3);
    g.give_table(0, RED, Color::Red);
    g.give_table(0, RED, Color::Red); // 2 Red = $3M
    g.give_hand(0, Kind::Rent(RentType::RedYellow));
    g.give_bank(1, M3); // exactly enough: paid automatically
    g.give_bank(2, M1);
    g.give_bank(2, M5); // more than enough: P2 chooses
    step(&mut g, PlayRent { color: Color::Red, wild: false });
    assert_eq!(g.phase, Phase::Pay { owed: 3 });
    assert_eq!(g.current_player(), Some(2));
    assert_eq!(bank(&g, 1), 0);
    assert_eq!(g.legal_actions(), vec![PayBank(M1), PayBank(M5)]);
    step(&mut g, PayBank(M5)); // no change given
    assert_eq!((g.phase, g.current_player()), (Phase::Main, Some(0)));
    assert_eq!((bank(&g, 0), bank(&g, 2)), (8, 1));
}

#[test]
fn a_rent_card_offers_only_its_colours_that_you_have() {
    let mut g = Game::new_test(2);
    g.give_hand(0, Kind::Rent(RentType::RedYellow));
    assert!(legal_of(&g, |a| matches!(a, PlayRent { .. })).is_empty(), "no Red or Yellow yet");
    g.give_table(0, GREEN, Color::Green);
    g.give_table(0, Kind::Property(Color::Yellow), Color::Yellow);
    assert_eq!(legal_of(&g, |a| matches!(a, PlayRent { .. })), vec![PlayRent { color: Color::Yellow, wild: false }]);
}

#[test]
fn rent_includes_house_and_hotel() {
    let mut g = Game::new_test(2);
    complete_set(&mut g, 0, Color::DarkBlue);
    g.give_building(0, Color::DarkBlue, false);
    g.give_building(0, Color::DarkBlue, true);
    assert_eq!(g.players[0].rent(Color::DarkBlue), 8 + 3 + 4);
}

#[test]
fn double_the_rent_stacks_and_uses_plays() {
    let mut g = Game::new_test(2);
    g.give_table(0, BROWN, Color::Brown); // $1M
    g.give_hand(0, Kind::Rent(RentType::LightBlueBrown));
    g.give_hand(0, DOUBLE);
    g.give_hand(0, DOUBLE);
    g.give_bank(1, M5);
    g.give_bank(1, M1);
    step(&mut g, PlayRent { color: Color::Brown, wild: false });
    assert!(matches!(g.phase, Phase::OfferDouble { multiplier: 1, .. }));
    step(&mut g, PlayDouble);
    assert!(matches!(g.phase, Phase::OfferDouble { multiplier: 2, .. }));
    step(&mut g, PlayDouble);
    assert_eq!(g.plays_left, 0);
    assert_eq!(g.phase, Phase::Pay { owed: 4 });
}

#[test]
fn no_double_without_a_play_left() {
    let mut g = Game::new_test(2);
    g.plays_left = 1;
    g.give_table(0, BROWN, Color::Brown);
    g.give_hand(0, Kind::Rent(RentType::LightBlueBrown));
    g.give_hand(0, DOUBLE);
    g.give_bank(1, M5);
    g.step(PlayRent { color: Color::Brown, wild: false }).unwrap();
    assert_eq!(g.phase, Phase::Respond { jsn_count: 0 }, "straight to the victim");
}

#[test]
fn wild_rent_targets_one_player_and_can_be_doubled() {
    let mut g = Game::new_test(3);
    g.give_table(0, GREEN, Color::Green); // $2M
    g.give_hand(0, Kind::Rent(RentType::Wild));
    g.give_hand(0, DOUBLE);
    g.give_bank(1, M5);
    g.give_bank(2, M10);
    step(&mut g, PlayRent { color: Color::Green, wild: true });
    assert_eq!(g.phase, Phase::ChooseTarget(TargetChoice::RentPlayer(Color::Green)));
    assert_eq!(g.legal_actions(), vec![TargetPlayer(1), TargetPlayer(2)]);
    step(&mut g, TargetPlayer(2));
    step(&mut g, PlayDouble);
    assert_eq!((g.phase, g.current_player()), (Phase::Pay { owed: 4 }, Some(2)));
    step(&mut g, PayBank(M10));
    assert_eq!((bank(&g, 0), bank(&g, 1)), (10, 5), "P1 untouched");
}
