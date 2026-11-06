# Learning path

Building a Monopoly Deal agent yourself, one working step at a time. Each step's code is yours; ask Claude for hints, explanations and code reviews.

## Setup

The repo is a [uv](https://docs.astral.sh/uv/) project. From the repo root:

```bash
uv sync                         # creates .venv with torch, numpy, matplotlib and monopd
uv run python some_script.py    # runs inside that environment
uv add <package>                # adds a dependency (updates pyproject.toml and uv.lock)
```

`monopd` is built from `bindings/`, and the build needs Rust. uv rebuilds it automatically whenever the engine or bindings change, so there's nothing to re-run.

Quick check (`uv run python`):

```python
import monopd
g = monopd.Game(num_players=2, seed=0)
print(g.describe(0))
print([monopd.action_name(a) for a in g.legal_actions()])
```

The full Python API, with docs, is in [`bindings/monopd.pyi`](../bindings/monopd.pyi).

## Docs

- [`NOTES.md`](NOTES.md): what we've worked through so far (the model, PyTorch concepts, mistakes worth remembering).
- [`NEXT_STEPS.md`](NEXT_STEPS.md): how to set up each step after the toy.

## Steps

1. **PyTorch on a toy problem:** [`01_pytorch_toy/`](01_pytorch_toy/). Tensors, models, the training loop, overfitting.
2. **Win predictor:** collect `(observe(seat), did that seat win?)` from random games and train a network to predict the winner. It becomes the value head.
3. **Policy network, untrained:** the two-column model (state → `h`, action index → embedding, score = `h · E`) playing full games.
4. **REINFORCE:** make the policy learn against random bots and watch the win rate climb.
5. **Actor-critic:** use the value head as a baseline.
6. **Self-play** against saved copies of itself.
7. **PPO.**

Later: action features in place of lookup-table embeddings, then attention across players.
