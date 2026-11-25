"""Actor-critic: REINFORCE plus a value head used as a baseline.

The policy (the actor) still chooses moves. A second output on h (the critic)
predicts the reward the deciding player will get from this state. Each move is
pushed by how much better or worse the game went than predicted.

Training games are split between RandomBot and a frozen copy of your step 4
model (../04_reinforce/reinforce.pt). Evaluation reports both.

Fill in every function marked TODO, in order. Check your work as you go:

    uv run python actor_critic.py check                     # runs the checks for every part
    uv run python actor_critic.py train --out ac            # with the baseline
    uv run python actor_critic.py train --no-baseline --out rf    # the same, as plain REINFORCE
    uv run python actor_critic.py compare ac rf             # both runs on one plot
    uv run python actor_critic.py show --model ac           # a game, with the critic's predictions

See README.md for the walkthrough and questions.
"""

import argparse
import json
import math
import sys
import time
from pathlib import Path

import monopd
import torch
from torch import nn
from torch.distributions import Categorical

HERE = Path(__file__).parent
sys.path.insert(0, str(HERE.parent / "03_policy"))
from policy import PolicyNet, evaluate  # noqa: E402  (your step 3 code)

TRAIN_SEED_OFFSET = 2_000_000
FROZEN_PATH = HERE.parent / "04_reinforce" / "reinforce.pt"


# ===========================================================================
# Provided helpers you'll use. You don't need to edit these.
# ===========================================================================

def random_opponent(seed):
    """An opponent function for one game: RandomBot(seed).choose."""
    return monopd.RandomBot(seed=seed).choose


def frozen_opponent(frozen):
    """An opponent function that samples moves from a frozen PolicyNet, without gradients."""
    def move(game):
        legal = game.legal_actions()
        with torch.no_grad():
            logits = frozen(torch.tensor(game.observe(game.current_player())), torch.tensor(legal))
        return legal[Categorical(logits=logits).sample().item()]
    return move


class _PolicyOnly(nn.Module):
    """Makes an ActorCritic look like a PolicyNet (logits only), for step 3's evaluate."""

    def __init__(self, model):
        super().__init__()
        self.model = model

    def forward(self, obs, legal):
        return self.model(obs, legal)[0]


def evaluate_both(model, frozen, n_games):
    """(win rate vs RandomBot, win rate vs the frozen model) over n_games each, seeds 0..n-1.

    Leaves the model in eval mode, like step 3's evaluate.
    """
    vs_random = evaluate(_PolicyOnly(model), n_games, seed=0)
    model.eval()
    opponent = frozen_opponent(frozen)
    wins = 0
    with torch.no_grad():
        for k in range(n_games):
            g, seat = monopd.Game(num_players=2, seed=k), k % 2
            while not g.is_over():
                if g.current_player() == seat:
                    legal = g.legal_actions()
                    logits, _ = model(torch.tensor(g.observe(seat)), torch.tensor(legal))
                    g.step(legal[Categorical(logits=logits).sample().item()])
                else:
                    g.step(opponent(g))
            wins += g.winner == seat
    return vs_random, wins / n_games


# ===========================================================================
# Part 1: the model.
# ===========================================================================

class ActorCritic(PolicyNet):
    """Your PolicyNet plus a value head on h.

    forward(obs, legal) returns (logits, value):
      logits: float [K], one per legal action, as before
      value:  a 0-d float tensor in [-1, 1], the predicted reward for the
              player observing `obs`. It depends on the state only, not on `legal`.
    """

    def __init__(self, obs_size=monopd.OBS_SIZE, num_actions=monopd.NUM_ACTIONS, hidden=256):
        super().__init__(obs_size, num_actions, hidden)
        self.value_head = nn.Sequential(
            nn.Linear(obs_size, hidden),
            nn.ReLU(),
            nn.Linear(hidden, hidden),
            nn.ReLU(),
            nn.Linear(hidden, hidden),
            nn.ReLU(),
            nn.Linear(hidden, 1),
            nn.Tanh(),
        )

    def forward(self, obs, legal):
        logits = super().forward(obs, legal)
        value = self.value_head(obs).squeeze()
        return (logits, value)


# ===========================================================================
# Part 2: play one game.
# ===========================================================================

