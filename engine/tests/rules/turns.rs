use crate::common::*;

#[test]
fn new_game_deals_five_and_the_first_player_draws_two() {
    let g = Game::new(3, 1);
    let hands: Vec<_> = g.players.iter().map(|p| p.hand.len()).collect();
    assert_eq!(hands, vec![7, 5, 5]);
    assert_eq!(g.deck.len(), 106 - 17);
    assert_eq!((g.active, g.plays_left, g.phase), (0, 3, Phase::Main));
    g.check_invariants();
}

#[test]
fn three_plays_then_the_turn_ends_automatically() {
    let mut g = Game::new_test(2);
    for _ in 0..4 {
        g.give_hand(0, M1);
    }
    step(&mut g, Bank(M1));
    step(&mut g, Bank(M1));
    assert_eq!(g.active, 0);
    step(&mut g, Bank(M1));
    assert_eq!(g.active, 1, "third play ends the turn");
    assert_eq!(g.players[0].hand.len(), 1);
    assert_eq!(g.plays_left, 3);
}

#[test]
fn ending_the_turn_early_is_allowed() {
    let mut g = Game::new_test(2);
    g.give_hand(0, M1);
    g.give_hand(1, M1);
    step(&mut g, EndTurn);
    assert_eq!(g.active, 1);
    assert_eq!(g.players[0].hand.len(), 1);
}

#[test]
fn an_empty_hand_draws_five_otherwise_two() {
    let mut g = Game::new_test(2);
    g.give_hand(1, M1);
    step(&mut g, EndTurn);
    assert_eq!(g.players[1].hand.len(), 3);
    step(&mut g, EndTurn);
    assert_eq!(g.players[0].hand.len(), 5);
}

#[test]
fn pass_go_draws_two_and_uses_a_play() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::PassGo));
    step(&mut g, PlayAction(ActionType::PassGo));
    assert_eq!(g.players[0].hand.len(), 2);
    assert_eq!(g.plays_left, 2);
    assert_eq!(g.discard.len(), 1);
}

#[test]
fn ending_over_seven_cards_means_discarding_first() {
    let mut g = Game::new_test(2);
    for _ in 0..6 {
        g.give_hand(0, M1);
    }
    for _ in 0..3 {
        g.give_hand(0, M2);
    }
    step(&mut g, EndTurn);
    assert_eq!(g.phase, Phase::Discard);
    assert_eq!(g.legal_actions(), vec![Discard(M1), Discard(M2)]);
    step(&mut g, Discard(M2));
    assert_eq!(g.phase, Phase::Discard, "still 8 cards");
    step(&mut g, Discard(M1));
    assert_eq!(g.active, 1);
    assert_eq!(g.players[0].hand.len(), 7);
    assert_eq!(g.discard.len(), 2);
}

#[test]
fn running_out_of_plays_still_requires_discarding() {
    let mut g = Game::new_test(2);
    for _ in 0..6 {
        g.give_hand(0, M1);
    }
    for _ in 0..5 {
        g.give_hand(0, M2);
    }
    for _ in 0..3 {
        step(&mut g, Bank(M1));
    }
    assert_eq!(g.phase, Phase::Discard, "8 cards left");
    assert_eq!(g.active, 0);
    step(&mut g, Discard(M2));
    assert_eq!(g.active, 1);
}

#[test]
fn the_turn_waits_for_a_third_play_rent_to_be_paid() {
    let mut g = Game::new_test(2);
    g.plays_left = 1;
    g.give_table(0, RED, Color::Red);
    g.give_hand(0, Kind::Rent(RentType::RedYellow));
    g.give_bank(1, M1);
    g.give_bank(1, M5);
    step(&mut g, PlayRent { color: Color::Red, wild: false });
    assert_eq!(g.phase, Phase::Pay { owed: 2 });
    assert_eq!(g.active, 0);
    step(&mut g, PayBank(M5));
    assert_eq!(g.active, 1);
    assert_eq!(bank(&g, 0), 5);
}

#[test]
fn an_empty_deck_reshuffles_the_discard_pile() {
    let mut g = Game::new_test(2);
    g.give_hand(1, M1);
    g.discard = std::mem::take(&mut g.deck);
    step(&mut g, EndTurn);
    assert_eq!(g.players[1].hand.len(), 3);
    assert!(g.discard.is_empty());
    assert_eq!(g.deck.len(), 106 - 3);
}

#[test]
fn with_no_cards_left_to_draw_you_draw_what_there_is() {
    let mut g = Game::new_test(3);
    g.give_hand(1, M1);
    let rest: Vec<_> = g.deck.drain(..).collect();
    g.players[2].hand.extend(rest); // deck and discard both empty
    step(&mut g, EndTurn);
    assert_eq!(g.active, 1);
    assert_eq!(g.players[1].hand.len(), 1, "nothing to draw");
}

#[test]
fn the_game_is_a_draw_at_the_turn_limit() {
    let mut g = Game::new_test(2);
    g.max_turns = 2;
    g.give_hand(0, M1);
    g.give_hand(1, M1);
    step(&mut g, EndTurn);
    assert!(!g.is_over());
    step(&mut g, EndTurn);
    assert!(g.is_over());
    assert_eq!(g.winner, None);
    assert_eq!(g.returns(), vec![0.0, 0.0]);
}

#[test]
fn illegal_and_out_of_range_actions_are_rejected_without_changing_anything() {
    let mut g = Game::new_test(2);
    g.give_hand(0, RED);
    let before = format!("{g:?}");
    assert_eq!(g.step(Bank(RED)), Err(StepError::Illegal(Bank(RED))));
    assert_eq!(g.step_index(monopd_engine::NUM_ACTIONS), Err(StepError::OutOfRange(monopd_engine::NUM_ACTIONS)));
    assert_eq!(format!("{g:?}"), before);
}

#[test]
fn step_index_applies_the_action_with_that_index() {
    let mut g = Game::new_test(2);
    g.give_hand(0, M1);
    g.step_index(Bank(M1).index()).unwrap();
    assert_eq!(bank(&g, 0), 1);
}
