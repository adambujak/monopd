# monopd_engine

A fast Monopoly Deal engine in Rust, built for self-play RL. No dependencies.

```bash
cargo test                                         # rule tests + fuzzing (hidden info, coverage, invariants)
FUZZ_GAMES=300000 cargo test --release --test fuzz -- --ignored   # long fuzz, every check after every step
cargo run --release --bin simulate -- 20000 3      # random-vs-random benchmark
cargo run --release --bin play -- 2                # play vs a random bot
```

Speed: about 6M decisions/s and about 27k full 3-player games/s on one core.

## Interface

```rust
let mut g = Game::new(num_players, seed);
while let Some(p) = g.current_player() {   // who decides now (may not be the active player)
    let obs  = g.observe(p);               // Vec<f32>, len OBS_SIZE, only what p can see
    let mask = g.legal_mask();             // Vec<bool>, len NUM_ACTIONS
    let a    = policy(obs, mask);          // index into 0..NUM_ACTIONS
    g.step_index(a)?;
}
g.returns();                               // +1 winner, -1 others, 0 on draw
```

`g.clone()` copies the full state for tree search. `g.determinize(seat, seed)` re-deals everything `seat` can't see, for Information-Set MCTS.

## How actions are encoded

**Actions name meanings, not cards.** Each physical card has an id, but no action refers to one. Every card has a `Kind` (40 kinds in all), and cards of the same kind are interchangeable. So:

- "Charge rent on Red" is `PlayRent { color: Red, wild: false }`. You get the same action from either of the two Red/Yellow rent cards.
- `wild: true` is the wild rent card. It is kept separate because it really does something different (it charges one target, not everyone).
- Taking a property is `TargetProperty { rel, kind, color }`, e.g. "the Red/Yellow wild sitting in Red on the next player's table". Two Red properties in the same pile are the same target.

**Players are named relative to whoever is deciding.** `rel = 1` is the next player and `0` is yourself. A move looks the same from every seat, and observations are ordered the same way.

**Big moves are broken into small decisions.** For example, Forced Deal is `PlayAction(ForcedDeal)`, then `TargetProperty` (theirs), then `TargetProperty` (yours). Reactions get their own decision points with their own decider: Just Say No, Accept, PayBank/PayProperty/PayBuilding (one card at a time) and Discard. Breaking moves up keeps the action space small: one flat space of `NUM_ACTIONS` = 2290 indices, of which usually only 2–30 are legal.

### Why this generalizes

When two situations call for the same move, they produce the same index, so the network learns the move once. You can push this further when you build the policy network:

- **Factored heads.** Every action is a tuple: verb (bank, place, rent, target, pay…), card kind, colour and player. The logit for each flat index can be a sum of per-part embeddings, `logit(a) = h · (E_verb + E_kind + E_color + E_rel)`. Then "rent on Red" and "rent on Green" share the *rent* weights, and "pay with $5M" shares weights with "bank $5M" through the kind embedding.
- **Masking.** Set illegal logits to −∞ before the softmax. The network never has to learn the rules.
- **Skipping forced moves.** When only one action is legal, take it in the environment wrapper without asking the network.

## Rule decisions

These follow the official rules unless noted:

- Deck: 106 cards (28 properties, 11 wilds, 34 actions, 13 rents, 20 money).
- Draw 2 at the start of a turn, or 5 if your hand is empty. Up to 3 plays per turn. Discard down to 7 at the end of the turn. You can end your turn early, and it ends automatically once all 3 plays are used (after any rent or steal finishes resolving), since nothing else would be legal.
- Win: 3 complete sets of *different* colours. This is checked after every change, so you can win on someone else's turn, e.g. when a payment completes one of your sets.
- Payment: the payer chooses what to pay with. No change is given. If the payer has too little, everything of value is handed over automatically. Properties go to the receiver's table with the same colour, and the receiver can move wilds later. Rainbow wilds are worth $0 and can't be used to pay.
- Just Say No: can be chained (a JSN against a JSN). Playing one in response **doesn't use a play** (simplification).
- Double The Rent is offered right after you play rent. Each double uses a play, and stacking two gives ×4.
- Rent needs at least one property of that colour.
- A rainbow (multi-colour) wild can be laid down any time as any colour, but has no colour of its own: on its own it can't collect rent. A pile of only rainbows doesn't count as a complete set (the rules don't cover this case; this follows from "no colour on its own"), so it isn't protected from Sly Deal or Forced Deal either.
- **Each colour is one pile (simplification).** A pile with ≥ set-size cards counts as one complete set, and the whole pile is safe from Sly Deal and Forced Deal. Deal Breaker takes the whole pile and its buildings.
- Buildings need a complete set that isn't a railroad or utility, and a hotel needs a house. If a set stops being complete, its buildings go to the owner's bank. Buildings you receive as payment go to your bank. A hotel must be paid before the house under it.
- Moving a wild already on your table to another colour uses one of your 3 plays (house rule; officially it is free).
- A game ends in a draw after 500 turns.

## Layout

| file | contents |
|---|---|
| file | contents |
|---|---|
| `src/cards.rs` | colours, kinds, the deck, rent ladders, values |
| `src/action.rs` | `Action` and its flat index encoding |
| `src/game/mod.rs` | `Game`, phases, setup, `step` |
| `src/game/player.rs` | `Player` and `Pile`: sets, rent, buildings, payable value |
| `src/game/legal.rs` | which actions are legal |
| `src/game/apply.rs` | applying an action; turn flow (draw, end turn, discard) |
| `src/game/resolve.rs` | rent, steals, payments, Just Say No chains, win check |
| `src/game/search.rs` | `determinize` for search over hidden cards |
| `src/game/debug.rs` | test setup helpers, `check_invariants`, text view |
| `src/obs.rs` | observation vector |
| `src/bots.rs`, `src/rng.rs` | random bot, seedable RNG |
| `tests/rules/` | one scenario per rule, grouped by topic |
| `tests/games.rs` | whole random games: invariants, determinism |
| `tests/fuzz.rs` | hidden-information, coverage and long fuzz checks |

Each `src` module also has unit tests (`cargo test --lib`).
