use monopd_engine::action::Action::*;
use monopd_engine::bots::RandomBot;
use monopd_engine::cards::*;
use monopd_engine::game::{Phase, TargetChoice};
use monopd_engine::{Action, Game, NUM_ACTIONS, OBS_SIZE};

const RED: Kind = Kind::Property(Color::Red);
const GREEN: Kind = Kind::Property(Color::Green);
const BROWN: Kind = Kind::Property(Color::Brown);
const DARKBLUE: Kind = Kind::Property(Color::DarkBlue);
const M1: Kind = Kind::Money(0);
const M2: Kind = Kind::Money(1);
const M3: Kind = Kind::Money(2);
const M5: Kind = Kind::Money(4);
const M10: Kind = Kind::Money(5);
const JSN: Kind = Kind::Action(ActionType::JustSayNo);
const DOUBLE: Kind = Kind::Action(ActionType::DoubleRent);

fn act(t: ActionType) -> Kind {
    Kind::Action(t)
}

fn step(g: &mut Game, a: Action) {
    g.step(a).unwrap_or_else(|e| panic!("{e}; legal: {:?}", g.legal_actions()));
    g.check_invariants();
}

fn bank_total(g: &Game, p: usize) -> u32 {
    g.players[p].bank_value()
}

// ---------------------------------------------------------------------------
// Deck and encoding
// ---------------------------------------------------------------------------

#[test]
fn deck_composition() {
    assert_eq!(DECK.len(), 106);
    let count = |f: fn(&Kind) -> bool| DECK.iter().filter(|k| f(k)).count();
    assert_eq!(count(|k| matches!(k, Kind::Property(_))), 28);
    assert_eq!(count(|k| matches!(k, Kind::Wild(_))), 11);
    assert_eq!(count(|k| matches!(k, Kind::Action(_))), 34);
    assert_eq!(count(|k| matches!(k, Kind::Rent(_))), 13);
    assert_eq!(count(|k| matches!(k, Kind::Money(_))), 20);
    let money: u32 = DECK.iter().filter(|k| matches!(k, Kind::Money(_))).map(|k| k.value() as u32).sum();
    assert_eq!(money, 57);
    assert_eq!(KIND_COUNTS[act(ActionType::ForcedDeal).index()], 3);
    assert_eq!(KIND_COUNTS[act(ActionType::Hotel).index()], 2);
    assert_eq!(KIND_COUNTS[act(ActionType::PassGo).index()], 10);
    assert_eq!(Kind::Wild(WildType::UtilityRailroad).value(), 2);
    assert_eq!(Kind::Wild(WildType::Rainbow).value(), 0);
}

#[test]
fn kind_index_roundtrip() {
    for i in 0..NUM_KINDS {
        assert_eq!(Kind::from_index(i).index(), i);
    }
}

#[test]
fn action_index_roundtrip() {
    for i in 0..NUM_ACTIONS {
        let a = Action::from_index(i).unwrap();
        assert_eq!(a.index(), i, "{a:?}");
    }
    assert!(Action::from_index(NUM_ACTIONS).is_none());
}

#[test]
fn same_kind_cards_give_same_action() {
    // Two different Red/Yellow rent cards produce exactly one rent action.
    let mut g = Game::new_test(2);
    g.give_table(0, RED, Color::Red);
    g.give_hand(0, Kind::Rent(RentType::RedYellow));
    g.give_hand(0, Kind::Rent(RentType::RedYellow));
    let rents: Vec<_> = g.legal_actions().into_iter().filter(|a| matches!(a, PlayRent { .. })).collect();
    assert_eq!(rents, vec![PlayRent { color: Color::Red, wild: false }]);
}

// ---------------------------------------------------------------------------
// Turn structure
// ---------------------------------------------------------------------------

#[test]
fn new_game_deals_and_draws() {
    let g = Game::new(3, 1);
    assert_eq!(g.players[0].hand.len(), 7);
    assert_eq!(g.players[1].hand.len(), 5);
    assert_eq!(g.players[2].hand.len(), 5);
    assert_eq!(g.deck.len(), 106 - 17);
    g.check_invariants();
}