def play_episode(model, seed, model_seat, opponent):
    """Like step 4's play_episode, against `opponent` and keeping the critic's predictions.

    The game is monopd.Game(num_players=2, seed=seed). `opponent` is a function
    game -> action index that makes every move for the other seat. At each of
    the model's decisions with 2+ legal actions, sample from the logits (global
    generator) and keep the chosen move's log-probability and the value.
    Forced moves are played without calling the model.

    Returns (log_probs, values, reward): two lists of 0-d tensors, one entry
    per non-forced model decision, and the model seat's reward as a float.
    """
    raise NotImplementedError("TODO part 2")


# ===========================================================================
# Part 3: the loss.
# ===========================================================================

def actor_critic_loss(episodes, value_coef=0.5, baseline=True):
    """Returns (total, policy_loss, value_loss), three 0-d tensors.

    episodes: a list of (log_probs, values, reward) from play_episode.

    policy_loss: step 4's loss, but each move's log-probability is weighted by
        its ADVANTAGE instead of the reward: reward minus that decision's value,
        with the value treated as a constant here. With baseline=False the
        weight is just the reward, as in step 4. Averaged over episodes.
    value_loss: the mean, over every recorded decision in the batch, of
        (value - reward) ** 2.
    total: policy_loss + value_coef * value_loss.
    """
    raise NotImplementedError("TODO part 3")


# ===========================================================================
# Part 4: the training loop.
# ===========================================================================

def train(model, frozen, n_updates, games_per_update=32, lr=1e-3, seed=0, eval_every=20, eval_games=300,
          baseline=True, value_coef=0.5):
    """Train with Adam on games against a mix of opponents. Returns the history.

    Number training games 0, 1, 2, ... across the run. Game j uses seed
    TRAIN_SEED_OFFSET + seed + j and model_seat j % 2, and its opponent is
    random_opponent(that seed) when j % 4 < 2, else frozen_opponent(frozen).

    Each update plays games_per_update games and takes one optimizer step on
    actor_critic_loss(..., value_coef, baseline). Measure with
    evaluate_both(model, frozen, eval_games) before the first update and after
    every eval_every updates, and leave the model ready to train again.

    Returns a list of (training games so far, win rate vs random, win rate vs frozen).
    """
    raise NotImplementedError("TODO part 4")


# ===========================================================================
# Below here is provided: checks and experiments. You don't need to edit it.
# ===========================================================================

class _Fixed(nn.Module):
    """A stand-in model: all probability on the first legal action, and a value that counts calls."""

    def __init__(self):
        super().__init__()
        self.calls = []

    def forward(self, obs, legal):
        self.calls.append(obs.clone())
        out = torch.full((len(legal),), -1e9)
        out[0] = 0.0
        return out, torch.tensor(len(self.calls) / 1000.0)


def _check_part1():
    torch.manual_seed(0)
    m = ActorCritic(obs_size=20, num_actions=50, hidden=16)
    assert isinstance(m, PolicyNet), "ActorCritic should build on your PolicyNet"
    obs, legal = torch.randn(20), torch.tensor([3, 7, 0, 41])
    out = m(obs, legal)
    assert isinstance(out, tuple) and len(out) == 2, "forward should return (logits, value)"
    logits, value = out
    assert logits.shape == (4,), f"logits should have shape (4,), got {tuple(logits.shape)}"
    assert torch.is_tensor(value) and value.dim() == 0, f"value should be a 0-d tensor, got shape {tuple(value.shape)}"
    _, v2 = m(obs, torch.tensor([9]))
    assert torch.allclose(value, v2), "the value should depend on the state only, not on the legal actions"
    with torch.no_grad():
        for p in m.parameters():
            p.mul_(50)
    vals = torch.stack([m(torch.randn(20) * 10, legal)[1] for _ in range(50)])
    assert vals.abs().max() <= 1.0 + 1e-6, f"the value should stay in [-1, 1] even for extreme weights; got {vals.abs().max().item():.2f}"
    torch.manual_seed(0)
    m = ActorCritic(obs_size=20, num_actions=50, hidden=16)
    names = [n for n, _ in m.named_parameters()]
    m(obs, legal)[1].backward()
    grads = {n for n, p in m.named_parameters() if p.grad is not None and p.grad.abs().sum() > 0}
    assert grads, "backward() from the value should reach the model's weights"
    assert len(names) > len(list(PolicyNet(obs_size=20, num_actions=50, hidden=16).parameters())), \
        "ActorCritic should add parameters for the value head"


