# Next steps: setup guides

How to set up each step after the [PyTorch toy](01_pytorch_toy/). You write the ML code. Ask Claude for hints, reviews, or a `TODO` skeleton with checks when you reach a step.

Finish the toy first: `train`, then the part 5 experiments and their questions.

## Ground rules for every step

- **Run everything with uv** from anywhere in the repo: `uv run python script.py`. To add a package, use `uv add <package>`.
- **Start with 2 players.** It's the simplest case and has almost no draws. Add more players once 2 works.
- **Set a seed and measure.** Each step below has a "done when" test. If a change doesn't move the measurement, it didn't help.
- **The API is documented** in [`../bindings/monopd.pyi`](../bindings/monopd.pyi).

The core loop you'll reuse throughout:

```python
import monopd

game = monopd.Game(num_players=2, seed=0)
bot = monopd.RandomBot(seed=0)
while not game.is_over():
    seat = game.current_player()     # may not be the player whose turn it is
    obs = game.observe(seat)         # monopd.OBS_SIZE floats, from that seat's view
    legal = game.legal_actions()     # action indices
    game.step(bot.choose(game))
print(game.winner, game.returns())   # winner is None on a draw
```

---

## Step 2: win predictor

**Goal:** predict from a state whether the player to act goes on to win. It's the toy's recipe on real data, and the network later becomes the value head.

**Setup:** create `learn/02_win_predictor/`, and copy over your `MLP`, `train` and `accuracy` as a starting point. `MLP` will need its input size as a parameter (`monopd.OBS_SIZE`, 1,474).

**What you'll write:**

1. **Collect data.** Play random games and save `(observe(seat), did seat win?)`. You only know the label when the game ends, so hold each game's rows until then.
2. **Split by whole game.** Assign games to train or test, not individual rows.
3. **Train** with your existing code, and watch both train and test accuracy.
4. **Break accuracy down by game stage,** such as the first and last quarter of each game. Save each row's position in its game when you collect it.

**Things to watch:**

- **Sample states.** Consecutive states are almost identical. Keeping every 8th decision worked well in a test run.
- **Mind the memory.** Python lists of floats are large, so convert each observation to a tensor as you collect it, or collect in chunks. In the test run, the final tensor alone was about 565 MB.
- **Skip draws for now,** since they have no winner. There were none in 5,000 2-player games.
- **Save the dataset** with `torch.save` so you don't need to regenerate it for each training run. `.pt` files are gitignored.
- **Label from the right seat.** The label is whether the seat you observed won, not whether player 0 won.

**What to expect:** figures from one quick test run on 2-player games, so use them as a rough guide:

| | Result |
|---|---|
| Data: 5,000 games, every 8th decision | About 96,000 rows, collected in about 20 seconds |
| Network: 256 → 256, 8 epochs, on CPU | About 6 seconds to train |
| Test accuracy overall | About 62% |
| Test accuracy by game stage | About 54% in the first quarter, rising to about 71% in the last |
| Train accuracy | About 90%: clear overfitting |

Random games are noisy, so nowhere near 100% is expected: a lucky draw can flip a game late on.

**Done when:**
- it clearly beats 50% on games it never saw
- accuracy rises later in the game

**Questions to answer:**
1. What happens to test accuracy if you split by row rather than by game? Why?
2. How would you reduce the overfitting? Try more games, then dropout or a smaller network, and compare.
3. These states come from random play. How might the predictor do on games played by a strong agent?

---

## Step 3: policy network, untrained

**Goal:** the two-column model makes every decision in a full game.

**Setup:** create `learn/03_policy/`.

**What you'll write:**

1. **State column:** reuse your MLP layers, ending in 256 numbers (`h`) rather than one.
2. **Action column:** `nn.Embedding(monopd.NUM_ACTIONS, 256)`, looked up with a tensor of legal action indices, giving `E` with shape `[K, 256]`.
3. **Choosing a move:** `scores = E @ h`, then `torch.distributions.Categorical(logits=scores)` to sample a position in the legal list. Remember to map that position back to the action index.
4. **A play function:** your model against `monopd.RandomBot`, with a win-rate counter.

**Things to watch:**
- **Position vs action index:** the sampled number is a position in `legal`. The action to play is `legal[position]`.
- **Seats:** alternate which seat your model takes, since seat 0 goes first and may have a small edge.
- **Decisions on other turns:** `current_player()` decides who acts. Your model also pays rent and answers Just Say No on the opponent's turn.

**Done when:** it plays full games without errors and wins about 50% against a random bot, over at least 1,000 games. An untrained model is effectively random, so 50% means your setup is working.

---

## Step 4: REINFORCE

**Goal:** the win rate climbs. This is the big milestone.

**Read first:** OpenAI's *Spinning Up*, "Part 3: Intro to Policy Optimization".

**What you'll write:**

1. During a game, save `dist.log_prob(position)` for each decision your model makes.
2. At the end, `reward = game.returns()[my_seat]`.
3. `loss = -(sum of log_probs) * reward`, then backpropagate and step, usually averaged over a batch of games.
4. Every so often, measure win rate over about 1,000 games with no training, and plot it.

**Things to watch:**
- **Only your model's decisions count,** not the bot's.
- **Skip forced moves.** When there's one legal action, there's nothing to learn, so you can play it without saving anything. Many Just Say No and Double The Rent questions are forced.
- **Expect noise.** REINFORCE is high-variance, so plot a smoothed curve and try several seeds before concluding anything.
- **Separate training from evaluation.** A rising training reward can mislead, so use a fixed evaluation run for measurement.

**Done when:** a plot of win rate against the random bot climbs well above 50% and stays there.

---

## Step 5: actor-critic

**Goal:** faster, steadier learning than step 4.

**Change:** add a value head on `h`, trained to predict the game's outcome. Use `(reward - predicted value)` in the policy loss, with the prediction detached so the policy loss doesn't train the value head.

**Done when:** on the same seeds, it reaches a given win rate in fewer games than REINFORCE, or its curve is visibly steadier.

---

## Step 6: self-play

**Goal:** learn beyond beating a random bot.

**Setup:** save copies of your model during training, and sample opponents from that pool, with the random bot still included.

**Things to watch:**
- **Win rate against the current self hovers near 50%,** even while improving, so it stops being useful as a measure. Keep a fixed benchmark: the random bot, plus a set of saved checkpoints.
- **A heuristic bot is a better yardstick.** Write a simple rule-based one in Python, such as "complete sets first, then charge rent". If the agent can't beat it, something's wrong.

**Done when:** the newest model beats older checkpoints and your heuristic bot.

---

## Step 7: PPO

**Goal:** the algorithm to use long term.

**Read first:** CleanRL's single-file `ppo.py`, noting which parts you already recognise.

**New ideas:** generalised advantage estimation, a clipped policy loss, several update passes per batch of games, and an entropy bonus that helps it keep exploring.

**Done when:** it beats your step 6 agent head to head.

---

## After that

- **More players:** train on 3, 4 and 5 players. In 500 test games, draws from the turn limit were rare at 3 players (1 game) but common at 5 (167 games) under random play. See how often they happen with trained agents.
- **Action features:** add an engine function that describes each action, and compare it against the lookup table.
- **Attention across players:** the first architecture upgrade to try.
- **Play it yourself:** connect a trained agent to the web viewer.