#[test]
fn three_plays_then_turn_ends_automatically() {
    let mut g = Game::new_test(2);
    for _ in 0..4 {
        g.give_hand(0, M1);
    }
    for _ in 0..2 {
        step(&mut g, Bank(M1));
    }
    assert_eq!(g.active, 0);
    step(&mut g, Bank(M1));
    assert_eq!(g.active, 1, "third play ends the turn");
    assert_eq!(g.players[0].hand.len(), 1);
    assert_eq!(g.plays_left, 3);
}

#[test]
fn auto_end_still_requires_discarding_to_seven() {
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
    assert_eq!(g.phase, Phase::Discard, "8 cards left: must discard before the turn passes");
    assert_eq!(g.active, 0);
    step(&mut g, Discard(M2));
    assert_eq!(g.active, 1);
}

#[test]
fn turn_ends_once_a_third_play_rent_is_paid() {
    let mut g = Game::new_test(2);
    g.plays_left = 1;
    g.give_table(0, RED, Color::Red);
    g.give_hand(0, Kind::Rent(RentType::RedYellow));
    g.give_bank(1, M1);
    g.give_bank(1, M5);
    step(&mut g, PlayRent { color: Color::Red, wild: false });
    assert_eq!(g.phase, Phase::Pay { owed: 2 });
    assert_eq!(g.active, 0, "turn waits for the payment");
    step(&mut g, PayBank(M5));
    assert_eq!(g.active, 1);
    assert_eq!(bank_total(&g, 0), 5);
}

#[test]
fn empty_hand_draws_five_otherwise_two() {
    let mut g = Game::new_test(2);
    g.give_hand(1, M1);
    step(&mut g, EndTurn);
    assert_eq!(g.active, 1);
    assert_eq!(g.players[1].hand.len(), 3);
    step(&mut g, EndTurn);
    assert_eq!(g.active, 0);
    assert_eq!(g.players[0].hand.len(), 5);
}

#[test]
fn discard_down_to_seven() {
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
    assert_eq!(g.phase, Phase::Discard);
    step(&mut g, Discard(M1));
    assert_eq!(g.active, 1);
    assert_eq!(g.players[0].hand.len(), 7);
    assert_eq!(g.discard.len(), 2);
}

#[test]
fn deck_reshuffles_discard_when_empty() {
    let mut g = Game::new_test(2);
    g.give_hand(1, M1);
    let all = std::mem::take(&mut g.deck);
    g.discard = all;
    step(&mut g, EndTurn);
    assert_eq!(g.players[1].hand.len(), 3);
    assert_eq!(g.deck.len() + g.discard.len(), 106 - 3);
    assert!(g.discard.is_empty());
}

// ---------------------------------------------------------------------------
// Properties and wilds
// ---------------------------------------------------------------------------

#[test]
fn wild_can_be_placed_as_either_colour_and_moving_it_uses_a_play() {
    let mut g = Game::new_test(2);
    let ry = Kind::Wild(WildType::RedYellow);
    g.give_hand(0, ry);
    let legal = g.legal_actions();
    assert!(legal.contains(&PlaceProperty { kind: ry, color: Color::Red }));
    assert!(legal.contains(&PlaceProperty { kind: ry, color: Color::Yellow }));
    assert!(!legal.contains(&PlaceProperty { kind: ry, color: Color::Green }));
    step(&mut g, PlaceProperty { kind: ry, color: Color::Red });
    assert_eq!(g.plays_left, 2);
    step(&mut g, MoveWild { kind: ry, from: Color::Red, to: Color::Yellow });
    assert_eq!(g.plays_left, 1, "moving a wild uses a play");
    assert_eq!(g.players[0].pile(Color::Yellow).len(), 1);
    step(&mut g, MoveWild { kind: ry, from: Color::Yellow, to: Color::Red });
    assert_eq!(g.active, 1, "third play (a wild move) ends the turn");
    assert_eq!(g.players[0].pile(Color::Red).len(), 1);
}

#[test]
fn properties_cannot_be_banked() {
    let mut g = Game::new_test(2);
    g.give_hand(0, RED);
    assert!(!g.legal_actions().contains(&Bank(RED)));
}