def _replay(seed, model_seat):
    """Observations at the model's non-forced decisions, when it always plays legal[0] vs RandomBot."""
    g, bot = monopd.Game(num_players=2, seed=seed), monopd.RandomBot(seed=seed)
    seen, opp_moves = [], 0
    while not g.is_over():
        if g.current_player() == model_seat:
            legal = g.legal_actions()
            if len(legal) > 1:
                seen.append(torch.tensor(g.observe(model_seat)))
            g.step(legal[0])
        else:
            opp_moves += 1
            g.step(bot.choose(g))
    return seen, opp_moves, g.returns()[model_seat]


def _check_part2():
    for seed, seat in ((3, 0), (4, 1)):
        fixed = _Fixed()
        bot = monopd.RandomBot(seed=seed)
        opp_calls = []
        out = play_episode(fixed, seed, seat, lambda g: (opp_calls.append(g.current_player()), bot.choose(g))[1])
        assert isinstance(out, tuple) and len(out) == 3, "play_episode should return (log_probs, values, reward)"
        log_probs, values, reward = out
        seen, opp_moves, expected = _replay(seed, seat)
        assert len(fixed.calls) == len(seen), \
            f"game {seed}: the model should be called {len(seen)} times (its non-forced decisions), got {len(fixed.calls)}"
        assert all(torch.equal(a, b) for a, b in zip(fixed.calls, seen)), "the model should see observe(model_seat)"
        assert len(opp_calls) == opp_moves and all(p != seat for p in opp_calls), \
            f"game {seed}: the opponent should make every move for the other seat ({opp_moves}), got {len(opp_calls)}"
        assert isinstance(reward, float) and reward == expected, f"game {seed}: reward should be {expected}, got {reward!r}"
        assert len(log_probs) == len(values) == len(seen), "one log-prob and one value per non-forced decision"
        assert all(abs(lp.item()) < 1e-6 for lp in log_probs), "log-probs should be of the chosen moves (here all log 1 = 0)"
        assert [round(v.item() * 1000) for v in values] == list(range(1, len(seen) + 1)), \
            "values should be the model's value outputs, in decision order"
    torch.manual_seed(0)
    m = ActorCritic(hidden=8)
    lps, vals, _ = play_episode(m, 5, 0, random_opponent(5))
    assert all(lp.requires_grad for lp in lps) and all(v.requires_grad for v in vals), "log-probs and values should carry gradients"


def _check_part3():
    theta = torch.zeros(3, requires_grad=True)
    d = Categorical(logits=theta)
    v = torch.tensor([0.5, -0.2, 0.1, 0.3], requires_grad=True)
    a, b = torch.tensor(0), torch.tensor(1)
    # 3 episodes (one empty), 4 decisions: (moves, value indices, reward)
    spec = [([a], [0], 1.0), ([b, b, a], [1, 2, 3], -1.0), ([], [], 1.0)]
    episodes = [([d.log_prob(x) for x in mv], [v[i] for i in vi], r) for mv, vi, r in spec]
    out = actor_critic_loss(episodes, value_coef=0.5, baseline=True)
    assert isinstance(out, tuple) and len(out) == 3, "actor_critic_loss should return (total, policy_loss, value_loss)"
    total, pl, vl = out
    assert all(torch.is_tensor(x) and x.dim() == 0 for x in out), "all three should be 0-d tensors"
    lp = math.log(1 / 3)
    vv = v.tolist()
    exp_pl = sum(-sum((r - vv[i]) * lp for i in vi) for _, vi, r in spec) / 3
    n_dec = sum(len(vi) for _, vi, _ in spec)
    exp_vl = sum((vv[i] - r) ** 2 for _, vi, r in spec for i in vi) / n_dec
    assert abs(pl.item() - exp_pl) < 1e-5, \
        f"policy_loss should be {exp_pl:.4f}, got {pl.item():.4f} (advantage = reward - value; average over episodes)"
    assert abs(vl.item() - exp_vl) < 1e-5, \
        f"value_loss should be {exp_vl:.4f}, got {vl.item():.4f} (mean over all {n_dec} decisions of (value - reward)^2)"
    assert abs(total.item() - (exp_pl + 0.5 * exp_vl)) < 1e-5, "total should be policy_loss + value_coef * value_loss"
    total.backward()
    rv = [1.0, -1.0, -1.0, -1.0]
    expected_vgrad = torch.tensor([2 * (vv[i] - rv[i]) * 0.5 / n_dec for i in range(4)])
    assert torch.allclose(v.grad, expected_vgrad, atol=1e-5), \
        "the value should get gradient only from value_loss: treat it as a constant inside the advantage"
    theta2 = torch.zeros(3, requires_grad=True)
    d2 = Categorical(logits=theta2)
    eps2 = [([d2.log_prob(a)], [torch.tensor(0.9)], 1.0), ([d2.log_prob(b), d2.log_prob(b)], [torch.tensor(0.3)] * 2, -1.0)]
    _, pl2, _ = actor_critic_loss(eps2, baseline=False)
    assert abs(pl2.item() - (-lp + 2 * lp) / 2) < 1e-5, "with baseline=False, the policy loss should weight log-probs by the reward alone"


