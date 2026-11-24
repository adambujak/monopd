"""REINFORCE: the policy learns to beat RandomBot from wins and losses alone.

Uses your PolicyNet and evaluate from step 3 (../03_policy/policy.py).

Fill in every function marked TODO, in order. Check your work as you go:

    uv run python reinforce.py check            # runs the checks for every part
    uv run python reinforce.py train            # train, plot win rate, save policy.pt
    uv run python reinforce.py show             # one game by the trained policy

See README.md for the walkthrough and questions.
"""

import argparse
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

TRAIN_SEED_OFFSET = 1_000_000   # training games use seeds far from the evaluation games' 0, 1, 2, ...


# ===========================================================================
# Part 1: play one game, keeping what learning needs.
# ===========================================================================


def choose(model: PolicyNet, game: monopd.Game) -> tuple[int, torch.Tensor]:
    """ return action and log prob """
    current_player = game.current_player()

    if current_player is None:
        raise RuntimeError("attempted to choose on finished game")

    obs = torch.tensor(game.observe(current_player))
    available_actions = torch.tensor(game.legal_actions())

    scores = model(obs, available_actions)
    sampler = Categorical(logits=scores)
    available_actions_pos = sampler.sample()

    action = int(available_actions[available_actions_pos])
    return (action, sampler.log_prob(available_actions_pos))


def play_episode(model, seed, model_seat):
    """Play one game, `model` in `model_seat` against RandomBot, WITH gradients.

    Same game and bot as step 3's play_game: monopd.Game(num_players=2, seed=seed)
    and monopd.RandomBot(seed=seed). At each of the model's decisions:
      - if there is exactly one legal action, play it without calling the model
      - otherwise score the legal actions with `model`, sample with Categorical
        (PyTorch's global generator), play the sampled action, and keep the
        log-probability of the choice

    Returns (log_probs, reward): a list with one 0-d tensor per non-forced
    model decision, in order, and the model seat's reward as a Python float.
    """

    game = monopd.Game(num_players=2, seed=seed)
    random_bot = monopd.RandomBot(seed=seed)
    log_probs = []

    while not game.is_over():
        if game.current_player() == model_seat:
            legal_actions = game.legal_actions()
            if len(legal_actions) == 1:
                action = legal_actions[0]
            else:
                action, log_prob = choose(model, game)
                log_probs.append(log_prob)
        else:
            action = random_bot.choose(game)
        game.step(action)

    reward = game.returns()[model_seat]
    return (log_probs, reward)


# ===========================================================================
# Part 2: the loss.
# ===========================================================================

def reinforce_loss(episodes):
    """The REINFORCE loss for a batch of games.

    episodes: a list of (log_probs, reward) pairs from play_episode.
    Returns a 0-d tensor. Average over episodes, not over decisions. Lowering
    it should make the moves of won games more likely and those of lost games
    less likely. An episode can have no log-probs at all.
    """

    loss = torch.tensor([0.0])
    for log_probs, r in episodes:
        if len(log_probs) == 0:
            continue
        log_probs_tensor = torch.stack(log_probs)
        game_loss = log_probs_tensor.sum() * r * -1.0
        loss += game_loss

    return loss.squeeze() / len(episodes)


# ===========================================================================
# Part 3: the training loop.
# ===========================================================================

def evaluate_model(model, eval_games):
    with torch.no_grad():
        model.eval()
        win_rate = evaluate(model, eval_games, seed=0)
        model.train()

        return win_rate


def train(model, n_updates, games_per_update=32, lr=1e-3, seed=0, eval_every=20, eval_games=400):
    """Train `model` with REINFORCE and Adam. Returns the evaluation history.

    Each update plays `games_per_update` games, computes the loss and takes one
    optimizer step. Number training games 0, 1, 2, ... across the whole run;
    game j uses seed TRAIN_SEED_OFFSET + seed + j and model_seat j % 2.

    Measure with step 3's evaluate(model, eval_games, seed=0) before the first
    update and after every `eval_every` updates, and leave the model ready to
    train again afterwards.

    Returns a list of (training games played so far, win rate) pairs.
    """
    results = []
    g = 0

    win_rate = evaluate_model(model, eval_games)
    results.append((g, win_rate))

    optimizer = torch.optim.Adam(model.parameters(), lr=lr)
    for i in range(n_updates):
        episodes = []
        optimizer.zero_grad()
        for _ in range(games_per_update):
            game_seed = g + seed + TRAIN_SEED_OFFSET
            episodes.append(play_episode(model, game_seed, g % 2))
            g += 1


        loss = reinforce_loss(episodes)
        loss.backward()
        optimizer.step()

        if (i+1) % eval_every == 0:
            win_rate = evaluate_model(model, eval_games)
            print(f"game: {g}, {win_rate}")
            results.append((g, win_rate))


    return results


