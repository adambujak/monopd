# Step 3: policy network, untrained

A model that makes every decision in a full game: the two-column design from [`PLAN.md`](../../PLAN.md). Nothing is trained yet. This step is about getting the machinery right, so step 4 can focus on learning.

```bash
cd learn/03_policy
uv run python policy.py check                  # tells you which part to do next
uv run python policy.py play                   # once every part passes: 1000 games vs RandomBot, ~20 s
uv run python policy.py play --emb-std 0.01    # the same model with small initial embeddings
uv run python policy.py show                   # one game: what the model chose, and its top 3 options
```

`play` and `show` also take `--model-seed` (the model's initial weights) and `--seed` (the games). `play` takes `--games`.

---

## Part 1: the model

Write `PolicyNet`:

```
h      = state_net(obs)       obs [1474]  -> h [hidden]
E      = embedding(legal)     legal [K]   -> E [K, hidden]
scores = E @ h                            -> [K]
```

**Concepts:** `nn.Embedding` as a lookup table; one network scoring a variable-length list; why scores can't depend on list position.

<details><summary>Hints</summary>

- The state column is step 2's MLP with `hidden` outputs instead of 1. Leave the last layer without a ReLU: `h` can be negative.
- `nn.Embedding(num_actions, hidden)` holds one learned row per action index. Calling it with an int64 tensor of indices, shape `[K]`, returns those rows, shape `[K, hidden]`.
- `E @ h` with `E` `[K, hidden]` and `h` `[hidden]` gives `[K]`. Write out the shapes to convince yourself.
- Don't use `torch.no_grad()` in `forward`. Step 4 needs gradients through it.

</details>

**Check yourself:** how many parameters does `PolicyNet()` have, and what fraction of them is the embedding? Most actions are rarely legal. What does that mean for how often each embedding row gets trained?

---

## Part 2: choosing a move

Write `choose`: score the legal actions, sample one, and return its action index.

**Concepts:** `torch.distributions.Categorical`; sampling vs taking the best; position vs index.

<details><summary>Hints</summary>

- `game.legal_actions()` is a list of action indices. `torch.tensor(...)` of it is the `legal` input.
- `Categorical(logits=scores)` builds the distribution. It applies the softmax for you. `.sample()` returns a *position* in `legal`, as a 0-d tensor.
- The action to play is `legal[position]`. Returning the position instead is the classic bug, and the check looks for it.
- Observe from the seat that's deciding: `game.current_player()`.

</details>

---

## Part 3: one full game

Write `play_game`: the model in one seat, `RandomBot` in the other, until the game ends.

<details><summary>Hints</summary>

- `current_player()` decides who acts, and it isn't always the player whose turn it is. Your model also pays rent and answers Just Say No on the opponent's turn.
- Reward: `game.returns()[model_seat]`.

</details>

---

## Part 4: measuring

Write `evaluate`: win rate over many games, alternating seats.

<details><summary>Hints</summary>

- Alternate seats with `k % 2`: seat 0 moves first and may have an edge.
- `model.eval()` and `torch.no_grad()`: this is measurement, not training.

</details>

---

## Part 5: experiments and questions

An untrained model should be "random". Is it?

1. **Run `play`.** What win rate do you get against RandomBot? Is it within the ± of 50%? Run `play --model-seed 1`, `2`, `3`. What changes between seeds, and why would an untrained network play differently from a random bot? Use `show` to see what this one prefers.
2. **Run `play --emb-std 0.01`.** What happens to the win rate, the top probability and the entropy? Work out why from `scores = E @ h`: what does the scale of `E` do to the scores, and what does the softmax do with small scores?
3. **Why does this matter for step 4?** REINFORCE learns from the moves the policy actually tries. What would happen to learning if the untrained policy almost never chose some action?
4. **Forced moves.** `play` reports how many of the model's decisions had only one legal action. What can a policy learn from a decision with one option? (Step 4 will skip them.)

**Done when:** `play` runs 1000 games without errors, and with small initial embeddings it wins about 50% (within the printed ±).

Bonus:
- Make `--emb-std 0.01` your model's own default: initialise the embedding inside `PolicyNet.__init__` with `nn.init.normal_`.
- Time it: how many decisions per second? Where does the time go: the engine, `observe`, or the forward pass?
