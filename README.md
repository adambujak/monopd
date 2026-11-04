# monopd

A Monopoly Deal engine and, eventually, a self-play RL agent.

- [`engine/`](engine/): a fast Rust game engine (rules, action encoding, observations, bots). See its [README](engine/README.md).
- [`web/`](web/): a browser UI to play against bots or watch them play, powered by the engine compiled to WebAssembly.