#[test]
fn action_cards_can_be_banked() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::DealBreaker));
    step(&mut g, Bank(act(ActionType::DealBreaker)));
    assert_eq!(bank_total(&g, 0), 5);
}

#[test]
fn win_needs_three_complete_sets_of_different_colours() {
    let mut g = Game::new_test(2);
    g.give_table(0, BROWN, Color::Brown);
    g.give_table(0, BROWN, Color::Brown);
    g.give_table(0, DARKBLUE, Color::DarkBlue);
    g.give_table(0, DARKBLUE, Color::DarkBlue);
    // An overflowing Brown pile doesn't count twice.
    g.give_table(0, Kind::Wild(WildType::LightBlueBrown), Color::Brown);
    g.give_table(0, Kind::Wild(WildType::Rainbow), Color::Brown);
    g.give_hand(0, Kind::Property(Color::Utility));
    g.give_hand(0, Kind::Property(Color::Utility));
    step(&mut g, PlaceProperty { kind: Kind::Property(Color::Utility), color: Color::Utility });
    assert!(!g.is_over());
    step(&mut g, PlaceProperty { kind: Kind::Property(Color::Utility), color: Color::Utility });
    assert!(g.is_over());
    assert_eq!(g.winner, Some(0));
    assert_eq!(g.returns(), vec![1.0, -1.0]);
}

// ---------------------------------------------------------------------------
// Rent
// ---------------------------------------------------------------------------

#[test]
fn two_colour_rent_charges_every_opponent_and_payer_chooses() {
    let mut g = Game::new_test(3);
    g.give_table(0, RED, Color::Red);
    g.give_table(0, RED, Color::Red); // 2 red = $3M
    g.give_hand(0, Kind::Rent(RentType::RedYellow));
    g.give_bank(1, M3); // exactly enough: paid automatically
    g.give_bank(2, M1);
    g.give_bank(2, M5); // more than enough: P2 chooses
    step(&mut g, PlayRent { color: Color::Red, wild: false });
    assert_eq!(g.phase, Phase::Pay { owed: 3 });
    assert_eq!(g.current_player(), Some(2));
    assert_eq!(bank_total(&g, 1), 0);
    let legal = g.legal_actions();
    assert_eq!(legal.len(), 2);
    step(&mut g, PayBank(M5)); // no change given
    assert_eq!(g.phase, Phase::Main);
    assert_eq!(g.current_player(), Some(0));
    assert_eq!(bank_total(&g, 0), 8);
    assert_eq!(bank_total(&g, 2), 1);
}

#[test]
fn rent_includes_house_and_hotel() {
    let mut g = Game::new_test(2);
    g.give_table(0, DARKBLUE, Color::DarkBlue);
    g.give_table(0, DARKBLUE, Color::DarkBlue);
    g.give_building(0, Color::DarkBlue, false);
    g.give_building(0, Color::DarkBlue, true);
    assert_eq!(g.players[0].rent(Color::DarkBlue), 8 + 3 + 4);
}