# ===========================================================================
# Below here is provided: checks and experiments. You don't need to edit it.
# ===========================================================================

class _Fixed(nn.Module):
    """A stand-in model: all the probability on the first (or last) legal action."""

    def __init__(self, last=False):
        super().__init__()
        self.last = last
        self.calls = []

    def forward(self, obs, legal):
        self.calls.append((obs.clone(), legal.clone()))
        out = torch.full((len(legal),), -1e9)
        out[-1 if self.last else 0] = 0.0
        return out


def _replay_fixed(seed, model_seat, last):
    g, bot = monopd.Game(num_players=2, seed=seed), monopd.RandomBot(seed=seed)
    calls = []
    while not g.is_over():
        if g.current_player() == model_seat:
            legal = g.legal_actions()
            if len(legal) > 1:
                calls.append(torch.tensor(g.observe(model_seat)))
            g.step(legal[-1] if last else legal[0])
        else:
            g.step(bot.choose(g))
    return calls, g.returns()[model_seat]


def _check_part1():
    for seed, seat, last in ((3, 0, False), (4, 1, True)):
        fixed = _Fixed(last)
        out = play_episode(fixed, seed, seat)
        assert isinstance(out, tuple) and len(out) == 2, "play_episode should return (log_probs, reward)"
        log_probs, reward = out
        calls, expected = _replay_fixed(seed, seat, last)
        assert isinstance(reward, float) and reward == expected, f"game {seed}: reward should be {expected}, got {reward!r}"
        assert len(fixed.calls) == len(calls), \
            f"game {seed}, seat {seat}: the model should be called {len(calls)} times (its decisions with 2+ legal actions), got {len(fixed.calls)}"
        assert all(torch.equal(o, c) for (o, _), c in zip(fixed.calls, calls)), "the model should see observe(model_seat)"
        assert isinstance(log_probs, list) and len(log_probs) == len(calls), \
            f"expected one log-prob per non-forced decision ({len(calls)}), got {len(log_probs) if isinstance(log_probs, list) else type(log_probs).__name__}"
        assert all(torch.is_tensor(lp) and lp.dim() == 0 for lp in log_probs), "each log-prob should be a 0-d tensor"
        assert all(abs(lp.item()) < 1e-6 for lp in log_probs), \
            "with all probability on one action, every log-prob should be log(1) = 0: are these log-probabilities of the chosen actions?"
    torch.manual_seed(0)
    m = PolicyNet(hidden=8)
    log_probs, _ = play_episode(m, 5, 0)
    assert all(lp.requires_grad for lp in log_probs), "log-probs should carry gradients back to the model (no torch.no_grad here)"
    assert all(lp.item() <= 1e-6 for lp in log_probs), "log-probabilities can't be positive"
    torch.stack(log_probs).sum().backward()
    assert any(p.grad is not None and p.grad.abs().sum() > 0 for p in m.parameters()), \
        "backward() should reach the model's weights"


