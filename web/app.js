// Monopoly Deal viewer: drives the Rust engine (compiled to wasm) and renders it.
'use strict';

const COLOR_HEX = ['#9a5b34', '#7fd0f0', '#ec4fa0', '#ff8f1f', '#ef3b3b', '#ffd23f', '#1fae5b', '#2457c5', '#353a48', '#a9d18e'];
const COLOR_INK = ['#fff', '#0b3550', '#fff', '#fff', '#fff', '#4a3a00', '#fff', '#fff', '#fff', '#21401a'];
const COLOR_LABEL = ['Brown', 'Light Blue', 'Pink', 'Orange', 'Red', 'Yellow', 'Green', 'Dark Blue', 'Railroad', 'Utility'];
const MONEY_COLOR = { 1: '#fff0a0', 2: '#ffc4b2', 3: '#c3efc6', 4: '#bfe2ff', 5: '#dccbff', 10: '#ffcf5c' };
const PLAYER_VAR = ['--p0', '--p1', '--p2', '--p3', '--p4'];
// Kind indices (see Kind::index in engine/src/cards.rs: actions start at 18).
const KIND_JSN = 19;
const KIND_DOUBLE = 24;
// [icon, card colour, one-line rules reminder]
const ACTION_STYLE = {
  'Deal Breaker': ['💥', '#7b3fe4', 'Steal a complete set'],
  'Just Say No': ['✋', '#e5383b', 'Cancel an action against you'],
  'Sly Deal': ['🦊', '#ff7b00', 'Steal one property'],
  'Forced Deal': ['🔁', '#1f8ef1', 'Swap a property with someone'],
  'Debt Collector': ['💼', '#0fa37f', 'One player pays you $5M'],
  "It's My Birthday": ['🎂', '#f0479a', 'Everyone pays you $2M'],
  'Double The Rent': ['×2', '#f4a100', 'Play with a rent card'],
  'House': ['🏠', '#2bb24c', '+$3M rent on a full set'],
  'Hotel': ['🏨', '#d6336c', '+$4M rent on a full set'],
  'Pass Go': ['🎲', '#ff5a5f', 'Draw 2 cards'],
};
const PAWN = '<svg class="pawn" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="6" r="4.2"/><path d="M8.2 11.5h7.6l2.2 7.5H6z"/><rect x="4" y="18.5" width="16" height="3.5" rx="1.75"/></svg>';

const ui = { mode: 'play', players: 3, seed: 0, reveal: false, bankDetail: false, paused: false, speed: 55, timer: null };
let wasm, CAT, S, SKIN = {}, prevIds = new Set(), actionMap = new Map();

// Stable name for each card kind, used as the key in cards/skin.json.
const COLOR_SLUG = ['brown', 'lightblue', 'pink', 'orange', 'red', 'yellow', 'green', 'darkblue', 'railroad', 'utility'];
function kindSlug(K) {
  const cs = (K.colors || []).map((c) => COLOR_SLUG[c]).join('-');
  switch (K.type) {
    case 'property': return `property-${cs}`;
    case 'wild': return K.colors.length > 2 ? 'wild-rainbow' : `wild-${cs}`;
    case 'rent': return K.colors.length > 2 ? 'rent-wild' : `rent-${cs}`;
    case 'money': return `money-${K.value}`;
    default: return `action-${K.short.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '')}`;
  }
}
const $ = (id) => document.getElementById(id);

// ---------------------------------------------------------------- wasm glue

async function init() {
  try {
    const bytes = await (await fetch('monopd.wasm')).arrayBuffer();
    const { instance } = await WebAssembly.instantiate(bytes, {});
    wasm = instance.exports;
  } catch (e) {
    showError('Could not load monopd.wasm. Run web/build.sh, then serve web/ over HTTP.');
    throw e;
  }
  wasm.catalog();
  CAT = readOut();
  // Optional local card images: web/cards/skin.json maps card slugs to files.
  try {
    const r = await fetch('cards/skin.json', { cache: 'no-store' });
    if (r.ok) SKIN = await r.json();
  } catch { /* no skin: use the built-in designs */ }
  bindControls();
  ui.seed = randomSeed();
  $('seed').value = ui.seed;
  newGame();
}