#[test]
fn double_the_rent_stacks() {
    let mut g = Game::new_test(2);
    g.give_table(0, BROWN, Color::Brown); // rent $1M
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
fn double_not_offered_without_plays_left() {
    let mut g = Game::new_test(2);
    g.plays_left = 1;
    g.give_table(0, BROWN, Color::Brown);
    g.give_hand(0, Kind::Rent(RentType::LightBlueBrown));
    g.give_hand(0, DOUBLE);
    g.give_bank(1, M5);
    step(&mut g, PlayRent { color: Color::Brown, wild: false });
    assert_eq!(g.phase, Phase::Pay { owed: 1 });
}

#[test]
fn wild_rent_targets_one_player() {
    let mut g = Game::new_test(3);
    g.give_table(0, GREEN, Color::Green); // $2M
    g.give_hand(0, Kind::Rent(RentType::Wild));
    g.give_bank(1, M5);
    g.give_bank(2, M5);
    step(&mut g, PlayRent { color: Color::Green, wild: true });
    assert_eq!(g.phase, Phase::ChooseTarget(TargetChoice::RentPlayer(Color::Green)));
    assert_eq!(g.legal_actions(), vec![TargetPlayer(1), TargetPlayer(2)]);
    step(&mut g, TargetPlayer(2));
    assert_eq!(g.phase, Phase::Pay { owed: 2 });
    assert_eq!(g.current_player(), Some(2));
    step(&mut g, PayBank(M5));
    assert_eq!(bank_total(&g, 1), 5);
    assert_eq!(bank_total(&g, 0), 5);
}

#[test]
fn rent_needs_a_property_of_that_colour() {
    let mut g = Game::new_test(2);
    g.give_hand(0, Kind::Rent(RentType::RedYellow));
    assert!(!g.legal_actions().iter().any(|a| matches!(a, PlayRent { .. })));
}

// ---------------------------------------------------------------------------
// Payment
// ---------------------------------------------------------------------------

#[test]
fn short_payer_hands_over_everything() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::DebtCollector));
    g.give_bank(1, M1);
    g.give_table(1, GREEN, Color::Green);
    g.give_table(1, Kind::Wild(WildType::Rainbow), Color::Green); // worth $0, stays
    step(&mut g, PlayAction(ActionType::DebtCollector));
    step(&mut g, TargetPlayer(1));
    assert_eq!(g.phase, Phase::Main);
    assert_eq!(bank_total(&g, 0), 1);
    assert_eq!(g.players[0].pile(Color::Green).len(), 1, "property goes to receiver's table");
    assert_eq!(g.players[1].pile(Color::Green).len(), 1, "rainbow wild can't be paid");
}

#[test]
fn payer_with_nothing_pays_nothing() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::DebtCollector));
    step(&mut g, PlayAction(ActionType::DebtCollector));
    step(&mut g, TargetPlayer(1));
    assert_eq!(g.phase, Phase::Main);
}

#[test]
fn paying_with_property_from_complete_set_moves_buildings_to_bank() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::DebtCollector));
    g.give_table(1, BROWN, Color::Brown);
    g.give_table(1, BROWN, Color::Brown);
    g.give_building(1, Color::Brown, false);
    g.give_bank(1, M10);
    step(&mut g, PlayAction(ActionType::DebtCollector));
    step(&mut g, TargetPlayer(1));
    let legal = g.legal_actions();
    assert!(legal.contains(&PayBuilding { color: Color::Brown, hotel: false }));
    assert!(legal.contains(&PayProperty { kind: BROWN, color: Color::Brown }));
    step(&mut g, PayProperty { kind: BROWN, color: Color::Brown }); // $1M paid, $4M left
    assert_eq!(g.players[1].pile(Color::Brown).house, None);
    assert_eq!(bank_total(&g, 1), 10 + 3, "house moved to payer's bank");
    assert_eq!(g.phase, Phase::Pay { owed: 4 });
    step(&mut g, PayBank(act(ActionType::House)));
    assert_eq!(g.phase, Phase::Pay { owed: 1 });
    step(&mut g, PayProperty { kind: BROWN, color: Color::Brown });
    assert_eq!(g.phase, Phase::Main);
    assert_eq!(g.players[0].pile(Color::Brown).len(), 2);
    assert_eq!(bank_total(&g, 1), 10);
}

#[test]
fn hotel_must_be_paid_before_house() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::DebtCollector));
    g.give_table(1, BROWN, Color::Brown);
    g.give_table(1, BROWN, Color::Brown);
    g.give_building(1, Color::Brown, false);
    g.give_building(1, Color::Brown, true);
    g.give_bank(1, M10);
    step(&mut g, PlayAction(ActionType::DebtCollector));
    step(&mut g, TargetPlayer(1));
    let legal = g.legal_actions();
    assert!(legal.contains(&PayBuilding { color: Color::Brown, hotel: true }));
    assert!(!legal.contains(&PayBuilding { color: Color::Brown, hotel: false }));
}

#[test]
fn birthday_collects_from_everyone() {
    let mut g = Game::new_test(4);
    g.give_hand(0, act(ActionType::Birthday));
    for p in 1..4 {
        g.give_bank(p, M2);
    }
    step(&mut g, PlayAction(ActionType::Birthday));
    assert_eq!(bank_total(&g, 0), 6);
}

