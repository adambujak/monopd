# Web viewer

Play Monopoly Deal against random bots, or watch bots play each other. The page runs the real Rust engine, compiled to WebAssembly, so the browser enforces exactly the same rules as training does.

```bash
./web/build.sh                                   # compile engine -> web/monopd.wasm
python3 -m http.server 8737 --directory web      # then open http://localhost:8737
```

Click highlighted cards, sets or players to act. When a card has several possible uses, a menu appears.
