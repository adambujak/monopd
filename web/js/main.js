// Viewer entry point: game loop, controls and click handling.

import { loadEngine } from './engine.js';
import { createCards, esc } from './cards.js';
import { centerHTML, logHTML, playerHTML, statusHTML } from './table.js';

const $ = (id) => document.getElementById(id);

const ui = { mode: 'play', seed: 0, reveal: false, bankDetail: false, paused: false, speed: 55, timer: null };
let engine, cards, S;
let actions = new Map(); // interaction key -> legal actions
let seenCards = new Set(); // card ids on screen last render, for the pop-in animation

// The human plays seat 0 in Play mode.
const human = () => (ui.mode === 'play' ? 0 : null);
const viewSeat = () => (ui.mode === 'watch' || ui.reveal ? -1 : 0);
const botDelay = () => Math.round(1600 * (1 - ui.speed / 100) ** 2 + 40);

async function init() {
  try {
    engine = await loadEngine();
  } catch (e) {
    showError('Could not load monopd.wasm. Run web/build.sh, then serve web/ over HTTP.');
    throw e;
  }
  cards = createCards(engine.catalog(), await loadSkin());
  bindControls();
  $('seed').value = randomSeed();
  newGame();
}

/** Optional local card images: web/cards/skin.json maps card slugs to files. */
async function loadSkin() {
  try {
    const r = await fetch('cards/skin.json', { cache: 'no-store' });
    return r.ok ? await r.json() : {};
  } catch {
    return {};
  }
}

// ---------------------------------------------------------------- game loop

function newGame() {
  clearTimeout(ui.timer);
  ui.seed = +$('seed').value >>> 0;
  engine.newGame(+$('players').value, ui.seed);
  seenCards = new Set();
  refresh();
}

function refresh() {
  S = engine.state(viewSeat());
  // Everyone targeted is asked about Just Say No (and the renter about Double
  // The Rent), card or not. Bots answer these at once, and when "no" is your
  // only possible answer it's given for you.
  while (S.current !== null && (S.phase === 'respond' || S.phase === 'double')) {
    if (S.current !== human()) engine.botStep();
    else if (S.legal.length === 1) engine.step(S.legal[0].i);
    else break;
    S = engine.state(viewSeat());
  }
  render();
  scheduleBot();
}

function scheduleBot() {
  clearTimeout(ui.timer);
  if (S.current === null) return;
  const botsTurn = ui.mode === 'watch' ? !ui.paused : S.current !== human();
  if (botsTurn) ui.timer = setTimeout(() => { engine.botStep(); refresh(); }, botDelay());
}

function act(index) {
  hidePopover();
  if (!engine.step(index)) showError('Illegal action');
  refresh();
}

// ---------------------------------------------------------------- rendering

function render() {
  actions = human() === S.current ? groupByKey(S.legal) : new Map();
  const view = {
    cards,
    actions,
    human: human(),
    reveal: ui.reveal,
    bankDetail: ui.bankDetail,
    isFresh: (id) => seenCards.size > 0 && !seenCards.has(id),
  };
  const others = [...Array(S.n).keys()].filter((i) => i !== view.human);
  $('opponents').innerHTML = others.map((i) => playerHTML(S, view, i)).join('');
  $('me').innerHTML = view.human !== null ? playerHTML(S, view, view.human) : '';
  $('center').innerHTML = centerHTML(S, view);
  $('status').innerHTML = statusHTML(S, view, ui.seed);

  const log = $('log');
  const atBottom = log.scrollHeight - log.scrollTop - log.clientHeight < 40;
  log.innerHTML = logHTML(S);
  if (atBottom) log.scrollTop = log.scrollHeight;

  $('legal-list').innerHTML = S.legal.map((a) => `<button data-act="${a.i}">${esc(a.label)}</button>`).join('')
    || '<span class="empty-note">Nothing to do right now</span>';
  $('pause').textContent = ui.paused ? 'Resume' : 'Pause';

  seenCards = new Set(S.players.flatMap((p) => [...p.bank, ...p.table.flatMap((s) => s.cards), ...(p.hand || [])].map((c) => c.id)));
}

/** Attach each legal action to the thing you click to take it. */
function groupByKey(legal) {
  const map = new Map();
  for (const a of legal) {
    const key = interactionKey(a);
    if (key) map.set(key, [...(map.get(key) || []), a]);
  }
  return map;
}