def _check_part4():
    global play_episode, evaluate_both
    real_play, real_eval = play_episode, evaluate_both
    played, evals = [], []

    def spy_play(model, seed, model_seat, opponent):
        kind = "frozen" if opponent.__qualname__.startswith("frozen_opponent") else "random"
        played.append((seed, model_seat, kind))
        return real_play(model, seed, model_seat, opponent)

    play_episode = spy_play
    evaluate_both = lambda model, frozen, n: (evals.append(n), real_eval(model, frozen, n))[1]
    try:
        torch.manual_seed(0)
        m, frozen = ActorCritic(hidden=16), PolicyNet(hidden=16)
        before = [p.detach().clone() for p in m.parameters()]
        torch.manual_seed(1)
        hist = train(m, frozen, n_updates=2, games_per_update=4, lr=1e-2, seed=0, eval_every=1, eval_games=2)
        assert isinstance(hist, list) and len(hist) == 3, f"with 2 updates and eval_every=1, expect 3 evaluations, got {hist!r}"
        assert [h[0] for h in hist] == [0, 4, 8], f"games at each evaluation should be [0, 4, 8], got {[h[0] for h in hist]}"
        assert all(len(h) == 3 and all(isinstance(x, float) for x in h[1:]) for h in hist), \
            "each entry should be (games, win rate vs random, win rate vs frozen)"
        o = TRAIN_SEED_OFFSET
        want = [(o + j, j % 2, "random" if j % 4 < 2 else "frozen") for j in range(8)]
        assert played == want, f"training games should be {[(s - o, st, k) for s, st, k in want]}, got {[(s - o, st, k) for s, st, k in played]}"
        assert evals == [2, 2, 2], f"evaluate_both should be called with eval_games each time, got {evals}"
        assert any(not torch.equal(a, b) for a, b in zip(before, m.parameters())), "training should change the weights"
        assert m.training, "after train returns, the model should be back in training mode"
    finally:
        play_episode, evaluate_both = real_play, real_eval


def check():
    parts = [("1 model", _check_part1), ("2 episode", _check_part2), ("3 loss", _check_part3), ("4 train", _check_part4)]
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
    print("\nAll parts pass. Now: uv run python actor_critic.py train --out ac, and --no-baseline --out rf")


def _load_frozen(path):
    if not Path(path).exists():
        sys.exit(f"No frozen model at {path}: run step 4's `uv run python reinforce.py train` first.")
    frozen = PolicyNet()
    frozen.load_state_dict(torch.load(path))
    frozen.eval()
    return frozen


SURF, INK, INK2, GRID = "#fcfcfb", "#0b0b0b", "#52514e", "#e8e7e3"
SERIES = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100"]


def _axes(ax, xlabel):
    ax.set_facecolor(SURF)
    ax.axhline(50, color=INK2, linewidth=1, linestyle=(0, (3, 3)))
    ax.set_ylim(0, 100)
    ax.set_xlabel(xlabel, color=INK2)
    ax.tick_params(colors=INK2)
    for s in ("top", "right"):
        ax.spines[s].set_visible(False)
    ax.yaxis.grid(True, color=GRID, linewidth=0.8)
    ax.set_axisbelow(True)


def _plot(runs, path, title):
    """runs: list of (label, history). Two panels: vs RandomBot and vs the frozen model."""
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    fig, axes = plt.subplots(1, 2, figsize=(11, 4.5), facecolor=SURF, sharey=True)
    for col, (ax, name) in enumerate(zip(axes, ("vs RandomBot", "vs frozen step 4 model"))):
        _axes(ax, "training games")
        for i, (label, hist) in enumerate(runs):
            xs = [h[0] for h in hist]
            ys = [100 * h[1 + col] for h in hist]
            ax.plot(xs, ys, color=SERIES[i % len(SERIES)], linewidth=2, marker="o", markersize=5,
                    markeredgecolor=SURF, markeredgewidth=1.5, label=label)
        ax.set_title(name, loc="left", fontsize=10, color=INK)
    axes[0].set_ylabel("win rate (%)", color=INK2)
    if len(runs) > 1:
        axes[0].legend(frameon=False, loc="lower right", labelcolor=INK)
    fig.suptitle(title, x=0.01, ha="left", fontsize=11, color=INK)
    fig.tight_layout()
    fig.savefig(path, dpi=130, facecolor=SURF)


