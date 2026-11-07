# Step 2: win predictor

From one state of a real game, predict whether the player about to act goes on to win. It's the toy's recipe on real data: collect examples, split them, train an MLP, measure. This network later becomes the agent's value head.

```bash
cd learn/02_win_predictor
uv run python predictor.py check       # tells you which part to do next
uv run python predictor.py collect     # once part 1 passes: ~20 s, saves data.pt (~570 MB, gitignored)
uv run python predictor.py train       # once every part passes
```

`train` takes `--epochs`, `--hidden` and `--by-row` (for question 1). `collect` takes `--games` and `--every`.

Work through the parts in order and run `check` after each one. The API for `monopd` is in [`bindings/monopd.pyi`](../../bindings/monopd.pyi).

---

## Part 1: collect data

Write `collect`: play random 2-player games and keep every `every`-th state, with its seat, its position in the game and its label. The docstring pins down exactly which games and which decisions, so the check can replay them.

**Concepts:** data comes from a process, not a file; labels you only learn later; memory.

<details><summary>Hints</summary>

- The loop is in [`NEXT_STEPS.md`](../NEXT_STEPS.md). Here every seat is played by the same `RandomBot`.
- You know the winner, and the game's total number of decisions, only when the game ends. Hold the game's rows in a list until then, and only then add the labels and progress.
- `observe` returns a Python list of 1,474 floats, and 96,000 of those take several GB. Convert each one with `torch.tensor(...)` straight away, then `torch.stack` the list at the end.
- Watch the dtypes: `torch.tensor` of Python ints gives int64, which is what `game` and `seat` want. Of Python floats it gives float32.
- Skipping draws: in 2-player random games there are essentially none, so the check can't test this. Do it anyway.

</details>

**Check yourself:** why record the observation *before* stepping? What would go wrong if you recorded it after?

---

## Part 2: split by whole game

Write `split_by_game`: choose whole games for the test set, then send each row to the side its game is on.

**Concepts:** data leakage; masks over rows.

<details><summary>Hints</summary>

- `data["game"].unique()` gives the game ids. Shuffle those with your own generator, as in the toy, and take the first `round(...)` as the test games.
- `torch.isin(data["game"], test_games)` gives a bool mask over rows. `~mask` is the rest.
- A dict comprehension applies the same mask to every key: `{k: v[mask] for k, v in data.items()}`.

</details>

---

## Part 3: model and training

Bring over your toy's `MLP`, `accuracy` and `train`, and make `MLP` take its input size as a parameter.

<details><summary>Hints</summary>

- The defaults change: a smaller `lr` (`1e-3`) and bigger batches (`256`) suit a bigger network and more data.
- Now's the time to add `model.eval()` in `accuracy` (and `model.train()` when it's done). It makes no difference yet, but question 2 adds dropout, and then it will.

</details>

**Check yourself:** how many parameters does `MLP(1474, hidden=256)` have? Which layer holds most of them?

---

## Part 4: accuracy by game stage

Write `accuracy_by_stage`: split the rows into quarters by `progress` and measure accuracy in each.

<details><summary>Hints</summary>

- `(progress * n_stages).floor().long()` gives each row's stage number.
- Reuse your `accuracy` on each stage's rows.

</details>

---

## Part 5: experiments and questions

Run `collect` once, then `train`. Expect roughly 60% test accuracy overall, rising across the quarters. Random games are noisy: a lucky draw can flip a game late on, so nowhere near 100% is possible.

**Done when:** test accuracy clearly beats 50%, and it rises later in the game.

1. **Leakage.** Run `train --by-row`. How does test accuracy change, and why? Think about how similar two states from the same game are.
2. **Overfitting.** Compare train with test accuracy. Try fixes and measure each one:
   - more games (`collect --games 20000`; check your memory first)
   - a smaller network (`--hidden 32`)
   - dropout: add `nn.Dropout` to your `MLP`
   - fewer epochs
3. **Distribution.** These states come from random play. How might the predictor do on games between strong players, and what does that mean for using it as a value head later?

Bonus:
- Per-epoch test accuracy: when does it peak? That's early stopping.
- Accuracy by seat: does the player who moves first win more often?
- Which inputs matter? Zero out one block of the observation (your hand, the opponent's table, ...) at test time and see what accuracy drops to.