def _check_part2():
    theta = torch.zeros(3, requires_grad=True)
    d = Categorical(logits=theta)
    a, b = torch.tensor(0), torch.tensor(1)
    episodes = [([d.log_prob(a)], 1.0), ([d.log_prob(b), d.log_prob(b)], -1.0), ([], 1.0), ([d.log_prob(a)], 0.0)]
    loss = reinforce_loss(episodes)
    assert torch.is_tensor(loss) and loss.dim() == 0, "reinforce_loss should return a 0-d tensor"
    assert loss.requires_grad, "the loss should carry gradients"
    lp = math.log(1 / 3)
    expected = (-1.0 * lp + 1.0 * 2 * lp + 0.0 + 0.0) / 4
    assert abs(loss.item() - expected) < 1e-5, \
        f"for this batch the loss should be {expected:.4f}, got {loss.item():.4f} (average over the 4 episodes, including the empty one)"
    loss.backward()
    with torch.no_grad():
        p = torch.softmax(theta - 0.1 * theta.grad, 0)
    assert p[0] > 1 / 3 > p[1], "a gradient step should raise the won game's move (0) and lower the lost game's move (1)"


def _check_part3():
    global play_episode, evaluate
    real_play, real_eval = play_episode, evaluate
    played, evals = [], []
    play_episode = lambda model, seed, model_seat: (played.append((seed, model_seat)), real_play(model, seed, model_seat))[1]
    evaluate = lambda model, n_games, seed=0: (evals.append((n_games, seed)), real_eval(model, n_games, seed=seed))[1]
    try:
        _check_part3_run(played, evals)
    finally:
        play_episode, evaluate = real_play, real_eval


def _check_part3_run(played, evals):
    torch.manual_seed(0)
    m = PolicyNet(hidden=16)
    before = [p.detach().clone() for p in m.parameters()]
    torch.manual_seed(1)
    hist = train(m, n_updates=2, games_per_update=2, lr=1e-2, seed=0, eval_every=1, eval_games=2)
    assert isinstance(hist, list) and len(hist) == 3, \
        f"with 2 updates and eval_every=1, expect 3 evaluations (before, after 1, after 2), got {len(hist) if isinstance(hist, list) else hist!r}"
    assert [h[0] for h in hist] == [0, 2, 4], f"games played at each evaluation should be [0, 2, 4], got {[h[0] for h in hist]}"
    assert all(isinstance(h[1], float) and 0 <= h[1] <= 1 for h in hist), "win rates should be floats in [0, 1]"
    assert any(not torch.equal(a, b) for a, b in zip(before, m.parameters())), "training should change the weights"
    assert m.training, "after train returns, the model should be back in training mode"
    o = TRAIN_SEED_OFFSET
    assert played == [(o, 0), (o + 1, 1), (o + 2, 0), (o + 3, 1)], \
        f"training games should use seeds TRAIN_SEED_OFFSET + seed + j and seats j % 2, got {[(sd - o, st) for sd, st in played]} (seed - offset, seat)"
    assert evals == [(2, 0)] * 3, f"evaluations should be evaluate(model, eval_games, seed=0), got {evals}"
    played.clear()
    evals.clear()
    torch.manual_seed(0)
    m2 = PolicyNet(hidden=16)
    torch.manual_seed(1)
    hist2 = train(m2, n_updates=2, games_per_update=2, lr=1e-2, seed=0, eval_every=1, eval_games=2)
    assert hist2 == hist and all(torch.equal(a, b) for a, b in zip(m.parameters(), m2.parameters())), \
        "the same seeds should give the same run"


def check():
    parts = [("1 episode", _check_part1), ("2 loss", _check_part2), ("3 train", _check_part3)]
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
    print("\nAll parts pass. Now: uv run python reinforce.py train")


def _make_model(model_seed, emb_std):
    torch.manual_seed(model_seed)
    model = PolicyNet()
    if emb_std > 0:
        for m in model.modules():
            if isinstance(m, nn.Embedding):
                nn.init.normal_(m.weight, std=emb_std)
    return model