#[test]
fn received_payment_can_win_off_turn() {
    // P1 charges P0 rent; P0 pays with a property that completes P1's third set.
    let mut g = Game::new_test(2);
    g.active = 1;
    for c in [Color::Brown, Color::DarkBlue] {
        for _ in 0..2 {
            g.give_table(1, Kind::Property(c), c);
        }
    }
    g.give_table(1, Kind::Property(Color::Utility), Color::Utility);
    g.give_hand(1, Kind::Rent(RentType::LightBlueBrown));
    g.give_table(0, Kind::Property(Color::Utility), Color::Utility);
    g.give_table(0, Kind::Property(Color::Pink), Color::Pink);
    g.give_table(0, Kind::Property(Color::Pink), Color::Pink);
    step(&mut g, PlayRent { color: Color::Brown, wild: false }); // $2M
    assert_eq!(g.current_player(), Some(0));
    step(&mut g, PayProperty { kind: Kind::Property(Color::Utility), color: Color::Utility });
    assert_eq!(g.winner, Some(1));
}

// ---------------------------------------------------------------------------
// Steals
// ---------------------------------------------------------------------------

#[test]
fn sly_deal_cannot_take_from_complete_set() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::SlyDeal));
    for _ in 0..3 {
        g.give_table(1, RED, Color::Red);
    }
    assert!(!g.legal_actions().contains(&PlayAction(ActionType::SlyDeal)));
    g.give_table(1, GREEN, Color::Green);
    step(&mut g, PlayAction(ActionType::SlyDeal));
    assert_eq!(g.legal_actions(), vec![TargetProperty { rel: 1, kind: GREEN, color: Color::Green }]);
    step(&mut g, TargetProperty { rel: 1, kind: GREEN, color: Color::Green });
    assert_eq!(g.players[0].pile(Color::Green).len(), 1);
    assert!(g.players[1].pile(Color::Green).is_empty());
}

#[test]
fn forced_deal_swaps() {
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
fn forced_deal_needs_something_to_give() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::ForcedDeal));
    g.give_table(1, GREEN, Color::Green);
    assert!(!g.legal_actions().contains(&PlayAction(ActionType::ForcedDeal)));
}

#[test]
fn deal_breaker_takes_set_with_buildings() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::DealBreaker));
    g.give_table(1, DARKBLUE, Color::DarkBlue);
    g.give_table(1, DARKBLUE, Color::DarkBlue);
    g.give_building(1, Color::DarkBlue, false);
    step(&mut g, PlayAction(ActionType::DealBreaker));
    step(&mut g, TargetSet { rel: 1, color: Color::DarkBlue });
    assert!(g.players[0].is_complete(Color::DarkBlue));
    assert!(g.players[0].pile(Color::DarkBlue).house.is_some());
    assert!(g.players[1].pile(Color::DarkBlue).is_empty());
}

// ---------------------------------------------------------------------------
// Just Say No
// ---------------------------------------------------------------------------

#[test]
fn just_say_no_cancels() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::DebtCollector));
    g.give_hand(1, JSN);
    g.give_bank(1, M5);
    step(&mut g, PlayAction(ActionType::DebtCollector));
    step(&mut g, TargetPlayer(1));
    assert_eq!(g.phase, Phase::Respond { jsn_count: 0 });
    assert_eq!(g.current_player(), Some(1));
    step(&mut g, JustSayNo);
    assert_eq!(g.phase, Phase::Main);
    assert_eq!(bank_total(&g, 1), 5);
}

#[test]
fn just_say_no_can_be_countered() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::DebtCollector));
    g.give_hand(0, JSN);
    g.give_hand(1, JSN);
    g.give_bank(1, M5);
    step(&mut g, PlayAction(ActionType::DebtCollector));
    step(&mut g, TargetPlayer(1));
    step(&mut g, JustSayNo);
    assert_eq!(g.phase, Phase::Respond { jsn_count: 1 });
    assert_eq!(g.current_player(), Some(0));
    step(&mut g, JustSayNo); // counter: debt stands again, P1 has no more JSN
    assert_eq!(bank_total(&g, 0), 5);
    assert_eq!(g.plays_left, 2, "JSN in response doesn't use a play");
}

