// Monopoly Deal viewer: drives the Rust engine (compiled to wasm) and renders it.
'use strict';

const COLOR_HEX = ['#8b5a2b', '#8fd3f4', '#e0559e', '#f28c28', '#e23b3b', '#f5d33f', '#2fa35b', '#2a4fa8', '#2f2f2f', '#bcd3a6'];
const COLOR_INK = ['#fff', '#0d2a3a', '#fff', '#fff', '#fff', '#3a3000', '#fff', '#fff', '#fff', '#1f3320'];
const PLAYER_VAR = ['--p0', '--p1', '--p2', '--p3', '--p4'];
const ACTION_ICON = {
  'Deal Breaker': ['💥', '#7b2cbf'], 'Just Say No': ['✋', '#c0392b'], 'Sly Deal': ['🦊', '#d35400'],
  'Forced Deal': ['⇄', '#2980b9'], 'Debt Collector': ['$', '#16a085'], "It's My Birthday": ['🎂', '#e84393'],
  'Double The Rent': ['×2', '#f39c12'], 'House': ['🏠', '#27ae60'], 'Hotel': ['🏨', '#c0392b'], 'Pass Go': ['GO', '#e74c3c'],
};

const ui = { mode: 'play', players: 3, seed: 0, reveal: false, paused: false, speed: 55, timer: null };
let wasm, CAT, S, prevIds = new Set(), actionMap = new Map();
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
  let inner = '';
  if (K.type === 'property') {
    const c = K.colors[0], col = CAT.colors[c];
    inner = `<div class="band" style="background:${COLOR_HEX[c]};color:${COLOR_INK[c]}">${col.name.replace('LightBlue', 'Light Blue').replace('DarkBlue', 'Dark Blue')}</div>
      <div class="ladder">${col.rent.map((r, i) => `<span>${i + 1}${i + 1 === col.size ? '★' : ''} <b>$${r}M</b></span>`).join('')}</div>`;
  } else if (K.type === 'wild') {
    const bg = K.colors.length > 2
      ? `linear-gradient(90deg, ${COLOR_HEX.join(',')})`
      : `linear-gradient(135deg, ${COLOR_HEX[K.colors[0]]} 50%, ${COLOR_HEX[K.colors[1]]} 50%)`;
    const names = K.colors.length > 2 ? 'Any colour' : K.colors.map((c) => CAT.colors[c].name).join(' / ');
    inner = `<div class="band" style="background:${bg}">WILD</div><div class="wild-label">${names}</div>`;
  } else if (K.type === 'action') {
    const [icon, color] = ACTION_ICON[K.short] || ['?', '#555'];
    inner = `<div class="emblem" style="background:${color}">${icon}</div><div class="aname">${K.short}</div>`;
  } else if (K.type === 'rent') {
    const cs = K.colors;
    const bg = cs.length > 2
      ? `conic-gradient(${COLOR_HEX.join(',')})`
      : `linear-gradient(135deg, ${COLOR_HEX[cs[0]]} 50%, ${COLOR_HEX[cs[1]]} 50%)`;
    inner = `<div class="emblem" style="background:${bg};font-size:.95em;text-shadow:0 1px 2px #000">RENT</div>
      <div class="aname">${cs.length > 2 ? 'Any colour, one player' : cs.map((c) => CAT.colors[c].name).join(' / ')}</div>`;
  } else {
    inner = `<div class="money-amt">$${K.value}<small>M</small></div>`;
  }
  const val = K.value > 0 && K.type !== 'money' ? `<div class="val">$${K.value}M</div>` : '';
  const fresh = prevIds.size && card.id !== undefined && !prevIds.has(card.id) ? ' fresh' : '';
  const cls = actionMap.has(key) ? ' clickable' : '';
  return `<div class="card ${K.type} ${size}${cls}${fresh}" ${key ? `data-key="${key}"` : ''} title="${esc(K.name)}">${inner}${val}</div>`;
}

function backs(n, size) {
  let s = '';
  for (let i = 0; i < Math.min(n, 12); i++) s += `<div class="card back ${size}"></div>`;
  return `<div class="hand backs">${s || '<span class="empty-note">no cards</span>'}</div>`;
}

function setHTML(seat, set) {
  const c = set.color, col = CAT.colors[c];
  const key = `set:${seat}:${c}`;
  const cls = (set.complete ? ' complete' : '') + (actionMap.has(key) ? ' clickable' : '');
  const blds = (set.house !== null ? '<span class="bld" title="House">🏠</span>' : '') +
    (set.hotel !== null ? '<span class="bld" title="Hotel">🏨</span>' : '');
  const cards = set.cards.map((cd) => cardHTML(cd, 'sm', `tcard:${seat}:${c}:${cd.k}`)).join('');
  return `<div class="set${cls}" data-key="${key}" style="--sc:${COLOR_HEX[c]}">
    <div class="set-head"><span class="count">${set.cards.length}/${col.size}</span><span class="rent">$${set.rent}M</span>${blds}</div>
    <div class="stack">${cards}</div></div>`;
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
    ? p.table.map((s) => setHTML(i, s)).join('')
    : '<span class="empty-note">No properties yet</span>';
  const bank = p.bank.length
    ? p.bank.map((cd) => cardHTML(cd, 'xs', `bank:${i}:${cd.k}`)).join('')
    : '<span class="empty-note">Empty</span>';
  let hand;
  if (p.hand) {
    const size = big ? '' : 'xs';
    hand = `<div class="hand">${p.hand.length ? p.hand.map((cd) => cardHTML(cd, size, isHuman || ui.mode === 'watch' ? `hand:${cd.k}` : null)).join('') : '<span class="empty-note">no cards</span>'}</div>`;
  } else {
    hand = backs(p.handCount, 'xs');
  }
  return `<section class="${classes.join(' ')}" data-key="${pkey}" style="--pc:${pc(i)}">
    <div class="phead">
      <span class="pname">P${i}</span><span class="ptag">${tag}</span>
      <div class="badges">
        <span class="badge sets">Sets <b>${p.sets}/3</b></span>
        <span class="badge">Bank <b>$${p.bankValue}M</b></span>
        <span class="badge">Hand <b>${p.handCount}</b></span>
      </div>
    </div>
    <div class="zone-label">Properties</div><div class="table">${table}</div>
    <div class="zone-label">Bank</div><div class="bank">${bank}</div>
    <div class="zone-label">Hand</div>${hand}
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
  return `<div class="pile-slot"><div class="card back sm"></div>Deck · ${S.deck}</div>
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
    case 'respond': return 'You hold a Just Say No. Use it, or accept.';
    case 'double': return 'You hold a Double The Rent. Play it to double the charge (it uses a play).';
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
    : `<li style="border-left-color:${pc(e.p)}"><span class="who" style="color:${pc(e.p)}">P${e.p}</span>${esc(e.t)}</li>`).join('');
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
  if (actions.length === 1) act(actions[0].i);
  else showPopover(el, actions);
  e.stopPropagation();
}

function showPopover(el, actions) {
  const pop = $('popover');
  const title = el.getAttribute('title') || 'Choose';
  pop.innerHTML = `<div class="ptitle">${esc(title)}</div>` +
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
  $('reveal').onchange = (e) => { ui.reveal = e.target.checked; refresh(); };
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