function readOut() {
  const ptr = wasm.out_ptr(), len = wasm.out_len();
  return JSON.parse(new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, ptr, len)));
}

function viewSeat() { return ui.mode === 'watch' || ui.reveal ? -1 : 0; }
function human() { return ui.mode === 'play' ? 0 : null; }

function newGame() {
  clearTimeout(ui.timer);
  ui.players = +$('players').value;
  ui.seed = (+$('seed').value >>> 0);
  wasm.new_game(ui.players, ui.seed);
  prevIds = new Set();
  refresh();
}

function refresh() {
  wasm.state(viewSeat());
  S = readOut();
  // In Play mode a bot is only asked about Just Say No / Double The Rent when
  // it holds one, so showing it "deciding" would reveal its hand. Let it
  // answer straight away instead.
  while (ui.mode === 'play' && S.current !== null && S.current !== human() && (S.phase === 'respond' || S.phase === 'double')) {
    wasm.bot_step();
    wasm.state(viewSeat());
    S = readOut();
  }
  render();
  schedule();
}

function delay() { return Math.round(1600 * Math.pow(1 - ui.speed / 100, 2) + 40); }

function schedule() {
  clearTimeout(ui.timer);
  if (S.current === null) return;
  const botTurn = ui.mode === 'watch' ? !ui.paused : S.current !== human();
  if (botTurn) ui.timer = setTimeout(() => { wasm.bot_step(); refresh(); }, delay());
}

function act(index) {
  hidePopover();
  if (!wasm.step(index)) showError('Illegal action');
  refresh();
}

// ---------------------------------------------------------------- rendering

const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
const pc = (p) => `var(${PLAYER_VAR[p]})`;

function cardHTML(card, size = '', key = null) {
  const k = card.k, K = CAT.kinds[k];
  const coin = (v) => `<div class="coin">${v}<small>M</small></div>`;
  const rentRows = (c) => {
    const col = CAT.colors[c];
    return col.rent.map((r, i) => {
      const dots = Array.from({ length: col.size }, (_, j) => `<i class="${j <= i ? 'on' : ''}"></i>`).join('');
      return `<div class="row"><span class="dots">${dots}</span><b>$${r}M</b></div>`;
    }).join('');
  };
  const splitBg = (cs) => cs.length > 2
    ? `linear-gradient(90deg, ${COLOR_HEX.map((h, i) => `${h} ${i * 10}% ${(i + 1) * 10}%`).join(',')})`
    : `linear-gradient(115deg, ${COLOR_HEX[cs[0]]} 50%, ${COLOR_HEX[cs[1]]} 50%)`;
  let inner = '', style = '';
  if (K.type === 'property') {
    const c = K.colors[0];
    style = `--cc:${COLOR_HEX[c]};--ci:${COLOR_INK[c]}`;
    inner = `<div class="head"><span>${COLOR_LABEL[c]}</span></div>
      <div class="body${CAT.colors[c].size > 3 ? ' tight' : ''}">${CAT.colors[c].size > 3 ? '' : '<div class="tag">Rent</div>'}${rentRows(c)}</div>${coin(K.value)}`;
  } else if (K.type === 'wild') {
    const any = K.colors.length > 2;
    inner = `<div class="head wildhead" style="background:${splitBg(K.colors)}"><span class="sticker">WILD</span></div>
      <div class="body">${any
        ? '<div class="anyc">Any colour</div><div class="tag">No cash value</div>'
        : K.colors.map((c) => `<div class="chip" style="background:${COLOR_HEX[c]};color:${COLOR_INK[c]}">${COLOR_LABEL[c]}<small>$${CAT.colors[c].rent.join('·')}M</small></div>`).join('')}</div>
      ${any ? '' : coin(K.value)}`;
  } else if (K.type === 'action') {
    const [icon, color, tip] = ACTION_STYLE[K.short] || ['?', '#555', ''];
    style = `--ac:${color}`;
    inner = `<div class="burst${icon === '×2' ? ' txt' : ''}"><span>${icon}</span></div><div class="ribbon">${K.short}</div><div class="tip">${tip}</div>${coin(K.value)}`;
  } else if (K.type === 'rent') {
    const cs = K.colors, any = cs.length > 2;
    style = `--ac:${any ? '#30364f' : '#fff7e6'}`;
    const disc = any ? `conic-gradient(${COLOR_HEX.join(',')}, ${COLOR_HEX[0]})` : splitBg(cs);
    inner = `<div class="burst disc" style="background:${disc}">${any ? '<span>ANY</span>' : ''}</div>
      <div class="ribbon">${any ? 'Wild Rent' : 'Rent'}</div>
      <div class="tip">${any ? 'Any colour · one player pays' : `${COLOR_LABEL[cs[0]]} or ${COLOR_LABEL[cs[1]]} · all players pay`}</div>${coin(K.value)}`;
  } else {
    style = `--mc:${MONEY_COLOR[K.value]}`;
    inner = `<div class="note"><span class="corner tl">${K.value}</span><span class="corner br">${K.value}</span>
      <div class="medal">$${K.value}<small>M</small></div></div>`;
  }
  const fresh = prevIds.size && card.id !== undefined && !prevIds.has(card.id) ? ' fresh' : '';
  const cls = actionMap.has(key) ? ' clickable' : '';
  const img = SKIN[kindSlug(K)];
  if (img) {
    return `<div class="card skin ${size}${cls}${fresh}" ${key ? `data-key="${key}"` : ''} title="${esc(K.name)}"><img src="cards/${esc(img)}" alt="${esc(K.name)}"></div>`;
  }
  return `<div class="card ${K.type} ${size}${cls}${fresh}" style="${style}" ${key ? `data-key="${key}"` : ''} title="${esc(K.name)}">${inner}</div>`;
}

