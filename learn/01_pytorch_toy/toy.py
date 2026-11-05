"""Toy problem: has this player won?

A player's table is 10 numbers: how many properties they hold of each colour.
They've won if at least 3 colours are complete sets. Your network gets the 10
numbers and must say "won" or "not won".

Fill in every function marked TODO, in order. Check your work as you go:

    uv run python toy.py check            # runs the checks for every part
    uv run python toy.py train linear     # experiments, once part 4 passes
    uv run python toy.py train mlp
    uv run python toy.py train nolinear
    uv run python toy.py train tiny

See README.md for the walkthrough, hints and questions.
"""

import random
import sys

import torch
from torch import nn

# How many properties complete each colour, in this order:
# Brown, LightBlue, Pink, Orange, Red, Yellow, Green, DarkBlue, Railroad, Utility
SET_SIZES = [2, 3, 3, 3, 3, 3, 3, 2, 4, 2]
SETS_TO_WIN = 3


def make_data(n, seed=0):
    """n random tables. Returns X: float [n, 10] property counts, y: float [n] (1.0 = won).

    Provided for you. Each colour count is random between 0 and a full set.
    """
    rng = random.Random(seed)
    rows, labels = [], []
    for _ in range(n):
        row = [rng.randint(0, size) for size in SET_SIZES]
        complete = sum(1 for count, size in zip(row, SET_SIZES) if count >= size)
        rows.append(row)
        labels.append(1.0 if complete >= SETS_TO_WIN else 0.0)
    return torch.tensor(rows, dtype=torch.float32), torch.tensor(labels)


# ===========================================================================
# Part 1: tensors. No loops: use whole-tensor operations and broadcasting.
# ===========================================================================

def complete_mask(X):
    """X: float [n, 10] property counts.
    Returns a bool tensor [n, 10]: True where that colour is a complete set.
    """
    raise NotImplementedError("TODO part 1")


def count_complete(X):
    """Returns an integer tensor [n]: how many complete sets each player has."""
    raise NotImplementedError("TODO part 1")


def won(X):
    """Returns a float tensor [n]: 1.0 where the player has won, else 0.0."""
    raise NotImplementedError("TODO part 1")


# ===========================================================================
# Part 2: a train/test split.
# ===========================================================================

def split(X, y, test_fraction=0.2, seed=0):
    """Shuffle the rows (keeping X and y lined up), then split.
    Returns (X_train, y_train, X_test, y_test).
    """
    raise NotImplementedError("TODO part 2")


# ===========================================================================
# Part 3: models. Both take X [n, 10] and return logits [n, 1].
# (A logit is a raw score: > 0 means "won", < 0 means "not won".)
# ===========================================================================

class LinearModel(nn.Module):
    """A single linear layer: 10 inputs -> 1 output."""

    def __init__(self):
        super().__init__()
        raise NotImplementedError("TODO part 3")

    def forward(self, x):
        raise NotImplementedError("TODO part 3")


class MLP(nn.Module):
    """10 -> hidden -> hidden -> 1, with a ReLU after each hidden layer.
    `activation=False` leaves the ReLUs out (for an experiment later).
    """

    def __init__(self, hidden=64, activation=True):
        super().__init__()
        raise NotImplementedError("TODO part 3")

    def forward(self, x):
        raise NotImplementedError("TODO part 3")


# ===========================================================================
# Part 4: training and evaluation.
# ===========================================================================

def accuracy(model, X, y):
    """Fraction of rows where the model's prediction (logit > 0) matches y.
    Returns a Python float. Don't track gradients here.
    """
    raise NotImplementedError("TODO part 4")


def train(model, X, y, epochs=30, lr=1e-2, batch_size=64):
    """Train with Adam and binary cross-entropy on shuffled mini-batches.
    Returns a list with the average loss of each epoch.
    """
    raise NotImplementedError("TODO part 4")


# ===========================================================================
# Below here is provided: checks and experiments. You don't need to edit it.
# ===========================================================================

def _check_part1():
    X = torch.tensor([[2, 3, 3, 0, 0, 0, 0, 0, 0, 0],    # Brown, LightBlue, Pink complete: won
                      [1, 3, 3, 3, 0, 0, 0, 0, 4, 2],    # 5 complete: won
                      [2, 2, 2, 2, 2, 2, 2, 1, 3, 1],    # 1 complete
                      [0, 0, 0, 0, 0, 0, 0, 0, 0, 0]], dtype=torch.float32)
    mask = complete_mask(X)
    assert mask.shape == (4, 10), f"complete_mask shape should be (4, 10), got {tuple(mask.shape)}"
    assert mask.dtype == torch.bool, f"complete_mask should be bool, got {mask.dtype}"
    assert mask[0].tolist() == [True, True, True] + [False] * 7, "complete_mask row 0 is wrong"
    counts = count_complete(X)
    assert counts.tolist() == [3, 5, 1, 0], f"count_complete should be [3, 5, 1, 0], got {counts.tolist()}"
    w = won(X)
    assert w.dtype == torch.float32, f"won should be float32, got {w.dtype}"
    assert w.tolist() == [1.0, 1.0, 0.0, 0.0], f"won should be [1, 1, 0, 0], got {w.tolist()}"
    X, y = make_data(2000, seed=1)
    assert torch.equal(won(X), y), "won() disagrees with make_data on random tables"


