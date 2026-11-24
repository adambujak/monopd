# Learnings

What came up while working through steps 1 to 4: the questions you asked, the mental models that needed correcting, what you learned, and the bugs worth remembering. [`NOTES.md`](NOTES.md) covers the earlier design discussion and PyTorch basics; this picks up from there.


---

## Mental models that changed

The most useful part of this file. Each one is a "before" that's easy to fall back into.

| You thought | What's actually true |
|---|---|
| A linear model fails at the toy because it can't count how many sets are needed to win. | The counting part is linear (`sum of complete flags - 2.5 > 0`). What a weighted sum can't do is the per-colour *jump* at the set size. Given `complete_mask` as input, a linear model is near perfect. |
| `tiny` overfit because of the huge number of epochs. | The root cause is capacity against data: about 69,000 parameters against 200 rows. Epochs only give it the time to memorise. |
| The win predictor has about 96,000 examples. | It has about **5,000 independent outcomes**: one per game, and in 2 players the seats' labels are just opposites. Rows from one game repeat the same fact. That's why a 444,000-parameter network overfits, and why more games, not more rows per game, is what adds information. |
| More data is the best fix for overfitting. | On its own it helped least (+1.6 points): the model still had capacity and more steps to memorise with. Data raises the ceiling; capacity limits (dropout, smaller nets, early stopping) make it generalise. Together they did best (+5.4). |
| Summing gradients over a batch loses information, so the update can't be trusted. | The batch gradient is exact: `G.T @ X` sums the per-row gradients in one multiply. What's lost is how much rows *disagreed*, which a single step can't use anyway. Averaging is the correct way to combine them for an average loss. |
| An embedding is a lower-dimensional representation of the *state*. | An embedding is a learned vector per item in a fixed set (tokens, actions, colours). The state's representation is `h`. Our action table plays the role of an LLM's *output* matrix (the LM head), not its input embedding. |
| Putting "decisions" in an embedding table is unusual. | It's the standard last layer of any classifier, seen from another angle: `nn.Linear(256, 2290)` has a `[2290, 256]` weight, one row per action, and `scores = W @ h`. Two-tower recommenders, DRRN and AlphaStar's pointer network all score candidates this way. |
| `E` decides, so it's "the decision model", not a description of actions. | Both are true. A row describes an action in the terms that matter for choosing it: which situations it's good in. The decision is `softmax(E @ h)`, made from both halves together. |
| The sampler has to be differentiable for backprop to work. | Sampling is not differentiable, and neither is the game engine. RL works around it: you differentiate the log-probability of the move that was sampled, not the sampling. |
| Forced moves still nudge the policy toward the only option. | With one legal action the softmax is always exactly 1, so log-prob is 0 and the gradient is exactly zero. Forced moves teach the policy nothing (though the state can still teach a value head). |
| You need the log-probability of the decision (the whole distribution). | You need the log-probability of the *one move that was played*: a single number per decision. |
| Multiplying by the reward works because it happens to be +1 / -1. | It falls out of the maths: the objective is expected reward, so its gradient is reward-weighted. Any reward scale works, and only *relative* rewards matter for direction, which leads to baselines (step 5). |
| A smaller learning rate gives a lower peak. | 3e-4 ended *higher* than 1e-3 (98-99% vs 97%). Smaller steps average out more batch noise; they cost speed, not quality. |
| Too large a learning rate just gives a lower peak. | 1e-2 made the policy *worse than random* (12%). In RL, bad steps produce worse training data, and big steps lock in near-certain bad habits. |
| A biased starting point traps the policy. | A mildly biased start (default init) recovered. Trapping needs near-certainty, which noisy big steps create and a mild bias doesn't. |
| The curve kept improving until about 4,000 games. | It levelled off around 1,300; later differences of 2-3 points sat inside the measurement noise of 400 evaluation games. |

---

## What you learned, step by step

### Step 1: PyTorch toy