function backs(n, size) {
  let s = '';
  const face = SKIN.back ? `<img src="cards/${esc(SKIN.back)}" alt="">` : '<span>deal</span>';
  for (let i = 0; i < Math.min(n, 12); i++) s += `<div class="card back ${SKIN.back ? 'skin' : ''} ${size}">${face}</div>`;
  return `<div class="hand backs">${s || '<span class="empty-note">no cards</span>'}</div>`;
}

function setHTML(seat, set) {
  const c = set.color, col = CAT.colors[c];
  const key = `set:${seat}:${c}`;
  const cls = (set.complete ? ' complete' : '') + (actionMap.has(key) ? ' clickable' : '');
  const blds = (set.house !== null ? '<span class="bld" title="House">🏠</span>' : '') +
    (set.hotel !== null ? '<span class="bld" title="Hotel">🏨</span>' : '');
  const cards = set.cards.map((cd) => cardHTML(cd, 'sm', `tcard:${seat}:${c}:${cd.k}`)).join('');
  // Dashed placeholders show how many more cards complete the set.
  const missing = Math.max(0, col.size - set.cards.length);
  const ghosts = Array.from({ length: missing }, () =>
    `<div class="card sm ghost" style="--cc:${COLOR_HEX[c]}"><span>${COLOR_LABEL[c]}</span></div>`).join('');
  return `<div class="set${cls}" data-key="${key}" style="--sc:${COLOR_HEX[c]}">
    <div class="set-head"><span class="count">${set.cards.length}/${col.size}</span><span class="rent">$${set.rent}M</span>${blds}</div>
    <div class="stack">${cards}${ghosts}</div>
    ${set.complete ? '<div class="set-flag">SET!</div>' : ''}</div>`;
}

function sortedSets(table) {
  const progress = (s) => s.cards.length / CAT.colors[s.color].size;
  return [...table].sort((a, b) => (b.complete - a.complete) || (progress(b) - progress(a)) || (a.color - b.color));
}