def _check_part2():
    X, y = make_data(100)
    X_tr, y_tr, X_te, y_te = split(X, y, test_fraction=0.2)
    assert len(X_tr) == 80 and len(X_te) == 20, f"expected 80/20 rows, got {len(X_tr)}/{len(X_te)}"
    assert len(y_tr) == 80 and len(y_te) == 20, "y should be split the same way as X"
    assert torch.equal(won(X_tr), y_tr) and torch.equal(won(X_te), y_te), "X and y rows got out of line"
    assert not torch.equal(X_tr, X[:80]), "rows should be shuffled before splitting"
    together = sorted(map(tuple, torch.cat([X_tr, X_te]).tolist()))
    assert together == sorted(map(tuple, X.tolist())), "every row should end up in exactly one half"


def _check_part3():
    X, _ = make_data(5)
    for model in (LinearModel(), MLP(), MLP(activation=False)):
        out = model(X)
        assert out.shape == (5, 1), f"{type(model).__name__} should output (5, 1), got {tuple(out.shape)}"
    n_params = sum(p.numel() for p in LinearModel().parameters())
    assert n_params == 11, f"LinearModel should have 10 weights + 1 bias = 11 parameters, has {n_params}"
    layers = [m for m in MLP().modules() if isinstance(m, nn.Linear)]
    assert len(layers) == 3, f"MLP should have 3 linear layers, has {len(layers)}"
    assert any(isinstance(m, nn.ReLU) for m in MLP().modules()), "MLP should use nn.ReLU"
    assert not any(isinstance(m, nn.ReLU) for m in MLP(activation=False).modules()), "activation=False should have no ReLU"


def _check_part4():
    X, y = make_data(512, seed=2)

    class Always(nn.Module):
        def forward(self, x):
            return torch.ones(len(x), 1)

    acc = accuracy(Always(), X, y)
    assert isinstance(acc, float), f"accuracy should return a Python float, got {type(acc).__name__}"
    assert abs(acc - y.mean().item()) < 1e-6, "an always-'won' model should score the fraction of winners"
    torch.manual_seed(0)
    model = MLP()
    losses = train(model, X, y, epochs=5)
    assert len(losses) == 5, f"train should return one loss per epoch, got {len(losses)}"
    assert all(isinstance(l, float) for l in losses), "losses should be Python floats (use .item())"
    assert losses[-1] < losses[0], f"loss should go down while training: {losses}"


def check():
    parts = [("1 tensors", _check_part1), ("2 split", _check_part2), ("3 models", _check_part3), ("4 training", _check_part4)]
    for name, fn in parts:
        try:
            fn()
            print(f"  part {name}: passed")
        except NotImplementedError:
            print(f"  part {name}: not started yet. Next up: the TODOs in part {name.split()[0]}.")
            return
        except AssertionError as e:
            print(f"  part {name}: not quite. {e}")
            return
    print("\nAll parts pass. Now run the experiments: uv run python toy.py train linear | mlp | nolinear | tiny")


def experiment(name):
    torch.manual_seed(0)
    n = 250 if name == "tiny" else 25_000
    X, y = make_data(n)
    X_tr, y_tr, X_te, y_te = split(X, y)
    if name == "tiny":
        X_te, y_te = make_data(5000, seed=99)  # a fair test set even with tiny training data
    model = {
        "linear": lambda: LinearModel(),
        "mlp": lambda: MLP(),
        "nolinear": lambda: MLP(activation=False),
        "tiny": lambda: MLP(hidden=256),
    }[name]()
    epochs = 300 if name == "tiny" else 30
    losses = train(model, X_tr, y_tr, epochs=epochs)
    print(f"{name}: {len(X_tr)} training rows, {epochs} epochs")
    print(f"  loss: first epoch {losses[0]:.3f}, last epoch {losses[-1]:.3f}")
    print(f"  accuracy: train {accuracy(model, X_tr, y_tr):.1%}, test {accuracy(model, X_te, y_te):.1%}")


if __name__ == "__main__":
    if len(sys.argv) >= 2 and sys.argv[1] == "check":
        check()
    elif len(sys.argv) == 3 and sys.argv[1] == "train":
        experiment(sys.argv[2])
    else:
        print(__doc__)
