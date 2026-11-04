# Web viewer

Play Monopoly Deal against random bots, or watch bots play each other. The page runs the real Rust engine, compiled to WebAssembly, so the browser enforces exactly the same rules as training does.

```bash
./web/build.sh                                   # compile engine -> web/monopd.wasm
python3 -m http.server 8737 --directory web      # then open http://localhost:8737
```

Click highlighted cards, sets or players to act. When a card has several possible uses, a menu appears.

## Layout

| file | contents |
|---|---|
| `js/main.js` | entry point: game loop, controls, click handling |
| `js/engine.js` | loads `monopd.wasm` and wraps its functions |
| `js/cards.js` | card faces (and optional local images) |
| `js/table.js` | players, property sets, banks, centre strip, status, log |
| `style.css` | all styles, in numbered sections |
| `wasm/src/lib.rs` | functions exported to the page; the game session and its log |
| `wasm/src/view.rs` | the card catalog and per-seat game state sent to the page |
| `wasm/src/text.rs` | wording for actions and phases; colour labels; card slugs |
| `wasm/src/json.rs` | a tiny JSON writer (no dependencies) |
