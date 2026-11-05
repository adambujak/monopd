# Learning path

Building a Monopoly Deal agent yourself, one working step at a time. Each step's code is yours; ask Claude for hints, explanations and code reviews.

## Setup

```bash
python3 -m venv .venv                         # once, from the repo root
source .venv/bin/activate
pip install torch numpy matplotlib maturin
(cd bindings && maturin develop --release)    # builds `import monopd`; re-run after engine changes
```

Quick check:

```python
import monopd
g = monopd.Game(num_players=2, seed=0)
print(g.describe(0))
print([monopd.action_name(a) for a in g.legal_actions()])
```

The full Python API, with docs, is in [`bindings/monopd.pyi`](../bindings/monopd.pyi).

## Steps

1. **PyTorch on a toy problem:** [`01_pytorch_toy/`](01_pytorch_toy/). Tensors, models, the training loop, overfitting.
2. **Win predictor:** collect `(observe(seat), did that seat win?)` from random games and train a network to predict the winner. It becomes the value head.
3. **Policy network, untrained:** the two-column model (state → `h`, action index → embedding, score = `h · E`) playing full games.
4. **REINFORCE:** make the policy learn against random bots and watch the win rate climb.
5. **Actor-critic:** use the value head as a baseline.
6. **Self-play** against saved copies of itself.
7. **PPO.**

Later: action features in place of lookup-table embeddings, then attention across players.
