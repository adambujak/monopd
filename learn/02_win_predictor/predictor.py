"""Win predictor: from one state of a real game, will the player to act win?

The toy's recipe on real Monopoly Deal data. This network later becomes the
value head of the agent.

Fill in every function marked TODO, in order. Check your work as you go:

    uv run python predictor.py check              # runs the checks for every part
    uv run python predictor.py collect            # play games, save data.pt (once part 1 passes)
    uv run python predictor.py train              # train and evaluate (once every part passes)

See README.md for the walkthrough, hints and questions.
"""

import argparse
import sys
from pathlib import Path

import monopd
import torch
from torch import nn

DATA_PATH = Path(__file__).with_name("data.pt")


# ===========================================================================
# Part 1: collect data from random games.
# ===========================================================================

def collect(n_games, every=8, seed=0):
    """Play n_games 2-player games with RandomBot in every seat, and sample states.

    Game i (0-based) must be `monopd.Game(num_players=2, seed=seed + i)`, played
    by one `monopd.RandomBot(seed=seed + i)` choosing for whoever is to act.

    Number the decisions of each game 0, 1, 2, ... (one per `step`). Keep a row
    for decision d when d % every == 0, recorded *before* stepping:
      - the observation of the player about to decide, from their own seat
      - that seat
      - progress = d / (total decisions in this game), in [0, 1)
      - the label: 1.0 if that seat goes on to win, else 0.0

    Skip games that end in a draw (winner is None).

    Returns a dict of tensors, one row per sampled decision:
        "X":        float32 [N, monopd.OBS_SIZE]
        "y":        float32 [N]
        "game":     int64   [N]   the game index i
        "seat":     int64   [N]
        "progress": float32 [N]
    """
    raise NotImplementedError("TODO part 1")


# ===========================================================================
# Part 2: split by whole game.
# ===========================================================================

def split_by_game(data, test_fraction=0.2, seed=0):
    """Put round(test_fraction * number of games) whole games in the test set.

    Every row of a game goes to the same side. Choose the test games at random
    with your own seeded generator (as in the toy's split).
    Returns (train, test): two dicts with the same keys as `data`.
    """
    raise NotImplementedError("TODO part 2")


# ===========================================================================
# Part 3: model and training. Start from your toy code.
# ===========================================================================

class MLP(nn.Module):
    """in_features -> hidden -> hidden -> 1, with a ReLU after each hidden layer."""

    def __init__(self, in_features, hidden=256):
        super().__init__()
        raise NotImplementedError("TODO part 3")

    def forward(self, x):
        raise NotImplementedError("TODO part 3")


def accuracy(model, X, y):
    """Fraction of rows where (logit > 0) matches y, as a Python float."""
    raise NotImplementedError("TODO part 3")


def train(model, X, y, epochs=8, lr=1e-3, batch_size=256):
    """Adam + BCE-with-logits on shuffled mini-batches.
    Returns the row-weighted average loss of each epoch (Python floats).
    """
    raise NotImplementedError("TODO part 3")


# ===========================================================================
# Part 4: accuracy by game stage.
# ===========================================================================

def accuracy_by_stage(model, X, y, progress, n_stages=4):
    """Accuracy separately for each stage of the game.

    Stage s holds the rows with s / n_stages <= progress < (s + 1) / n_stages.
    Returns a list of n_stages Python floats (float('nan') for an empty stage).
    """
    raise NotImplementedError("TODO part 4")


# ===========================================================================
# Below here is provided: checks and experiments. You don't need to edit it.
# ===========================================================================

def _replay(seed):
    """Every (decision, seat, observation) of one game, and its winner."""
    g = monopd.Game(num_players=2, seed=seed)
    bot = monopd.RandomBot(seed=seed)
    steps = []
    while not g.is_over():
        seat = g.current_player()
        steps.append((seat, g.observe(seat)))
        g.step(bot.choose(g))
    return steps, g.winner


