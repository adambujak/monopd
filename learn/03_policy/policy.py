"""Policy network, untrained: a model that makes every decision in a full game.

The two-column model from PLAN.md:

    h      = state_net(obs)          [OBS_SIZE] -> [hidden]
    E      = embedding(legal)        [K]        -> [K, hidden]
    scores = E @ h                   [K]        one score (logit) per legal action

Fill in every function marked TODO, in order. Check your work as you go:

    uv run python policy.py check                  # runs the checks for every part
    uv run python policy.py play                   # untrained model vs RandomBot, 1000 games
    uv run python policy.py play --emb-std 0.01    # the same with small initial embeddings
    uv run python policy.py show                   # one game, with the model's choices and probabilities

See README.md for the walkthrough, hints and questions.
"""

import argparse
import math
import time

import monopd
import torch
from torch import nn
from torch.distributions import Categorical


# ===========================================================================
# Part 1: the model.
# ===========================================================================

class PolicyNet(nn.Module):
    """Scores each legal action for one state.

    state column:  obs [obs_size] -> h [hidden], an MLP like step 2's (no final ReLU)
    action column: nn.Embedding(num_actions, hidden), looked up by action index
    """

    def __init__(self, obs_size=monopd.OBS_SIZE, num_actions=monopd.NUM_ACTIONS, hidden=256):
        super().__init__()
        self.mlp = nn.Sequential(
            nn.Linear(obs_size, hidden),
            nn.ReLU(),
            nn.Linear(hidden, hidden),
            nn.ReLU(),
            nn.Linear(hidden, hidden)
        )
        self.actions = nn.Embedding(num_actions, hidden)


    def forward(self, obs, legal):
        """obs: float [obs_size], legal: int64 [K] action indices.
        Returns logits: float [K], one per legal action, in the same order as `legal`.
        """
        h = self.mlp(obs)
        E = self.actions(legal)
        return E @ h


# ===========================================================================
# Part 2: choosing a move.
# ===========================================================================

def choose(model, game: monopd.Game) -> int:
    """Sample an action for the player about to decide in `game`.

    Score the legal actions with `model`, sample one with
    torch.distributions.Categorical (using PyTorch's global random generator),
    and return the chosen ACTION INDEX as a Python int (not its position in
    the legal list).
    """
    current_player = game.current_player()

    if current_player is None:
        raise RuntimeError("attempted to choose on game that is over")

    obs = torch.tensor(game.observe(current_player))
    legal_actions = torch.tensor(game.legal_actions())
    scores = model(obs, legal_actions)
    sampler = Categorical(logits=scores)
    legal_action_pos = int(sampler.sample().item())

    return int(legal_actions[legal_action_pos].item())


# ===========================================================================
# Part 3: one full game.
# ===========================================================================

def play_game(model, seed, model_seat):
    """Play one 2-player game: `model` in `model_seat`, RandomBot in the other seat.

    The game is `monopd.Game(num_players=2, seed=seed)` and the bot is
    `monopd.RandomBot(seed=seed)`. Whoever `current_player()` says is deciding
    makes the move: the model through `choose`, the bot through `bot.choose`.
    Returns the model seat's reward as a Python float: +1 win, -1 loss, 0 draw.
    """

    game = monopd.Game(num_players=2, seed=seed)
    random_bot = monopd.RandomBot(seed=seed)

    while not game.is_over():

        if game.current_player() == model_seat:
            action = choose(model, game)
        else:
            action = random_bot.choose(game)

        game.step(action)

    return game.returns()[model_seat]


# ===========================================================================
# Part 4: measuring.
# ===========================================================================

def evaluate(model, n_games, seed=0):
    """Win rate of `model` against RandomBot over n_games.

    Game k (0-based) is play_game(model, seed + k, model_seat = k % 2), so the
    model takes each seat equally often. No training here: no gradients, and
    put the model in eval mode. Returns wins / n_games as a Python float.
    """

    with torch.no_grad():
        model.eval()
        wins = 0
        for i in range(n_games):
            if play_game(model, seed=seed+i, model_seat=i%2) > 0.0:
                wins += 1
        return wins / n_games


# ===========================================================================
# Below here is provided: checks and experiments. You don't need to edit it.
# ===========================================================================