// A player's bank. Your own bank is always open to you. Other banks are
// squared up like a real pile: you see the card count and the newest card on
// top, unless "Bank details" is on (the perfect-memory view).
function bankHTML(i, p, open) {
  const n = p.bank.length;
  if (!n) return `<div class="bank-box"><div class="bank-title">Bank</div><div class="bank-empty">empty</div></div>`;
  if (open) {
    const groups = new Map();
    for (const cd of p.bank) groups.set(cd.k, [...(groups.get(cd.k) || []), cd]);
    const items = [...groups.entries()]
      .sort((a, b) => CAT.kinds[b[0]].value - CAT.kinds[a[0]].value)
      .map(([k, cds]) => {
        const K = CAT.kinds[k], key = `bank:${i}:${k}`;
        const name = K.type === 'money' ? '' : `<small>${esc(K.type === 'rent' ? K.name : K.short)}</small>`;
        const bg = K.type === 'money' ? MONEY_COLOR[K.value] : '#fff';
        return `<div class="bank-chip${actionMap.has(key) ? ' clickable' : ''}" data-key="${key}" title="${esc(K.name)}" style="background:${bg}">
          <b>$${K.value}M</b>${name}${cds.length > 1 ? `<span class="times">×${cds.length}</span>` : ''}</div>`;
      })
      .join('');
    return `<div class="bank-box open"><div class="bank-title">Bank <b>$${p.bankValue}M</b></div><div class="bank-list">${items}</div></div>`;
  }
  const layers = Array.from({ length: Math.min(n, 4) - 1 }, (_, j) => `<div class="layer" style="--d:${(j + 1) * 3}px"></div>`).join('');
  return `<div class="bank-box"><div class="bank-title">Bank</div>
    <div class="pile">${layers}${cardHTML(p.bank[n - 1], 'sm')}</div>
    <div class="bank-count">${n} card${n === 1 ? '' : 's'}</div></div>`;
}

function playerHTML(i, big) {
  const p = S.players[i];
  const classes = ['player'];
  if (S.active === i && S.current !== null) classes.push('active');
  if (S.current === i) classes.push('deciding');
  if (S.winner === i) classes.push('winner');
  const pkey = `player:${i}`;
  if (actionMap.has(pkey)) classes.push('clickable');
  const isHuman = human() === i;
  const tag = isHuman ? 'You' : 'Random bot';
  const table = p.table.length
    ? sortedSets(p.table).map((s) => setHTML(i, s)).join('')
    : '<div class="no-props">No properties yet</div>';
  const stars = [0, 1, 2].map((j) => `<span class="star ${j < p.sets ? 'on' : ''}">★</span>`).join('');
  const handCount = `<span class="handcount" title="Cards in hand"><span class="mini-back"></span><b>${p.handCount}</b></span>`;

  let handRow = '';
  if (isHuman && p.hand) {
    handRow = `<div class="myhand"><div class="myhand-title">Your hand</div><div class="hand">${p.hand.length
      ? p.hand.map((cd) => cardHTML(cd, '', `hand:${cd.k}`)).join('')
      : '<span class="empty-note">no cards</span>'}</div></div>`;
  } else if (ui.reveal && p.hand && p.hand.length) {
    handRow = `<div class="peek"><span class="peek-title">Hand</span>${p.hand.map((cd) => cardHTML(cd, 'xs')).join('')}</div>`;
  }
  return `<section class="${classes.join(' ')}" data-key="${pkey}" style="--pc:${pc(i)}">
    <div class="phead">
      <span class="token">${PAWN}</span><span class="pname">P${i}</span><span class="ptag">${tag}</span>
      <span class="stars" title="${p.sets} of 3 complete sets">${stars}</span>
      ${handCount}
    </div>
    <div class="pbody">
      <div class="table">${table}</div>
      ${bankHTML(i, p, isHuman || ui.bankDetail)}
    </div>
    ${handRow}
  </section>`;
}

const BAR_VERBS = { end: ['End turn', ''], double: ['Double the rent', 'go'], noDouble: ["Don't double", ''], jsn: ['Just Say No!', 'jsn'], accept: ['Accept', ''] };

function centerHTML() {
  const top = S.discardTop !== null ? cardHTML({ k: S.discardTop }, 'sm') : '<div class="card sm" style="opacity:.15"></div>';
  const pips = S.phase === 'main'
    ? `<div class="pips">Plays ${[0, 1, 2].map((i) => `<span class="pip ${i < S.playsLeft ? 'full' : ''}"></span>`).join('')}</div>`
    : '';
  const bar = S.legal.filter((a) => BAR_VERBS[a.verb])
    .map((a) => `<button class="${BAR_VERBS[a.verb][1]}" data-act="${a.i}">${BAR_VERBS[a.verb][0]}</button>`).join('');
  return `<div class="pile-slot"><div class="card back sm"><span>deal</span></div>Deck · ${S.deck}</div>
    <div class="pile-slot">${top}Discard · ${S.discard}</div>
    <div class="center-info"><div class="phase-text">${esc(S.phaseText)}</div>${pips}</div>
    <div class="action-bar">${bar}</div>`;
}