def _check_part1():
    data = collect(6, every=5, seed=40)
    keys = {"X", "y", "game", "seat", "progress"}
    assert isinstance(data, dict) and set(data) == keys, f"collect should return a dict with keys {sorted(keys)}"
    n = len(data["X"])
    expect = {"X": ((n, monopd.OBS_SIZE), torch.float32), "y": ((n,), torch.float32),
              "game": ((n,), torch.int64), "seat": ((n,), torch.int64), "progress": ((n,), torch.float32)}
    for k, (shape, dtype) in expect.items():
        assert tuple(data[k].shape) == shape, f'data["{k}"] should have shape {shape}, got {tuple(data[k].shape)}'
        assert data[k].dtype == dtype, f'data["{k}"] should be {dtype}, got {data[k].dtype}'
    assert sorted(set(data["game"].tolist())) == list(range(6)), "game should hold the game index 0..n_games-1"
    for i in range(6):
        steps, winner = _replay(40 + i)
        rows = (data["game"] == i).nonzero().squeeze(1)
        kept = list(range(0, len(steps), 5))
        assert len(rows) == len(kept), f"game {i}: expected {len(kept)} rows (every 5th of {len(steps)} decisions), got {len(rows)}"
        for r, d in zip(rows.tolist(), kept):
            seat, obs = steps[d]
            assert data["seat"][r].item() == seat, f"game {i}, decision {d}: seat should be the player about to decide"
            assert torch.equal(data["X"][r], torch.tensor(obs)), \
                f"game {i}, decision {d}: X should be observe(seat) before stepping, from the deciding seat"
            assert abs(data["progress"][r].item() - d / len(steps)) < 1e-6, f"game {i}, decision {d}: progress should be d / total decisions"
            assert data["y"][r].item() == float(seat == winner), \
                f"game {i}, decision {d}: y should say whether the OBSERVED seat won, not player 0"
    assert data["progress"].min() >= 0 and data["progress"].max() < 1, "progress should be in [0, 1)"


def _fake_data(n_games=50, rows_per_game=7):
    game = torch.arange(n_games).repeat_interleave(rows_per_game)
    n = len(game)
    return {"X": torch.randn(n, 3), "y": (game % 2).float(), "game": game,
            "seat": game % 2, "progress": torch.arange(n) % rows_per_game / rows_per_game}


def _check_part2():
    data = _fake_data()
    train_d, test_d = split_by_game(data, test_fraction=0.2)
    assert set(train_d) == set(data) and set(test_d) == set(data), "train and test should have the same keys as data"
    tr, te = set(train_d["game"].tolist()), set(test_d["game"].tolist())
    assert not tr & te, f"games {sorted(tr & te)[:5]} appear in both train and test"
    assert len(te) == 10, f"test_fraction=0.2 of 50 games should give 10 test games, got {len(te)}"
    assert len(train_d["y"]) + len(test_d["y"]) == len(data["y"]), "every row should end up on exactly one side"
    for d in (train_d, test_d):
        assert torch.equal(d["y"], (d["game"] % 2).float()), "rows got out of line: keep every key indexed the same way"
        assert torch.equal(d["X"][:, 0], data["X"][d["game"] * 7 + (d["progress"] * 7).round().long(), 0]), \
            "X rows got out of line with their games"
    assert te != set(range(40, 50)) and te != set(range(10)), "pick the test games at random, not the first or last ones"
    a, b = split_by_game(data, seed=3)[1]["game"], split_by_game(data, seed=3)[1]["game"]
    assert torch.equal(a, b), "the same seed should give the same split"
    assert not torch.equal(a, split_by_game(data, seed=4)[1]["game"]), "different seeds should give different splits"
    torch.manual_seed(123)
    expected = torch.rand(3)
    torch.manual_seed(123)
    split_by_game(data)
    assert torch.equal(torch.rand(3), expected), "split_by_game shouldn't use PyTorch's global random generator"


def _check_part3():
    model = MLP(monopd.OBS_SIZE, hidden=32)
    out = model(torch.randn(5, monopd.OBS_SIZE))
    assert out.shape == (5, 1), f"MLP should output (5, 1), got {tuple(out.shape)}"
    n_params = sum(p.numel() for p in model.parameters())
    expected = (monopd.OBS_SIZE * 32 + 32) + (32 * 32 + 32) + (32 + 1)
    assert n_params == expected, f"MLP({monopd.OBS_SIZE}, hidden=32) should have {expected} parameters, has {n_params}"
    assert any(isinstance(m, nn.ReLU) for m in model.modules()), "MLP should use nn.ReLU"
    g = torch.Generator().manual_seed(0)
    X = torch.randn(600, 20, generator=g)
    y = (X[:, 0] + X[:, 1] > 0).float()
    torch.manual_seed(0)
    model = MLP(20, hidden=32)
    losses = train(model, X, y, epochs=5, lr=1e-2, batch_size=64)
    assert len(losses) == 5 and all(isinstance(l, float) for l in losses), "train should return 5 Python floats"
    assert 0.3 < losses[0] < 0.75, f"first-epoch loss {losses[0]:.3f} looks off: is it the per-row average?"
    assert losses[-1] < losses[0], f"loss should go down while training: {losses}"
    acc = accuracy(model, X, y)
    assert isinstance(acc, float) and acc > 0.9, f"accuracy should be a Python float, and > 0.9 on this easy rule, got {acc}"