def _states(n, seed=0):
    """Some (game, seat) positions from random games, for checks."""
    out = []
    g, bot = monopd.Game(num_players=2, seed=seed), monopd.RandomBot(seed=seed)
    while len(out) < n:
        if g.is_over():
            seed += 1
            g, bot = monopd.Game(num_players=2, seed=seed), monopd.RandomBot(seed=seed)
        out.append(g.clone())
        g.step(bot.choose(g))
    return out


class _Fixed(nn.Module):
    """A stand-in model: logits are given by position in the legal list."""

    def __init__(self, logits_by_position):
        super().__init__()
        self.logits = torch.tensor(logits_by_position, dtype=torch.float32)
        self.seen = []

    def forward(self, obs, legal):
        self.seen.append(obs.clone())
        k = len(legal)
        return self.logits[:k] if k <= len(self.logits) else torch.cat([self.logits, torch.full((k - len(self.logits),), -1e9)])


def _check_part1():
    torch.manual_seed(0)
    m = PolicyNet(obs_size=20, num_actions=50, hidden=16)
    embs = [x for x in m.modules() if isinstance(x, nn.Embedding)]
    shapes = [tuple(e.weight.shape) for e in embs]
    assert (50, 16) in shapes, \
        f"PolicyNet should have an nn.Embedding of shape (num_actions, hidden) = (50, 16) for the actions, found {shapes or 'none'}"
    obs = torch.randn(20)
    legal = torch.tensor([3, 7, 0, 41, 12])
    out = m(obs, legal)
    assert out.shape == (5,), f"forward should return one logit per legal action: shape (5,), got {tuple(out.shape)}"
    assert m(obs, torch.tensor([7])).shape == (1,), "a single legal action should give shape (1,)"
    perm = torch.tensor([4, 2, 0, 3, 1])
    assert torch.allclose(m(obs, legal[perm]), out[perm], atol=1e-5), \
        "shuffling the legal list should shuffle the scores the same way: each action's score can't depend on its position"
    assert torch.allclose(m(obs, torch.tensor([7, 7]))[0], m(obs, torch.tensor([7, 7]))[1]), "the same action should get the same score"
    assert not torch.allclose(m(torch.randn(20), legal), out), "scores should depend on the observation"
    assert out.requires_grad, "logits should be differentiable (don't use torch.no_grad inside forward)"


def _check_part2():
    states = _states(300)
    torch.manual_seed(0)
    m = PolicyNet(hidden=32)
    for g in states:
        a = choose(m, g)
        assert isinstance(a, int), f"choose should return a Python int, got {type(a).__name__}"
        assert a in g.legal_actions(), \
            f"choose returned {a}, which isn't a legal action index. Is it the position in the legal list instead of legal[position]?"
    g = next(s for s in states if len(s.legal_actions()) >= 3)
    legal = g.legal_actions()
    fixed = _Fixed([math.log(0.6), math.log(0.3), math.log(0.1)] + [-1e9] * (len(legal) - 3))
    torch.manual_seed(1)
    picks = [choose(fixed, g) for _ in range(3000)]
    freq = [picks.count(legal[i]) / 3000 for i in range(3)]
    assert all(abs(f - p) < 0.04 for f, p in zip(freq, (0.6, 0.3, 0.1))), \
        f"with probabilities 0.6 / 0.3 / 0.1 the picks came out {freq[0]:.2f} / {freq[1]:.2f} / {freq[2]:.2f}: sample from the softmax"
    torch.manual_seed(5)
    a = [choose(m, g) for _ in range(20)]
    torch.manual_seed(5)
    assert a == [choose(m, g) for _ in range(20)], "sampling should use PyTorch's global generator, so torch.manual_seed repeats it"


def _replay_first_legal(seed, model_seat):
    """The model's observations and the result when the model always plays legal[0]."""
    g, bot = monopd.Game(num_players=2, seed=seed), monopd.RandomBot(seed=seed)
    seen = []
    while not g.is_over():
        if g.current_player() == model_seat:
            seen.append(torch.tensor(g.observe(model_seat)))
            g.step(g.legal_actions()[0])
        else:
            g.step(bot.choose(g))
    return seen, g.returns()[model_seat]


def _check_part3():
    for seed, seat in ((3, 0), (4, 1), (11, 1)):
        fixed = _Fixed([0.0] + [-1e9] * 200)
        r = play_game(fixed, seed, seat)
        seen, expected = _replay_first_legal(seed, seat)
        assert isinstance(r, float), f"play_game should return a Python float, got {type(r).__name__}"
        assert len(fixed.seen) == len(seen), \
            f"game {seed}, seat {seat}: the model should decide {len(seen)} times (exactly when current_player() is its seat), got {len(fixed.seen)}"
        assert all(torch.equal(a, b) for a, b in zip(fixed.seen, seen)), \
            f"game {seed}, seat {seat}: the model should see observe(model_seat), before each of its moves"
        assert r == expected, f"game {seed}, seat {seat}: expected reward {expected}, got {r}"