function hintText() {
  if (S.current === null) return '';
  if (ui.mode === 'watch') return 'Bots are playing. Use Pause and Step to go one decision at a time.';
  if (S.current !== 0) return 'Waiting for the bots…';
  switch (S.phase) {
    case 'main': return 'Click a highlighted card in your hand to bank it or play it. Click a wild on your table to move it (that uses a play).';
    case 'target': return 'Click a highlighted player, property or set.';
    case 'pay': return 'Click cards in your bank or on your table to pay. No change is given.';
    case 'respond': return 'You hold a Just Say No. Click it in your hand (or the button) to cancel, or accept.';
    case 'double': return 'Click the Double The Rent in your hand (or the button) to double the charge. It uses a play.';
    case 'discard': return 'You have more than 7 cards. Click cards in your hand to discard them.';
  }
  return '';
}

function render() {
  actionMap = new Map();
  const myDecision = ui.mode === 'play' && S.current === 0;
  for (const a of myDecision ? S.legal : []) {
    const key = keyFor(a);
    if (!key) continue;
    if (!actionMap.has(key)) actionMap.set(key, []);
    actionMap.get(key).push(a);
  }

  const me = ui.mode === 'play' ? 0 : null;
  const others = [];
  for (let i = 0; i < S.n; i++) if (i !== me) others.push(i);
  $('opponents').innerHTML = others.map((i) => playerHTML(i, ui.mode === 'watch' && S.n <= 3)).join('');
  $('me').innerHTML = me !== null ? playerHTML(0, true) : '';
  $('center').innerHTML = centerHTML();

  const over = S.current === null;
  $('status').innerHTML = `
    ${over ? `<div class="winner-banner">${esc(S.phaseText)}</div>` : `<div class="status-phase">${esc(S.phaseText)}</div>`}
    <div class="status-hint">${esc(hintText())}</div>
    <div class="status-meta"><span>Turn <b>${S.turn + 1}</b></span><span>Players <b>${S.n}</b></span><span>Seed <b>${ui.seed}</b></span></div>`;

  const log = $('log');
  const stick = log.scrollHeight - log.scrollTop - log.clientHeight < 40;
  log.innerHTML = S.log.map((e) => e.p < 0
    ? `<li class="sys">${esc(e.t)}</li>`
    : `<li style="--lc:${pc(e.p)}"><span class="who">P${e.p}</span>${esc(e.t)}</li>`).join('');
  if (stick) log.scrollTop = log.scrollHeight;

  $('legal-list').innerHTML = S.legal.length
    ? S.legal.map((a) => `<button data-act="${a.i}">${esc(a.label)}</button>`).join('')
    : '<span class="empty-note">Nothing to do right now</span>';

  prevIds = new Set();
  for (const p of S.players) {
    for (const c of p.bank) prevIds.add(c.id);
    for (const s of p.table) for (const c of s.cards) prevIds.add(c.id);
    if (p.hand) for (const c of p.hand) prevIds.add(c.id);
  }
  $('pause').textContent = ui.paused ? 'Resume' : 'Pause';
}

function keyFor(a) {
  switch (a.verb) {
    case 'bank': case 'place': case 'play': case 'rent': case 'discard': return `hand:${a.kind}`;
    case 'move': case 'targetProperty': case 'payProperty': return `tcard:${a.seat}:${a.color}:${a.kind}`;
    case 'targetSet': case 'payBuilding': return `set:${a.seat}:${a.color}`;
    case 'payBank': return `bank:${a.seat}:${a.kind}`;
    case 'targetPlayer': return `player:${a.seat}`;
    // Responses can also be made by clicking the card in your hand.
    case 'double': return `hand:${KIND_DOUBLE}`;
    case 'jsn': return `hand:${KIND_JSN}`;
    default: return null;
  }
}

// ---------------------------------------------------------------- interaction