- Tensors: shapes, dtypes, broadcasting, reductions along a dimension, bool tensors from comparisons.
- `nn.Module`, `nn.Sequential`, logits, and why no sigmoid goes at the end (the loss applies it stably).
- The training loop: shuffle, mini-batches, `zero_grad -> forward -> loss -> backward -> step`.
- Why mini-batches: many updates per epoch, a gradient that's good enough, hardware efficiency, helpful noise, bounded memory.
- `backward()` adds into `.grad`, so you clear it every batch, and why accumulating is the default (gradient accumulation, several losses).
- Loss modules are stateless; the loss *tensor* holds the graph; `.item()` keeps you from holding it alive; Adam's state is meant to persist.
- BCE: what it measures, why confident mistakes cost so much, and 0.693 as the coin-flip baseline.
- Results: linear 80%, mlp 100%, nolinear 79%, tiny 100% train / 69% test.
- Looking inside a trained MLP: weight heatmaps, correlating activations with features, probing by sweeping one input, dead ReLUs, redundancy.

### Step 2: win predictor

- Data comes from a process: labels you only know at the end of a game, metadata (`game`, `seat`, `progress`) kept beside the inputs, memory-aware collection.
- Boolean-mask indexing (`v[mask]`) and applying one mask across a dict of tensors.
- Leakage: splitting by row lets the model recognise a game's "fingerprint". It added about 14 points that meant nothing, most of all late in games, where states are most distinctive.
- Overfitting fixes compared by measurement, and the noise floor of a measurement: about 1,000 test games means about +-1.5 points, so smaller differences mean nothing.
- Distribution shift: a predictor trained on random play may be wrong, sometimes in reverse, on skilled play. So the value head has to keep training on the agent's own games.
- Results: 63% test (55% first quarter, 73% last); row split 77%; best fix 20k games + dropout at 69%.

### Step 3: policy network

- `nn.Embedding` as a lookup table, and its equivalence to a linear layer on one-hot inputs.
- The dot product as the shared core of the linear layer, the embedding version, and any later feature-based version.
- Softmax turns scores into probabilities; only differences between scores matter; ties stay mixed.
- `Categorical`: no parameters, `logits=` vs `probs=`, sampling a *position* and playing `legal[position]`.
- Why sample rather than always take the top move: exploration, mixed strategies, and what the learning rule assumes.
- `model.eval()` vs `torch.no_grad()`: layer behaviour vs gradient tracking.
- Initialisation scale sets an untrained model's confidence: default embeddings gave arbitrary, consistent preferences (35% win rate); std 0.01 gave uniform choices (51%).
- Premature convergence: a biased start can lock in a mediocre habit, because moves never tried are never learned.

### Step 4: REINFORCE