function interactionKey(a) {
  switch (a.verb) {
    // Cards in your hand (Just Say No and Double The Rent too, when offered).
    case 'bank': case 'place': case 'play': case 'rent': case 'discard': case 'double': case 'jsn':
      return `hand:${a.kind}`;
    case 'move': case 'targetProperty': case 'payProperty': return `tcard:${a.seat}:${a.color}:${a.kind}`;
    case 'targetSet': case 'payBuilding': return `set:${a.seat}:${a.color}`;
    case 'payBank': return `bank:${a.seat}:${a.kind}`;
    case 'targetPlayer': return `player:${a.seat}`;
    default: return null; // buttons only: end turn, accept, don't double
  }
}

// ---------------------------------------------------------------- clicking

function onBoardClick(e) {
  const button = e.target.closest('[data-act]');
  if (button) { act(+button.dataset.act); return; }
  if (!actions.size) return;
  // The innermost clickable element wins (a card inside a set inside a player).
  let el = e.target.closest('[data-key]');
  while (el && !actions.has(el.dataset.key)) el = el.parentElement?.closest('[data-key]');
  if (!el) { hidePopover(); return; }
  e.stopPropagation();
  const options = actions.get(el.dataset.key);
  const only = options.length === 1 ? options[0] : null;
  // Banking an action or rent card throws its effect away, so always confirm.
  if (only?.verb === 'bank' && cards.kinds[only.kind].type !== 'money') showPopover(el, options, whyNotPlayable(only.kind));
  else if (only) act(only.i);
  else showPopover(el, options);
}

function whyNotPlayable(k) {
  const K = cards.kinds[k];
  switch (K.slug) {
    case 'action-double-the-rent': return 'You play this right after charging rent: play a rent card first and you’ll be offered the double.';
    case 'action-just-say-no': return 'You play this in response, when someone targets you.';
    case 'action-house': case 'action-hotel': return 'This needs a complete set (and a hotel needs a house). Railroads and utilities can’t have buildings.';
    default: return K.type === 'rent' ? 'You need a property of a matching colour on the table to charge this rent.' : 'There’s no valid target for this right now.';
  }
}

function showPopover(el, options, note = '') {
  const pop = $('popover');
  pop.innerHTML = `<div class="ptitle">${esc(el.getAttribute('title') || 'Choose')}</div>${note ? `<div class="pnote">${esc(note)}</div>` : ''}`
    + options.map((a) => `<button data-act="${a.i}">${esc(a.label)}</button>`).join('');
  pop.hidden = false;
  const r = el.getBoundingClientRect();
  const x = Math.min(r.left, window.innerWidth - pop.offsetWidth - 10);
  let y = r.bottom + 6;
  if (y + pop.offsetHeight > window.innerHeight - 10) y = Math.max(10, r.top - pop.offsetHeight - 6);
  pop.style.left = `${Math.max(10, x)}px`;
  pop.style.top = `${y}px`;
}

function hidePopover() { $('popover').hidden = true; }

function showError(msg) {
  const el = $('error');
  el.textContent = msg;
  el.hidden = false;
  setTimeout(() => { el.hidden = true; }, 3500);
}

// ---------------------------------------------------------------- controls

function randomSeed() { return Math.floor(Math.random() * 1e6); }

function setMode(mode) {
  ui.mode = mode;
  ui.paused = false;
  document.body.className = `mode-${mode}`;
  for (const b of $('mode').children) b.classList.toggle('on', b.dataset.mode === mode);
}

function bindControls() {
  setMode('play');
  ui.speed = +$('speed').value;
  const onAct = (e) => { const b = e.target.closest('[data-act]'); if (b) act(+b.dataset.act); };

  $('mode').addEventListener('click', (e) => { const b = e.target.closest('button'); if (b) { setMode(b.dataset.mode); newGame(); } });
  $('new').onclick = newGame;
  $('dice').onclick = () => { $('seed').value = randomSeed(); newGame(); };
  $('players').onchange = newGame;
  $('speed').oninput = (e) => { ui.speed = +e.target.value; };
  $('pause').onclick = () => { ui.paused = !ui.paused; refresh(); };
  $('stepbtn').onclick = () => { ui.paused = true; engine.botStep(); refresh(); };
  $('reveal').onchange = (e) => { ui.reveal = e.target.checked; refresh(); };
  $('bankinfo').onchange = (e) => { ui.bankDetail = e.target.checked; refresh(); };
  document.querySelector('.board').addEventListener('click', onBoardClick);
  $('popover').addEventListener('click', onAct);
  $('legal-list').addEventListener('click', onAct);
  document.addEventListener('click', (e) => { if (!e.target.closest('#popover, .board')) hidePopover(); });
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') hidePopover();
    if (e.key === ' ' && ui.mode === 'watch' && e.target.tagName !== 'INPUT') { e.preventDefault(); $('pause').click(); }
  });
}

init();
