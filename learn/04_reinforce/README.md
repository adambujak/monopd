# Step 4: REINFORCE

The policy learns. Your step 3 model plays RandomBot, and after every batch of games its moves are nudged: those from won games become more likely, those from lost games less likely. Nothing tells it which move was good; it only sees who won.

**Read first:** OpenAI's Spinning Up, [Part 3: Intro to Policy Optimization](https://spinningup.openai.com/en/latest/spinningup/rl_intro3.html), "Deriving the Simplest Policy Gradient". The "Implementing" section right after it is close to this step's solution in another setting: read it now for a worked example, or afterwards if you'd rather work it out yourself.

```bash
cd learn/04_reinforce
uv run python reinforce.py check      # tells you which part to do next
uv run python reinforce.py train      # once every part passes: plots reinforce.png, saves reinforce.pt
uv run python reinforce.py show       # watch the trained policy play
```

`train` options: `--updates`, `--batch` (games per update), `--lr`, `--emb-std` (0 keeps PyTorch's default), `--seed` (training games and sampling), `--model-seed` (initial weights), `--eval-every`, `--eval-games`, `--out` (file name, so runs don't overwrite each other). `show` takes `--model <name>` to load a run saved with `--out`.

This step imports your `PolicyNet` and `evaluate` from [`03_policy/policy.py`](../03_policy/policy.py).

---

## Part 1: play one game, keeping what learning needs

Write `play_episode`: like step 3's `play_game`, but gradients stay on, forced moves skip the model, and each real decision keeps the log-probability of the move it chose.

## Part 2: the loss

Write `reinforce_loss`: one number for a batch of games whose gradient pushes in the right direction.

**Check yourself:** what does this loss's value tell you about how well the policy plays? (Compare with steps 1 and 2.)

## Part 3: the training loop

Write `train`: play a batch, compute the loss, step the optimizer, and measure on fixed evaluation games every so often.

**Check yourself:** the evaluation uses the same 400 games (seeds 0 to 399) every time. Why is that better than fresh games each time, for comparing one evaluation with the next?

---

## Part 4: experiments and questions

Each `train` run with the defaults takes about 5 minutes. Use `--out` to keep runs side by side.

**Done when:** `reinforce.png` climbs well above 50% and stays there.

1. **The default run.** How many training games until it clearly beats the random bot, and where does it level off? Then run `show`: what has it learned to prefer, and what has it learned to avoid?
2. **Learning rate.** Run `--lr 3e-4` and `--lr 1e-2`. What happens to each curve, and why?
3. **Batch size.** Run `--batch 8 --updates 800 --eval-every 80` (the same number of games as the default). Compare it with the default. What does a smaller batch do to each update?
4. **Starting point.** Run `--emb-std 0`, PyTorch's default initialisation, which won only 35% in step 3. Does starting biased hurt?
5. **Noise.** Run the default with `--seed 1` and `--seed 2`. How different are the curves? Where does REINFORCE's noise come from?

Bonus:
- `show` picks moves by sampling. How does the trained policy do if it always takes its top move? (You'd change `evaluate` or write a variant.)
- What's its win rate when it plays *against itself*? Why is that number not useful for measuring progress? (This is step 6's problem.)
