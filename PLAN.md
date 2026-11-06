# Project plan and context

## What this is

Building an AI that plays Monopoly Deal, trained through self-play. There's little human game data to learn from, so the agent has to learn by playing, and the game has hidden hands and varying player counts.

There are two goals, and the second matters as much as the first:

1. **Get a strong agent.**
2. **Learn how to build it.** Adam writes the machine learning code himself. Claude writes the supporting pieces (engine, viewer, bindings, exercise scaffolding), and explains and reviews the rest without handing over solutions.

## Current status

| Piece | Status |
|---|---|
| Rust game engine (`engine/`) | Done: rules, action encoding, observations, tests, fuzzing |
| Browser viewer (`web/`) | Done: play against random bots or watch them |
| Python bindings (`bindings/`) | Done: `import monopd` |
| Python environment | Done: uv project (`uv sync`, `uv run`) |
| Learning step 1, PyTorch toy | **In progress:** parts 1–3 and `accuracy` are done. Next is `train`, then the part 5 experiments. |
| Learning steps 2–7 | Not started |

## Learning plan

Each step ends with something that works and can be measured. Details are in [`learn/README.md`](learn/README.md).

1. **PyTorch on a toy problem** ([`learn/01_pytorch_toy/`](learn/01_pytorch_toy/)): tensors, models, the training loop, and overfitting.
2. **Win predictor:** predict the winner of real games from `observe()`, using states from random games. Split train and test by whole game, because states from one game are nearly identical. This network later becomes the value head.
3. **Policy network, untrained:** the model below plays full games, at about a 50% win rate against a random bot.
4. **REINFORCE:** the policy learns against random bots and its win rate climbs. This is the big milestone.
5. **Actor-critic:** use the value head as a baseline for steadier learning.
6. **Self-play:** train against a pool of saved past versions, which avoids strategy cycles.
7. **PPO:** the long-term training algorithm.

Later upgrades, added only if measurements call for them:
- action features in place of the lookup table
- attention across players
- a tree-search (Information-Set MCTS) bot as a strong baseline

## Planned model

```
game state (~1,500 numbers) -> state network  -> h      (state summary, 256 numbers)
legal actions (K rows)      -> action network -> E      (K action embeddings, same weights per row)

scores = E @ h                    one score per legal action
probs  = softmax(scores)          sample one action
value  = value head(h)            win estimate, used during training
```

- **Why the action list is an input:** the model only produces scores for legal actions, so it can't pick an illegal one. Each action is described by what it is, not by its position in the list, so the menu's order and size don't matter.
- **Action embeddings:** these work like word embeddings in an LLM, and scoring works like choosing the next word from a vocabulary filtered to legal moves.
  - Step 3 starts with a lookup table (`nn.Embedding`).
  - Later, embeddings are computed from action features (verb, card, colour, payers, money collected, set completed), so related actions such as Red rent and Yellow rent start out close together.
- **Why a policy rather than always taking the best action:** hidden information rewards being unpredictable, and a policy can stay deliberately mixed.

## Engine decisions

The full rule list is in [`engine/README.md`](engine/README.md). These are the choices that shaped the design:

- **Actions name meanings, not card instances.** "Charge Red rent" is one action whichever physical card is used. Players are named relative to whoever is deciding.
- **Big moves are split into small decisions,** with reactions such as paying and Just Say No as their own decision points.
- **Who gets asked must not reveal hands.** Every targeted player is asked about Just Say No, and every renter with a play left is offered Double The Rent, whether or not they hold the card.
- **The agent sees full bank contents.** Deposits and payments are public, so this equals perfect memory. The viewer squares up other players' banks for humans, since people don't remember perfectly.

House rules and interpretations:

- Moving a wild costs a play.
- Just Say No in response doesn't use a play.
- A rainbow wild can be laid alone, but it collects no rent on its own, and rainbows alone never make a complete set.
- Each colour is one pile, including any extra cards beyond a full set.
- Turns end automatically when all 3 plays are used.

## Working agreement

- Adam writes the ML code. Claude gives hints, explanations and reviews rather than solutions.
- Each exercise has a skeleton with `TODO`s and a `check` command.
- Official card art isn't part of this repo. The viewer uses original card designs, and `web/cards/` (gitignored) can hold personal local images.
