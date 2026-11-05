# monopd

A Monopoly Deal engine and, eventually, a self-play RL agent.

- [`engine/`](engine/): a fast Rust game engine (rules, action encoding, observations, bots). See its [README](engine/README.md).
- [`web/`](web/): a browser UI to play against bots or watch them play, using the engine compiled to WebAssembly.
- [`bindings/`](bindings/): `import monopd`, the engine from Python (pyo3). API in [`monopd.pyi`](bindings/monopd.pyi).
- [`learn/`](learn/): the learning path toward the agent, starting with a PyTorch toy problem.
