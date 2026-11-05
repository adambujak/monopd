# Toy problem: has this player won?

A table is 10 numbers: how many properties a player holds of each colour. They've won with 3 or more complete sets. You'll build a network that learns this rule purely from examples.

It's a miniature of the real project's first step, the win predictor. Every piece you write here (tensors, a model class, a training loop, accuracy, a train/test split) you'll reuse on the real game.

```bash
cd learn/01_pytorch_toy
uv run python toy.py check         # tells you which part to do next
```

Work through the parts in order. Run `check` after each one. When you're stuck, look at the hints, then ask me.

---

## Part 1: tensors

Write `complete_mask`, `count_complete` and `won` with no Python loops, using only whole-tensor operations.

**Concepts:** a tensor's shape and dtype; comparisons produce bool tensors; reductions along a dimension; broadcasting.

<details><summary>Hints</summary>

- Turn `SET_SIZES` into a tensor. Its shape is `[10]`, and `X` is `[n, 10]`. What happens when you compare `X >= sizes`? Look up "broadcasting" in the PyTorch docs.
- `.sum(dim=...)` adds along one dimension. Which dimension are the colours?
- Summing a bool tensor counts the `True`s.
- `.float()` converts a bool tensor to 0.0 / 1.0.

</details>

**Check yourself:** what's the shape of `X >= sizes`? Why didn't you need to repeat `sizes` n times?

---

## Part 2: train/test split

Write `split`: shuffle the rows, keep `X` and `y` lined up, and cut off the last 20% for testing.

**Concepts:** indexing a tensor with a tensor of indices; why you test on data the model never trained on.

<details><summary>Hints</summary>

- `torch.randperm(n)` gives a shuffled ordering of `0..n-1`.
- `X[idx]` picks rows by an index tensor. Use the same `idx` for `y`.
- Use a `torch.Generator` seeded with `seed` (or call `torch.manual_seed`) so the split is the same every run.

</details>

---

## Part 3: models

Write `LinearModel` (one `nn.Linear(10, 1)`) and `MLP` (three linear layers with ReLUs between them).

**Concepts:** `nn.Module`; layers created in `__init__` and used in `forward`; logits; `nn.Sequential`.

<details><summary>Hints</summary>

- Layers must be attributes (`self.layer = ...`) so PyTorch finds their parameters.
- `nn.Sequential(a, b, c)` chains layers. You can build its list of layers conditionally for `activation=False`.
- Don't put a sigmoid at the end: return raw logits. The loss in part 4 applies the sigmoid itself, which is more numerically stable.

</details>

**Check yourself:** how many parameters does `MLP()` have? Count them by hand, then compare with `sum(p.numel() for p in MLP().parameters())`.

---

## Part 4: training and evaluation

Write `accuracy` and `train`.

**Concepts:** the training loop. Every deep-learning project is this loop:

```
for each epoch:
    shuffle the data
    for each mini-batch:
        prediction = model(batch)
        loss = how wrong the prediction is
        clear old gradients
        compute new gradients (backpropagation)
        nudge the weights (optimizer step)
```

<details><summary>Hints</summary>

- Loss: `nn.BCEWithLogitsLoss()`, binary cross-entropy that takes logits. Watch the shapes: the model gives `[n, 1]` and `y` is `[n]`.
- Optimizer: `torch.optim.Adam(model.parameters(), lr=lr)`.
- The three calls, in this order: `optimizer.zero_grad()`, `loss.backward()`, `optimizer.step()`.
- `loss.item()` turns a one-number tensor into a Python float.
- In `accuracy`, wrap the work in `with torch.no_grad():`. You're not training, so don't build the gradient graph.

</details>

---

## Part 5: experiments

Once `check` passes everything, run each experiment and answer the questions before reading on.

```bash
uv run python toy.py train linear
uv run python toy.py train mlp
uv run python toy.py train nolinear
uv run python toy.py train tiny
```

1. **`linear`:** what accuracy does it reach? About half the tables are wins, so 50% is coin-flipping. Why can't a single linear layer get this right? Think about what a weighted sum of counts can and can't express, given that only whole sets count.
2. **`mlp`:** how much better? What do the hidden layers let it do that the linear model couldn't?
3. **`nolinear`:** the same three layers, minus the ReLUs. Compare it with `linear` and explain the result. (Write out what two linear layers in a row compute.)
4. **`tiny`:** 200 training rows and a big network. Compare train accuracy with test accuracy. What's happening, and what are two ways to fix it?

Bonus, if you want more:
- Change `lr` to 1.0, then to 0.00001. What happens to the loss, and why?
- Plot the loss per epoch with matplotlib.
- Make the problem harder: a set only counts if a second number per colour (like "has a house") is also on. Does the MLP still learn it?

When you're done, tell me your answers. Next is the same thing on real Monopoly Deal states, collected with `import monopd`.