def _check_part4():
    class Always(nn.Module):
        def forward(self, x):
            return torch.ones(len(x), 1)

    X = torch.zeros(8, 3)
    y = torch.tensor([1., 0., 1., 1., 0., 0., 1., 0.])
    progress = torch.tensor([0.0, 0.1, 0.3, 0.45, 0.5, 0.74, 0.75, 0.99])
    accs = accuracy_by_stage(Always(), X, y, progress, n_stages=4)
    assert len(accs) == 4 and all(isinstance(a, float) for a in accs), "accuracy_by_stage should return 4 Python floats"
    assert accs == [0.5, 1.0, 0.0, 0.5], f"expected [0.5, 1.0, 0.0, 0.5], got {accs}: check which stage each progress falls in"
    accs = accuracy_by_stage(Always(), X[:2], y[:2], progress[:2], n_stages=4)
    assert accs[0] == 0.5 and all(a != a for a in accs[1:]), "empty stages should be float('nan')"


def check():
    parts = [("1 collect", _check_part1), ("2 split", _check_part2), ("3 model", _check_part3), ("4 stages", _check_part4)]
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
    print("\nAll parts pass. Now: uv run python predictor.py collect, then train")


def _split_by_row(data, test_fraction=0.2, seed=0):
    """For question 1: the toy's row-level split, ignoring games."""
    idx = torch.randperm(len(data["y"]), generator=torch.Generator().manual_seed(seed))
    n_test = round(len(idx) * test_fraction)
    return ({k: v[idx[n_test:]] for k, v in data.items()}, {k: v[idx[:n_test]] for k, v in data.items()})


def run_collect(args):
    data = collect(args.games, every=args.every, seed=args.seed)
    torch.save(data, DATA_PATH)
    n_games = len(set(data["game"].tolist()))
    print(f"{len(data['y'])} rows from {n_games} games ({args.games - n_games} draws skipped), "
          f"{data['y'].mean().item():.1%} labelled 'won'. Saved to {DATA_PATH.name}")


def run_train(args):
    if not DATA_PATH.exists():
        sys.exit(f"No {DATA_PATH.name} yet: run `uv run python predictor.py collect` first.")
    data = torch.load(DATA_PATH)
    torch.manual_seed(args.seed)
    splitter = _split_by_row if args.by_row else split_by_game
    tr, te = splitter(data)
    model = MLP(monopd.OBS_SIZE, hidden=args.hidden)
    losses = train(model, tr["X"], tr["y"], epochs=args.epochs)
    print(f"{'row' if args.by_row else 'game'} split: {len(tr['y'])} train rows, {len(te['y'])} test rows; "
          f"hidden={args.hidden}, {args.epochs} epochs")
    print("  loss per epoch: " + " ".join(f"{l:.3f}" for l in losses))
    print(f"  accuracy: train {accuracy(model, tr['X'], tr['y']):.1%}, test {accuracy(model, te['X'], te['y']):.1%}")
    stages = accuracy_by_stage(model, te["X"], te["y"], te["progress"])
    print("  test accuracy by game quarter: " + "  ".join(f"Q{i + 1} {a:.1%}" for i, a in enumerate(stages)))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("check")
    c = sub.add_parser("collect")
    c.add_argument("--games", type=int, default=5000)
    c.add_argument("--every", type=int, default=8)
    c.add_argument("--seed", type=int, default=0)
    t = sub.add_parser("train")
    t.add_argument("--epochs", type=int, default=8)
    t.add_argument("--hidden", type=int, default=256)
    t.add_argument("--seed", type=int, default=0)
    t.add_argument("--by-row", action="store_true", help="split by row instead of by game (question 1)")
    args = parser.parse_args()
    {"check": lambda a: check(), "collect": run_collect, "train": run_train}[args.cmd](args)