function onBoardClick(e) {
  const btn = e.target.closest('[data-act]');
  if (btn) { act(+btn.dataset.act); return; }
  if (ui.mode === 'watch' || S.current !== 0) return;
  // Innermost clickable element wins (card inside set inside player).
  let el = e.target.closest('[data-key]');
  while (el && !actionMap.has(el.dataset.key)) el = el.parentElement && el.parentElement.closest('[data-key]');
  if (!el) { hidePopover(); return; }
  const actions = actionMap.get(el.dataset.key);
  const only = actions.length === 1 ? actions[0] : null;
  // Banking an action or rent card throws away its effect, so always confirm it.
  if (only && only.verb === 'bank' && CAT.kinds[only.kind].type !== 'money') {
    showPopover(el, actions, whyNotPlayable(only.kind));
  } else if (only) {
    act(only.i);
  } else {
    showPopover(el, actions);
  }
  e.stopPropagation();
}

function whyNotPlayable(k) {
  if (k === KIND_DOUBLE) return 'You play this right after charging rent: play a rent card first and you’ll be offered the double.';
  if (k === KIND_JSN) return 'You play this in response, when someone targets you.';
  const K = CAT.kinds[k];
  if (K.type === 'rent') return 'You need a property of a matching colour on the table to charge this rent.';
  if (K.short === 'House' || K.short === 'Hotel') return 'This needs a complete set (and a hotel needs a house). Railroads and utilities can’t have buildings.';
  return 'There’s no valid target for this right now.';
}

function showPopover(el, actions, note = '') {
  const pop = $('popover');
  const title = el.getAttribute('title') || 'Choose';
  pop.innerHTML = `<div class="ptitle">${esc(title)}</div>` + (note ? `<div class="pnote">${esc(note)}</div>` : '') +
    actions.map((a) => `<button data-act="${a.i}">${esc(a.label)}</button>`).join('');
  pop.hidden = false;
  const r = el.getBoundingClientRect();
  const pw = pop.offsetWidth, ph = pop.offsetHeight;
  let x = Math.min(r.left, window.innerWidth - pw - 10);
  let y = r.bottom + 6;
  if (y + ph > window.innerHeight - 10) y = Math.max(10, r.top - ph - 6);
  pop.style.left = `${Math.max(10, x)}px`;
  pop.style.top = `${y}px`;
}

function hidePopover() { $('popover').hidden = true; }

function showError(msg) {
  const el = $('error');
  el.textContent = msg;
  el.hidden = false;
  setTimeout(() => (el.hidden = true), 3500);
}

function randomSeed() { return Math.floor(Math.random() * 1e6); }

function setMode(mode) {
  ui.mode = mode;
  document.body.className = `mode-${mode}`;
  for (const b of $('mode').children) b.classList.toggle('on', b.dataset.mode === mode);
}

function bindControls() {
  setMode('play');
  $('mode').addEventListener('click', (e) => {
    const b = e.target.closest('button');
    if (!b) return;
    setMode(b.dataset.mode);
    ui.paused = false;
    newGame();
  });
  $('new').onclick = newGame;
  $('dice').onclick = () => { $('seed').value = randomSeed(); newGame(); };
  $('players').onchange = newGame;
  $('speed').oninput = (e) => { ui.speed = +e.target.value; };
  ui.speed = +$('speed').value;
  $('pause').onclick = () => { ui.paused = !ui.paused; refresh(); };
  $('stepbtn').onclick = () => { ui.paused = true; if (S.current !== null) wasm.bot_step(); refresh(); };
  $("reveal").onchange = (e) => { ui.reveal = e.target.checked; refresh(); };
  $("bankinfo").onchange = (e) => { ui.bankDetail = e.target.checked; refresh(); };
  document.querySelector('.board').addEventListener('click', onBoardClick);
  $('popover').addEventListener('click', (e) => {
    const b = e.target.closest('[data-act]');
    if (b) act(+b.dataset.act);
  });
  $('legal-list').addEventListener('click', (e) => {
    const b = e.target.closest('[data-act]');
    if (b) act(+b.dataset.act);
  });
  document.addEventListener('click', (e) => {
    if (!e.target.closest('#popover') && !e.target.closest('.board')) hidePopover();
  });
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') hidePopover();
    if (e.key === ' ' && ui.mode === 'watch' && e.target.tagName !== 'INPUT') { e.preventDefault(); $('pause').click(); }
  });
}

init();
