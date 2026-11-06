# Learning notes

What we've worked through so far, in the order the ideas build on each other. For the overall plan, see [`../PLAN.md`](../PLAN.md). For what to do next, see [`NEXT_STEPS.md`](NEXT_STEPS.md).

## The model

### The big picture

The model takes the game state and chooses a move. It gets two inputs: the state, and the menu of legal moves the engine provides. Its output is a choice from that menu.

```python
h      = state_net(obs)       # [~1500]  -> [256]      once per decision
E      = action_net(actions)  # [K, F]   -> [K, 256]   same weights for every row
scores = E @ h                # [K]      one score per legal action
probs  = softmax(scores)      # [K]      across the whole menu
```

A separate value head reads `h` and estimates the chance of winning. It helps training and doesn't choose moves.

### Questions that came up

**How can "action 1" work when it means different things in different states?**
There are no numbered input slots. Each option arrives as its own description of what it is and does. "Charge Red rent" gets the same description wherever it appears on the menu, and shuffling the menu doesn't change any option's score.

**Aren't inputs just inputs, all fed into one layer?**
At the lowest level, yes. The difference is the wiring. If you glued everything into one long vector for a plain layer, the weights for "slot 1" would have to handle every possible action, and a changing menu size wouldn't fit. Instead, the same small network runs on every option. This weight sharing is the same idea as a convolution reusing one filter, or a transformer reusing weights for every word.

**Does the model run once per action?**
No. It's one forward pass. The K actions are stacked into one array and go through shared weights row by row, in a single call. They're compared at the softmax, so an option's probability depends on what else is on the menu.

**Is the score the chance of winning?**
Not in the plan we chose. There are two standard versions:

| | Score means | Then | Fit for this game |
|---|---|---|---|
| Q-learning | Estimated chance of winning after this action | Take the best | Deterministic, so predictable and exploitable |
| Policy (PPO) | How much to prefer this action | Softmax, then sample | Can stay deliberately mixed, which suits hidden information |

**Is this an action embedding, like word embeddings in an LLM?**
Yes. An LLM turns its context into a hidden vector, dots it with each word's embedding, and applies a softmax to choose the next word. This model does the same over legal actions, so the menu acts as a filtered vocabulary. The difference is where embeddings come from:

- **Lookup table** (`nn.Embedding`): one learned vector per action index, as with LLM word embeddings. Related actions start unrelated. We'll start here because it's simplest.
- **Computed from features:** a small network builds the embedding from the action's description, so related actions start close together. This is the later upgrade.

**Why make related actions look related at all?**
With independent outputs, what the model learns about Red rent never transfers to Yellow rent, so rare actions stay poorly learned for a long time. With features, it can learn once that "everyone pays me about $5M" is good, whichever card does it. Features should stay factual (what happens) rather than strategic (whether it's good), so the agent discovers strategy itself.

**Is there attention in this model?**
Not in the first version, which is plain dense layers. The final `h · E` looks like attention but is a single step. Attention across players is the first upgrade to try, because it handles varying player counts and seat symmetry naturally. Add it when measurements show a need.

## PyTorch concepts

### Layers

- **`nn.Linear(in, out)`** computes `output = input @ Wᵀ + b`. It has `in × out` weights and `out` biases, because each output has its own bias. It works on whole batches, as long as the last dimension matches `in`.
- **Affine** means linear plus a shift. Strictly, `nn.Linear` is affine because of its bias. Stacked affine functions are still one affine function, so stacking without an activation adds no expressive power.
- **Activations** such as ReLU supply the non-linearity. Linear layers mix values, and activations bend the result. `nn.Sigmoid` and `nn.Softmax` turn scores into probabilities.
- **Other layers:** `nn.Embedding` is a lookup table, `nn.Conv2d` shares a filter across positions, attention mixes items based on the data, `nn.LayerNorm` stabilises training, and `nn.Dropout` fights overfitting.

Parameter count for `MLP()`, which goes 10 → 64 → 64 → 1:

| Layer | Weights | Biases | Total |
|---|---|---|---|
| `Linear(10, 64)` | 640 | 64 | 704 |
| `Linear(64, 64)` | 4,096 | 64 | 4,160 |
| `Linear(64, 1)` | 64 | 1 | 65 |
| **Total** | | | **4,929** |

Inputs and ReLU have no parameters.

### The forward and backward passes

- **Forward pass:** data flows from inputs to outputs. You define it in `forward`.
- **Backward pass:** `loss.backward()` walks the recorded graph in reverse to compute a gradient for each weight.
- **Update:** `optimizer.step()` nudges the weights to lower the loss.
- **Evaluating:** use a forward pass inside `torch.no_grad()`, which skips recording the graph.

**`model(x)` vs `model.forward(x)`:** always write `model(x)`. Both compute the same thing, but `model.forward(x)` skips hooks. Gradient tracking works either way, because autograd is built into tensor operations, not into the call. It depends on whether inputs require gradients and on the grad mode.

### Shapes

- **`squeeze(1)`** removes dimension 1 if its size is 1, so `[n, 1]` becomes `[n]`. `unsqueeze(1)` adds one.
- **Plain `squeeze()`** removes every size-1 dimension, so a one-row batch `[1, 1]` becomes `[]`. Prefer `squeeze(1)`.
- **A 4×4 tensor** has no size-1 dimensions, so `squeeze` does nothing. To change its shape, use `flatten` or `reshape`.
- **Watch `[n, 1]` against `[n]`.** Broadcasting silently stretches them to `[n, n]` and compares everything with everything, which gives a wrong answer with no error.

### Numbers and randomness

- **Float comparison:** small whole numbers are exact in floating point, so comparing raw counts is safe. Trouble comes from fractions and computed values: `1 - 0.3` isn't exactly `0.7`, so `int(90 * (1 - 0.3))` gives 62.
- **Global randomness:** `torch.manual_seed` resets the generator everything else uses. A helper should use its own generator: `torch.randperm(n, generator=torch.Generator().manual_seed(seed))`. Otherwise "different" random trials can silently become identical.

### Local minima

Large networks rarely get stuck in truly bad local minima, because a real trap needs every one of millions of directions to curve upward. Saddle points and plateaus are the more common obstacles. Mini-batch noise, momentum, Adam, learning rate schedules, good initialisation and normalisation all help with those.

In RL, the real version of getting stuck is settling on a mediocre strategy and never trying anything else. Sampling from the policy, an entropy bonus, and an opponent pool all counter that.

## Mistakes worth remembering

Real bugs from the toy, because they'll come up again:

| What happened | Lesson |
|---|---|
| `X.eq(sizes)` for a complete set | The rule was "at least", and the test data never exceeded a full set. Passing tests only cover what they test. |
| A seeded `randperm` whose result was thrown away | The next line used the global generator. Check that the value you prepared is the one you actually use. |
| `forward` with no `return` | Python returns `None` with no error. |
| Comparing logits directly with labels | A prediction needs a decision rule (logit > 0) before comparing. |
| Counting one bias per layer | It's one bias per output. |

The first, second and fourth of these got past the first version of the checks, which were then made stricter. A green check is good evidence, not proof.