def _plot(hist, path, title, eval_games):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    games = [g for g, _ in hist]
    wr = [100 * w for _, w in hist]
    band = [196 * math.sqrt(max(w * (1 - w), 1e-9) / eval_games) for _, w in hist]
    surf, ink, ink2 = "#fcfcfb", "#0b0b0b", "#52514e"
    fig, ax = plt.subplots(figsize=(8, 4.5), facecolor=surf)
    ax.set_facecolor(surf)
    ax.fill_between(games, [w - b for w, b in zip(wr, band)], [w + b for w, b in zip(wr, band)], color="#86b6ef", alpha=0.35, linewidth=0)
    ax.plot(games, wr, color="#2a78d6", linewidth=2, marker="o", markersize=5, markeredgecolor=surf, markeredgewidth=1.5)
    ax.axhline(50, color=ink2, linewidth=1, linestyle=(0, (3, 3)))
    ax.text(games[-1], 51, "random bot level", ha="right", va="bottom", fontsize=8, color=ink2)
    ax.set_ylim(0, 100)
    ax.set_xlabel("training games", color=ink2)
    ax.set_ylabel(f"win rate vs RandomBot (%), {eval_games} eval games", color=ink2)
    ax.tick_params(colors=ink2)
    for s in ("top", "right"):
        ax.spines[s].set_visible(False)
    ax.yaxis.grid(True, color="#e8e7e3", linewidth=0.8)
    ax.set_axisbelow(True)
    ax.set_title(title + "  (band: 95% interval)", loc="left", fontsize=11, color=ink)
    fig.tight_layout()
    fig.savefig(path, dpi=130, facecolor=surf)


def run_train(args):
    model = _make_model(args.model_seed, args.emb_std)
    torch.manual_seed(args.seed)
    t = time.time()
    hist = train(model, args.updates, games_per_update=args.batch, lr=args.lr, seed=args.seed,
                 eval_every=args.eval_every, eval_games=args.eval_games)
    for g, w in hist:
        print(f"  after {g:6d} training games: win rate {w:.1%}")
    name = f"lr={args.lr}, batch={args.batch}, emb-std={args.emb_std}, seed={args.seed}"
    out = HERE / (args.out or "reinforce")
    _plot(hist, out.with_suffix(".png"), name, args.eval_games)
    torch.save(model.state_dict(), out.with_suffix(".pt"))
    print(f"{name}: {time.time() - t:.0f}s. Plot: {out.with_suffix('.png').name}, model: {out.with_suffix('.pt').name}")


def run_show(args):
    model = PolicyNet()
    path = HERE / f"{args.model or 'reinforce'}.pt"
    model.load_state_dict(torch.load(path))
    model.eval()
    torch.manual_seed(args.seed)
    g, bot = monopd.Game(num_players=2, seed=args.seed), monopd.RandomBot(seed=args.seed)
    shown = 0
    with torch.no_grad():
        while not g.is_over() and shown < args.moves:
            if g.current_player() != 0:
                g.step(bot.choose(g))
                continue
            legal = g.legal_actions()
            if len(legal) == 1:
                g.step(legal[0])
                continue
            probs = torch.softmax(model(torch.tensor(g.observe(0)), torch.tensor(legal)), 0)
            pos = Categorical(probs=probs).sample().item()
            top = sorted(zip(probs.tolist(), legal), reverse=True)[:3]
            print(f"turn {g.turn:3}  chose {monopd.action_name(legal[pos])!r}")
            print("           top: " + ";  ".join(f"{p:.2f} {monopd.action_name(i)}" for p, i in top) + f"   ({len(legal)} legal)")
            g.step(legal[pos])
            shown += 1
    print(f"\n({path.name}, {shown} decisions shown)")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("check")
    t = sub.add_parser("train")
    t.add_argument("--updates", type=int, default=200)
    t.add_argument("--batch", type=int, default=32, help="games per update")
    t.add_argument("--lr", type=float, default=1e-3)
    t.add_argument("--emb-std", type=float, default=0.01, help="initial embedding std; 0 keeps PyTorch's default")
    t.add_argument("--seed", type=int, default=0, help="seed for training games and sampling")
    t.add_argument("--model-seed", type=int, default=0, help="seed for the initial weights")
    t.add_argument("--eval-every", type=int, default=20, help="updates between evaluations")
    t.add_argument("--eval-games", type=int, default=400)
    t.add_argument("--out", default=None, help="name for the .png and .pt files (default: reinforce)")
    s = sub.add_parser("show")
    s.add_argument("--model", default=None, help="name of a saved .pt (default: reinforce)")
    s.add_argument("--seed", type=int, default=0)
    s.add_argument("--moves", type=int, default=15)
    args = parser.parse_args()
    {"check": lambda a: check(), "train": run_train, "show": run_show}[args.cmd](args)
