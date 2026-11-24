# Step 5: actor-critic

REINFORCE pushes every move of a game by the same ±1. Here a second output on `h`, the **critic** (a value head), predicts the reward from each state. Each move is then pushed by its **advantage**: how much better or worse the game went than the critic expected. The average direction of learning doesn't change, but the noise should drop. This is step 2's win predictor, finally used, and trained on the agent's own games.

This step also gets a harder opponent. Training games alternate between RandomBot and a **frozen copy of your step 4 model**, and evaluation reports both. Against the random bot alone there's little left to improve on; the frozen model gives room to see a difference.

```bash
cd learn/05_actor_critic
uv run python actor_critic.py check                            # tells you which part to do next
uv run python actor_critic.py train --out ac                   # once every part passes, ~10 min
uv run python actor_critic.py train --no-baseline --out rf     # the same run as plain REINFORCE
uv run python actor_critic.py compare ac rf                    # both on one plot
uv run python actor_critic.py show --model ac                  # a game with the critic's predictions
```

Needs `../04_reinforce/reinforce.pt` from step 4's `train`. `train` takes the same options as step 4 (`--updates`, `--batch`, `--lr`, `--seed`, `--model-seed`, `--eval-every`, `--eval-games`), plus `--no-baseline`. `show` takes `--vs random` or `--vs frozen`.

**Read first:** Spinning Up Part 3, "Baselines in Policy Gradients", and the paragraph after it on value functions as baselines.

---

## Part 1: the model

Write `ActorCritic`: your `PolicyNet` with a value head on `h`. `forward` returns the logits as before, plus the value.

**Check yourself:** the value has to stay between -1 and +1. Why that range, and what does a value of 0 mean?

## Part 2: play one game

Write `play_episode`: like step 4's, but against any opponent, and keeping the critic's prediction at each decision as well.

## Part 3: the loss

Write `actor_critic_loss`: the policy loss with advantages in place of rewards, plus the critic's own loss.

**Check yourself:** the docstring says to treat the value as a constant inside the advantage. What would go wrong if the policy loss could change the critic's prediction?

## Part 4: the training loop

Write `train`: step 4's loop, with the opponent mix and both evaluations.

---

## Part 5: experiments and questions

Each run takes about 10 minutes. Two can run at once in separate terminals.

**Done when:** on the same seed, the actor-critic run reaches a given win rate against the frozen model in fewer games than the no-baseline run, or its curve is visibly steadier, and you've checked whether that holds on a second seed.

1. **Baseline or not.** Run `ac` and `rf` as above and `compare` them. Which climbs faster against the frozen model? Which is steadier?
2. **Is it real?** Run both again with `--seed 1` (`--out ac1`, `--out rf1`) and compare all four. Is the difference between baseline and no baseline bigger than the difference between seeds?
3. **The collapse.** Step 4's batch of 8 collapsed. Run both with `--batch 8 --updates 800 --eval-every 80`. Does the baseline change what happens? Why would it?
4. **What the critic learned.** Run `show`. How does the critic's prediction move through a game? Does it see a win or a loss coming, and how early?
5. **Two yardsticks.** Do the win rates against the random bot and against the frozen model rise together? What would it mean if one rose while the other fell?