- RL vocabulary: environment, state, action, reward, policy, credit assignment.
- The loss's value is meaningless; only its gradient matters. Measure progress by win rate.
- Why the log-probability: its gradient is `grad p / p`, which cancels how often each move is sampled, so equally good moves get equal credit (the 0.9 vs 0.1 example).
- Why average over games, not decisions: a game's probability is the product of its moves' probabilities, so its log is their sum.
- Empty episodes and draws contribute zero but still count in the average.
- `torch.tensor(...)` copies values and cuts the gradient link; `torch.stack` keeps it. A missing `grad_fn` in a printout is the tell.
- Changing *when* you evaluate changes the whole run, because evaluation draws from the same random generator as training. Same code, two random streams: one run peaked at 97% and slid to 67%, the other stayed at 93-97%. One RL run proves little.
- It beat RandomBot within 640 games and levelled off around 1,300 (about 94%). Later "gains" were mostly inside the +-2 point measurement noise.
- What it learned, from wins and losses alone: never end a turn early, always Just Say No, play action and rent cards rather than banking them, build sets, and pay with money to keep properties. Habits that beat a random player, not necessarily a strong one.
- Learning rate: 3e-4 was slower but steadier and ended higher (98-99%); 1e-2 swung wildly and fell to 12%, far below random. Small steps let batch noise cancel; big steps act on it in full.
- In RL the policy makes its own training data, so a bad step produces worse data, and big steps saturate the softmax into near-certainty on bad moves. A confidently wrong player is worse than a random one. (PPO's step limit exists for this.)
- Batch size is the same lever: batch 8 learned faster at first, then collapsed to 0% and stayed there. Noise per update scales like 1/sqrt(batch).
- Starting biased (default init, 35%) slowed it and cost a few points mid-run, but it recovered: the bias was mild (entropy 0.91), so it still explored. Getting trapped needs near-certainty.
- Sources of REINFORCE's noise: random deals and opponent, the policy's own sampling, one reward shared by about 60 decisions (credit assignment, the biggest), a coarse +-1 reward, data that depends on the policy, and the evaluation's own noise.

---

## Bugs worth remembering

These come from your own code. Most will show up again in some other form.

| Bug | Lesson |
|---|---|
| `int(len(X) * (1 - test_fraction))` gave 62 instead of 63 for n=90 | Float arithmetic before truncation. Compute the quantity you mean directly, and round deliberately. |
| `squeeze()` instead of `squeeze(1)` | Name the dimension. A one-row batch otherwise collapses to a scalar. |
| `nn.BCELoss` on logits | The "WithLogits" versions take raw scores. Read the traceback: it ended in the probability version. |
| Epoch loss as a sum, then as a sum of means divided by rows | Numerator and denominator must count the same thing. The check passed both times: a green check isn't proof. |
| A fresh `torch.Generator()` in `train` | An unseeded generator always starts from the same fixed seed, so `torch.manual_seed` had no effect. |
| A ReLU after the output layer | Outputs that must be able to go negative (logits) can't pass through a ReLU. Accuracy stuck at chance is the symptom. |
| A new `RandomBot` created on every decision | Create stateful objects once, where their lifetime begins (per game), not inside the loop. |
| `g.winner()` | It's a property. "NoneType is not callable" meant the value was `None` (a draw caused by the bug above). |
| `data[mixed]` on a dict, `torch.tensor(dict or ragged list)` | Know your container. A dict of tensors and a list of tensors need dict comprehensions, `torch.stack` or plain loops. |
| A Python loop over 96,000 rows | Whole-tensor operations (masks, `isin`) are faster and can't misalign rows. |
| `model(X[i])` | `X[i]` drops the batch dimension. Models take `[n, features]`. |
| `exit()` inside a helper | Report misuse with an exception, so the caller gets a traceback and can react. |
| `log(softmax(scores))` | The two-step form loses precision and can produce `-inf`. Use the stable combined version the library provides. |
| `seed = g + seed + OFFSET` inside the loop | Reassigning an argument changes it for every later iteration. Seeds grew on each game. Give derived values their own names. |
| Recording the index of the last game instead of the number played | Off by one: a 0-based index is one less than a count. |
| `if i % eval_every == 0` after the update | `i` counts updates finished *before* this pass. Trace a loop by hand with a small number to see which value the test needs. |
| `i+1 % eval_every` | `%` binds tighter than `+`, as `*` does. When in doubt, parenthesise. |
| `print("game: {g}")` | Without the `f` prefix the braces print literally. |

---

## Other learnings worth keeping

General habits that came out of the work above, beyond any one step.

- **Count independent units, not rows.** Rows from one game aren't independent, so judge both data size and measurement noise in games. About 1,000 test games gives about +-1.5 points; smaller differences between runs mean nothing.
- **Predict before you run.** Your ranking of the overfitting fixes was mostly wrong in an informative way. The gap between prediction and result is where the learning happened.
- **A passing check is evidence, not proof.** The summed epoch loss passed the toy's check; the empty-stage `nan` passed by accident. Ask what a check doesn't test.
- **Print shapes, dtypes and `grad_fn` first when something's off.** Most of the bugs above (the dict passed to `torch.tensor`, the dropped batch dimension, the cut gradient) are visible in one print.
- **Keep the skepticism.** Pushing back on the embedding framing until it made sense found a real wording problem and produced a better mental model. Ask for the evidence behind a claim, including mine.