def run_train(args):
    frozen = _load_frozen(args.frozen)
    torch.manual_seed(args.model_seed)
    model = ActorCritic()
    for m in model.modules():
        if isinstance(m, nn.Embedding):
            nn.init.normal_(m.weight, std=0.01)
    torch.manual_seed(args.seed)
    t = time.time()
    hist = train(model, frozen, args.updates, games_per_update=args.batch, lr=args.lr, seed=args.seed,
                 eval_every=args.eval_every, eval_games=args.eval_games, baseline=not args.no_baseline)
    for g, wr, wf in hist:
        print(f"  after {g:6d} training games: vs random {wr:.1%}   vs frozen {wf:.1%}")
    kind = "REINFORCE (no baseline)" if args.no_baseline else "actor-critic"
    name = f"{kind}, lr={args.lr}, batch={args.batch}, seed={args.seed}"
    out = HERE / args.out
    _plot([(kind, hist)], out.with_suffix(".png"), name)
    torch.save(model.state_dict(), out.with_suffix(".pt"))
    out.with_suffix(".json").write_text(json.dumps({"name": name, "history": hist}))
    print(f"{name}: {time.time() - t:.0f}s. Saved {out.name}.png, .pt and .json")


def run_compare(args):
    runs = []
    for n in args.runs:
        d = json.loads((HERE / f"{n}.json").read_text())
        runs.append((f"{n}: {d['name']}", d["history"]))
    path = HERE / ("compare_" + "_".join(args.runs) + ".png")
    _plot(runs, path, "comparison")
    print(f"Plot: {path.name}")


def run_show(args):
    model = ActorCritic()
    model.load_state_dict(torch.load(HERE / f"{args.model}.pt"))
    model.eval()
    frozen = _load_frozen(args.frozen) if args.vs == "frozen" else None
    torch.manual_seed(args.seed)
    g = monopd.Game(num_players=2, seed=args.seed)
    opponent = frozen_opponent(frozen) if frozen is not None else random_opponent(args.seed)
    print(f"{args.model} as seat 0 vs {args.vs}. value = the critic's predicted reward for seat 0 (-1 lose .. +1 win)\n")
    with torch.no_grad():
        while not g.is_over():
            if g.current_player() != 0:
                g.step(opponent(g))
                continue
            legal = g.legal_actions()
            logits, value = model(torch.tensor(g.observe(0)), torch.tensor(legal))
            a = legal[Categorical(logits=logits).sample().item()]
            if len(legal) > 1:
                bar = "#" * round((value.item() + 1) * 10)
                print(f"turn {g.turn:3}  value {value.item():+.2f} {bar:20}  {monopd.action_name(a)}")
            g.step(a)
    print(f"\nResult for seat 0: {g.returns()[0]:+.0f}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("check")
    t = sub.add_parser("train")
    t.add_argument("--no-baseline", action="store_true", help="plain REINFORCE: weight by the reward, not the advantage")
    t.add_argument("--updates", type=int, default=200)
    t.add_argument("--batch", type=int, default=32)
    t.add_argument("--lr", type=float, default=1e-3)
    t.add_argument("--seed", type=int, default=0)
    t.add_argument("--model-seed", type=int, default=0)
    t.add_argument("--eval-every", type=int, default=20)
    t.add_argument("--eval-games", type=int, default=300)
    t.add_argument("--frozen", default=str(FROZEN_PATH))
    t.add_argument("--out", required=True, help="name for the .png, .pt and .json files")
    c = sub.add_parser("compare")
    c.add_argument("runs", nargs="+", help="names given to --out")
    s = sub.add_parser("show")
    s.add_argument("--model", required=True)
    s.add_argument("--vs", choices=["random", "frozen"], default="frozen")
    s.add_argument("--seed", type=int, default=0)
    s.add_argument("--frozen", default=str(FROZEN_PATH))
    args = parser.parse_args()
    {"check": lambda a: check(), "train": run_train, "compare": run_compare, "show": run_show}[args.cmd](args)
