// The Rust engine, compiled to WebAssembly (see web/wasm). Structured data
// comes back as JSON in a shared buffer.

export async function loadEngine(url = 'monopd.wasm') {
  const bytes = await (await fetch(url)).arrayBuffer();
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const w = instance.exports;
  const read = () => JSON.parse(new TextDecoder().decode(new Uint8Array(w.memory.buffer, w.out_ptr(), w.out_len())));

  return {
    /** Static card and colour data: { kinds: [...], colors: [...] }. */
    catalog() { w.catalog(); return read(); },
    newGame(players, seed) { w.new_game(players, seed); },
    /** State for `seat`; -1 shows every hand. */
    state(seat) { w.state(seat); return read(); },
    /** Apply a legal action by index. Returns false if it wasn't legal. */
    step(index) { return w.step(index) === 1; },
    /** Let the bot act for the current player. Returns false if the game is over. */
    botStep() { return w.bot_step() === 1; },
  };
}
