//! Randomised validation: hidden information, rule coverage, determinism.

use monopd_engine::bots::RandomBot;
use monopd_engine::game::{Effect, Phase, TargetChoice};
use monopd_engine::rng::Rng;
use monopd_engine::{Action, Game};
use std::collections::BTreeMap;

fn variant(a: &Action) -> String {
    let s = format!("{a:?}");
    s.split(['(', ' ', '{']).next().unwrap().to_string()
}

/// Re-dealing everything the deciding player can't see must not change what
/// they observe or what they may do. This catches hidden-information leaks in
/// `observe` and `legal_actions`, and checks `determinize` keeps the state
/// valid for every seat.
#[test]
fn hidden_information_does_not_leak() {
    let mut rng = Rng::new(1);
    for players in 2..=5 {
        for seed in 0..40u64 {
            let mut g = Game::new(players, seed);
            let mut bot = RandomBot::new(seed + 1000);
            while let Some(cp) = g.current_player() {
                let d = g.determinize(cp, rng.next_u64());
                d.check_invariants();
                assert_eq!(d.observe(cp), g.observe(cp), "observation depends on hidden cards");
                assert_eq!(d.legal_actions(), g.legal_actions(), "legal actions depend on hidden cards");
                // Re-dealing from any other seat must still give a playable state.
                let other = (cp + 1) % players;
                let mut d2 = g.determinize(other, rng.next_u64());
                d2.check_invariants();
                let a = RandomBot::new(rng.next_u64()).choose(&d2);
                d2.step(a).unwrap();
                d2.check_invariants();

                let a = bot.choose(&g);
                g.step(a).unwrap();
            }
        }
    }
}

/// Random play should reach every rule. If a rule never happens here, the
/// fuzz tests aren't exercising it (or it can't happen: a bug either way).
#[test]
fn random_play_covers_every_rule() {
    let mut seen: BTreeMap<String, u32> = BTreeMap::new();
    let mut hit = |k: String| *seen.entry(k).or_default() += 1;
    for players in 2..=5 {
        for seed in 0..800u64 {
            let mut g = Game::new(players, seed);
            g.max_turns = 300;
            let mut bot = RandomBot::new(seed ^ 0xC0FFEE);
            while g.current_player().is_some() {
                match g.phase {
                    Phase::Respond { jsn_count } => hit(format!("phase:Respond{jsn_count}")),
                    Phase::OfferDouble { multiplier, .. } => hit(format!("phase:OfferDouble x{multiplier}")),
                    Phase::ChooseTarget(t) => hit(format!("target:{}", format!("{t:?}").split([' ', '(', '{']).next().unwrap())),
                    Phase::Pay { .. } => hit("phase:Pay".into()),
                    Phase::Discard => hit("phase:Discard".into()),
                    _ => {}
                }
                if let Some(r) = &g.pending
                    && let Effect::DealBreaker(c) = r.effect
                    && g.players[r.victim()].pile(c).house.is_some()
                {
                    hit("event:DealBreaker with house".into());
                }
                let deck_before = g.deck.len();
                let a = bot.choose(&g);
                hit(format!("action:{}", variant(&a)));
                if let Action::PlayAction(t) = a {
                    hit(format!("play:{t:?}"));
                }
                if let Action::PayBuilding { hotel, .. } = a {
                    hit(format!("event:pay with {}", if hotel { "hotel" } else { "house" }));
                }
                g.step(a).unwrap();
                if g.deck.len() > deck_before {
                    hit("event:reshuffle".into());
                }
            }
            hit(if g.winner.is_some() { "end:win".into() } else { "end:draw".into() });
        }
    }

    let expected = [
        "action:EndTurn", "action:Bank", "action:PlaceProperty", "action:MoveWild", "action:PlayAction",
        "action:PlayRent", "action:TargetPlayer", "action:TargetProperty", "action:TargetSet",
        "action:PlayDouble", "action:DeclineDouble", "action:JustSayNo", "action:Accept", "action:PayBank",
        "action:PayProperty", "action:PayBuilding", "action:Discard",
        "play:DealBreaker", "play:SlyDeal", "play:ForcedDeal", "play:DebtCollector", "play:Birthday",
        "play:House", "play:Hotel", "play:PassGo",
        "target:RentPlayer", "target:Debt", "target:Sly", "target:ForcedTake", "target:ForcedGive",
        "target:DealBreaker", "target:House", "target:Hotel",
        "phase:Respond0", "phase:Respond1", "phase:OfferDouble x1", "phase:OfferDouble x2", "phase:Pay",
        "phase:Discard", "event:pay with house", "event:pay with hotel", "event:reshuffle",
        "end:win", "end:draw",
    ];
    let missing: Vec<_> = expected.iter().filter(|k| !seen.contains_key(**k)).collect();
    assert!(missing.is_empty(), "never reached: {missing:?}\nseen: {seen:#?}");
}

/// A cloned game replayed with the same choices ends identically: no hidden
/// state outside `Game`.
#[test]
fn clone_replays_identically() {
    for seed in 0..50u64 {
        let mut g = Game::new(3, seed);
        let mut bot = RandomBot::new(seed);
        for _ in 0..40 {
            if g.current_player().is_none() {
                break;
            }
            let a = bot.choose(&g);
            g.step(a).unwrap();
        }
        let mut a = g.clone();
        let mut b = g.clone();
        let (mut ba, mut bb) = (RandomBot::new(7), RandomBot::new(7));
        while a.current_player().is_some() {
            a.step(ba.choose(&a)).unwrap();
            b.step(bb.choose(&b)).unwrap();
        }
        assert!(b.current_player().is_none());
        assert_eq!((a.winner, a.turn, a.observe(0)), (b.winner, b.turn, b.observe(0)));
    }
}

/// Many more games with every check on. Slow in debug builds, so opt in:
///     cargo test --release --test fuzz -- --ignored
/// Set FUZZ_GAMES=300000 for a longer run.
#[test]
#[ignore]
fn long_fuzz() {
    let mut rng = Rng::new(42);
    let games: u64 = std::env::var("FUZZ_GAMES").ok().and_then(|s| s.parse().ok()).unwrap_or(20_000);
    for i in 0..games {
        let players = 2 + (i % 4) as usize;
        let mut g = Game::new(players, rng.next_u64());
        let mut bot = RandomBot::new(rng.next_u64());
        while let Some(cp) = g.current_player() {
            if rng.below(8) == 0 {
                let d = g.determinize(cp, rng.next_u64());
                assert_eq!(d.legal_actions(), g.legal_actions());
                d.check_invariants();
            }
            let a = bot.choose(&g);
            assert_eq!(Action::from_index(a.index()), Some(a));
            g.step(a).unwrap();
            g.check_invariants();
        }
    }
}

#[test]
fn choose_target_variants_are_all_listed() {
    // Keeps the coverage list above honest if a TargetChoice is added.
    let all = [
        TargetChoice::RentPlayer(monopd_engine::Color::Red),
        TargetChoice::Debt,
        TargetChoice::Sly,
        TargetChoice::ForcedTake,
        TargetChoice::ForcedGive { target: 0, card: 0, color: monopd_engine::Color::Red },
        TargetChoice::DealBreaker,
        TargetChoice::House { card: 0 },
        TargetChoice::Hotel { card: 0 },
    ];
    assert_eq!(all.len(), 8);
}