def _check_part4():
    torch.manual_seed(0)
    m = PolicyNet(hidden=32)
    torch.manual_seed(7)
    wr = evaluate(m, 6, seed=20)
    assert isinstance(wr, float) and 0.0 <= wr <= 1.0, f"evaluate should return a win rate in [0, 1] as a float, got {wr}"
    torch.manual_seed(7)
    with torch.no_grad():
        expected = sum(play_game(m, 20 + k, k % 2) == 1.0 for k in range(6)) / 6
    assert abs(wr - expected) < 1e-9, \
        f"expected {expected:.3f} from play_game(model, seed + k, k % 2) for k in 0..5, got {wr:.3f}"
    m.train()
    evaluate(m, 1)
    assert not m.training, "evaluate should put the model in eval mode"


def check():
    parts = [("1 model", _check_part1), ("2 choose", _check_part2), ("3 game", _check_part3), ("4 evaluate", _check_part4)]
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
    print("\nAll parts pass. Now: uv run python policy.py play, then play --emb-std 0.01, then show")


class _Watch(nn.Module):
    """Wraps a model and records how peaked its choices are."""

    def __init__(self, inner):
        super().__init__()
        self.inner = inner
        self.forced = self.free = 0
        self.max_prob = self.entropy = 0.0

    def forward(self, obs, legal):
        logits = self.inner(obs, legal)
        if len(legal) == 1:
            self.forced += 1
        else:
            d = Categorical(logits=logits)
            self.free += 1
            self.max_prob += d.probs.max().item()
            self.entropy += d.entropy().item() / math.log(len(legal))
        return logits


def _make_model(args):
    torch.manual_seed(args.model_seed)
    model = PolicyNet()
    if args.emb_std is not None:
        for m in model.modules():
            if isinstance(m, nn.Embedding):
                nn.init.normal_(m.weight, std=args.emb_std)
    return model


def run_play(args):
    w = _Watch(_make_model(args))
    torch.manual_seed(args.seed)
    t = time.time()
    wr = evaluate(w, args.games, seed=args.seed)
    se = math.sqrt(wr * (1 - wr) / args.games)
    init = "default" if args.emb_std is None else f"std {args.emb_std}"
    print(f"untrained PolicyNet (model seed {args.model_seed}, embeddings {init}) vs RandomBot, {args.games} games:")
    print(f"  win rate {wr:.1%}  (+/- {1.96 * se:.1%} at 95%)")
    print(f"  its decisions: {w.free + w.forced}, of which {w.forced / (w.free + w.forced):.0%} forced (one legal action)")
    print(f"  on the others: average top probability {w.max_prob / w.free:.2f}, entropy {w.entropy / w.free:.2f} of uniform")
    print(f"  {time.time() - t:.1f}s")


def run_show(args):
    model = _make_model(args)
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
            probs = torch.softmax(model(torch.tensor(g.observe(0)), torch.tensor(legal)), 0)
            a = choose(model, g)
            if len(legal) > 1:
                top = sorted(zip(probs.tolist(), legal), reverse=True)[:3]
                print(f"turn {g.turn:3}  chose {monopd.action_name(a)!r}")
                print("           top: " + ";  ".join(f"{p:.2f} {monopd.action_name(i)}" for p, i in top) + f"   ({len(legal)} legal)")
                shown += 1
            g.step(a)
    print(f"\n... after {shown} non-forced decisions. Winner so far: {g.winner}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("check")
    for name in ("play", "show"):
        p = sub.add_parser(name)
        p.add_argument("--model-seed", type=int, default=0, help="seed for the model's initial weights")
        p.add_argument("--emb-std", type=float, default=None, help="re-initialise the embeddings with this std")
        p.add_argument("--seed", type=int, default=0, help="seed for the games and sampling")
    sub.choices["play"].add_argument("--games", type=int, default=1000)
    sub.choices["show"].add_argument("--moves", type=int, default=15)
    args = parser.parse_args()
    {"check": lambda a: check(), "play": run_play, "show": run_show}[args.cmd](args)