#[test]
fn just_say_no_only_protects_the_player_who_plays_it() {
    let mut g = Game::new_test(3);
    g.give_hand(0, act(ActionType::Birthday));
    g.give_hand(1, JSN);
    g.give_bank(1, M2);
    g.give_bank(2, M2);
    step(&mut g, PlayAction(ActionType::Birthday));
    step(&mut g, JustSayNo);
    assert_eq!(bank_total(&g, 1), 2);
    assert_eq!(bank_total(&g, 2), 0);
    assert_eq!(bank_total(&g, 0), 2);
}

#[test]
fn holding_jsn_but_accepting_still_pays() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::DebtCollector));
    g.give_hand(1, JSN);
    g.give_bank(1, M5);
    step(&mut g, PlayAction(ActionType::DebtCollector));
    step(&mut g, TargetPlayer(1));
    step(&mut g, Accept);
    assert_eq!(bank_total(&g, 0), 5);
    assert!(g.players[1].holds(JSN));
}

// ---------------------------------------------------------------------------
// Buildings
// ---------------------------------------------------------------------------

#[test]
fn house_and_hotel_rules() {
    let mut g = Game::new_test(2);
    g.give_hand(0, act(ActionType::House));
    g.give_hand(0, act(ActionType::Hotel));
    for _ in 0..4 {
        g.give_table(0, Kind::Property(Color::Railroad), Color::Railroad);
    }
    g.give_table(0, RED, Color::Red);
    g.give_table(0, RED, Color::Red);
    let legal = g.legal_actions();
    assert!(!legal.contains(&PlayAction(ActionType::House)), "no houses on railroads");
    assert!(!legal.contains(&PlayAction(ActionType::Hotel)));
    g.give_table(0, RED, Color::Red);
    assert!(!g.legal_actions().contains(&PlayAction(ActionType::Hotel)), "hotel needs a house");
    step(&mut g, PlayAction(ActionType::House));
    assert_eq!(g.legal_actions(), vec![TargetSet { rel: 0, color: Color::Red }]);
    step(&mut g, TargetSet { rel: 0, color: Color::Red });
    step(&mut g, PlayAction(ActionType::Hotel));
    step(&mut g, TargetSet { rel: 0, color: Color::Red });
    assert_eq!(g.players[0].rent(Color::Red), 6 + 3 + 4);
}

// ---------------------------------------------------------------------------
// Whole games
// ---------------------------------------------------------------------------

#[test]
fn random_games_keep_invariants() {
    let mut finished = 0;
    for players in 2..=5 {
        for seed in 0..150u64 {
            let mut g = Game::new(players, seed);
            let mut bot = RandomBot::new(seed * 31 + players as u64);
            let mut steps = 0;
            while let Some(p) = g.current_player() {
                let a = bot.choose(&g);
                assert_eq!(Action::from_index(a.index()), Some(a));
                let obs = g.observe(p);
                assert_eq!(obs.len(), OBS_SIZE);
                assert!(obs.iter().all(|x| x.is_finite()));
                g.step(a).unwrap();
                g.check_invariants();
                steps += 1;
                assert!(steps < 200_000, "game did not terminate");
            }
            if g.winner.is_some() {
                finished += 1;
            }
        }
    }
    assert!(finished > 0, "random play should sometimes produce a winner");
}

#[test]
fn same_seed_same_game() {
    let play = |seed| {
        let mut g = Game::new(4, seed);
        let mut bot = RandomBot::new(99);
        let mut trace = Vec::new();
        while g.current_player().is_some() {
            let a = bot.choose(&g);
            trace.push(a.index());
            g.step(a).unwrap();
        }
        (trace, g.winner, g.turn)
    };
    assert_eq!(play(5), play(5));
    assert_ne!(play(5).0, play(6).0);
}

#[test]
fn determinize_keeps_what_the_player_can_see() {
    let g = Game::new(3, 11);
    let d = g.determinize(0, 1234);
    d.check_invariants();
    assert_eq!(d.players[0].hand, g.players[0].hand);
    for p in 0..3 {
        assert_eq!(d.players[p].hand.len(), g.players[p].hand.len());
        assert_eq!(d.players[p].table, g.players[p].table);
        assert_eq!(d.players[p].bank, g.players[p].bank);
    }
    assert_eq!(d.observe(0), g.observe(0));
}
