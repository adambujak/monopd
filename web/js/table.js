// Drawing the table: players (property sets, bank, hand), the centre strip,
// the status panel and the log.
//
// Every function takes `view`:
//   cards      from createCards()
//   actions    Map of interaction key -> legal actions (what's clickable)
//   human      the human's seat, or null when watching
//   reveal     show other players' hands
//   bankDetail show every bank in full (perfect memory)
//   isFresh    (cardId) -> true if the card just arrived (pop animation)

import { COLOR_HEX, MONEY_COLOR, esc } from './cards.js';

const PLAYER_COLOR = (p) => `var(--p${p})`;
const PAWN = '<svg class="pawn" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="6" r="4.2"/><path d="M8.2 11.5h7.6l2.2 7.5H6z"/><rect x="4" y="18.5" width="16" height="3.5" rx="1.75"/></svg>';
const RESPONSE_BUTTONS = {
  end: ['End turn', ''],
  double: ['Double the rent', 'go'],
  noDouble: ["Don't double", ''],
  jsn: ['Just Say No!', 'jsn'],
  accept: ['Accept', ''],
};

function card(view, c, size, key = null) {
  return view.cards.html(c, { size, key, clickable: view.actions.has(key), fresh: c.id !== undefined && view.isFresh(c.id) });
}

// ---------------------------------------------------------------- players

export function playerHTML(S, view, i) {
  const p = S.players[i];
  const isHuman = view.human === i;
  const key = `player:${i}`;
  const classes = ['player',
    S.active === i && S.current !== null && 'active',
    S.current === i && 'deciding',
    S.winner === i && 'winner',
    view.actions.has(key) && 'clickable'].filter(Boolean).join(' ');

  const sets = sortedSets(view, p.table).map((s) => setHTML(view, i, s)).join('');
  const stars = [0, 1, 2].map((j) => `<span class="star ${j < p.sets ? 'on' : ''}">★</span>`).join('');

  let hand = '';
  if (isHuman && p.hand) {
    const cards = p.hand.map((c) => card(view, c, '', `hand:${c.k}`)).join('') || '<span class="empty-note">no cards</span>';
    hand = `<div class="myhand"><div class="myhand-title">Your hand</div><div class="hand">${cards}</div></div>`;
  } else if (view.reveal && p.hand?.length) {
    hand = `<div class="peek"><span class="peek-title">Hand</span>${p.hand.map((c) => card(view, c, 'xs')).join('')}</div>`;
  }

  return `<section class="${classes}" data-key="${key}" style="--pc:${PLAYER_COLOR(i)}">
    <div class="phead">
      <span class="token">${PAWN}</span><span class="pname">P${i}</span><span class="ptag">${isHuman ? 'You' : 'Random bot'}</span>
      <span class="stars" title="${p.sets} of 3 complete sets">${stars}</span>
      <span class="handcount" title="Cards in hand"><span class="mini-back"></span><b>${p.handCount}</b></span>
    </div>
    <div class="pbody">
      <div class="table">${sets || '<div class="no-props">No properties yet</div>'}</div>
      ${bankHTML(view, i, p, isHuman || view.bankDetail)}
    </div>
    ${hand}
  </section>`;
}

/** Complete sets first, then by how close each set is to complete. */
function sortedSets(view, table) {
  const progress = (s) => s.cards.length / view.cards.colors[s.color].size;
  return [...table].sort((a, b) => (b.complete - a.complete) || (progress(b) - progress(a)) || (a.color - b.color));
}

function setHTML(view, seat, set) {
  const c = set.color;
  const { size, label } = view.cards.colors[c];
  const key = `set:${seat}:${c}`;
  const cls = `${set.complete ? ' complete' : ''}${view.actions.has(key) ? ' clickable' : ''}`;
  const buildings = (set.house ? '<span class="bld" title="House">🏠</span>' : '') + (set.hotel ? '<span class="bld" title="Hotel">🏨</span>' : '');
  const cards = set.cards.map((cd) => card(view, cd, 'sm', `tcard:${seat}:${c}:${cd.k}`)).join('');
  // Dashed slots for the cards still needed.
  const slots = `<div class="card sm ghost" style="--cc:${COLOR_HEX[c]}"><span>${label}</span></div>`.repeat(Math.max(0, size - set.cards.length));
  return `<div class="set${cls}" data-key="${key}" style="--sc:${COLOR_HEX[c]}">
    <div class="set-head"><span class="count">${set.cards.length}/${size}</span><span class="rent">$${set.rent}M</span>${buildings}</div>
    <div class="stack">${cards}${slots}</div>
    ${set.complete ? '<div class="set-flag">SET!</div>' : ''}</div>`;
}

/**
 * A bank. Your own is always open to you; others are squared up like a real
 * pile (count and the newest card on top) unless `open` (perfect memory).
 */
function bankHTML(view, i, p, open) {
  const n = p.bank.length;
  if (!n) return '<div class="bank-box"><div class="bank-title">Bank</div><div class="bank-empty">empty</div></div>';
  if (!open) {
    const layers = Array.from({ length: Math.min(n, 4) - 1 }, (_, j) => `<div class="layer" style="--d:${(j + 1) * 3}px"></div>`).join('');
    return `<div class="bank-box"><div class="bank-title">Bank</div>
      <div class="pile">${layers}${card(view, p.bank[n - 1], 'sm')}</div>
      <div class="bank-count">${n} card${n === 1 ? '' : 's'}</div></div>`;
  }
  const counts = new Map();
  for (const c of p.bank) counts.set(c.k, (counts.get(c.k) || 0) + 1);
  const chips = [...counts]
    .sort(([a], [b]) => view.cards.kinds[b].value - view.cards.kinds[a].value)
    .map(([k, count]) => {
      const K = view.cards.kinds[k];
      const key = `bank:${i}:${k}`;
      const name = K.type === 'money' ? '' : `<small>${esc(K.type === 'rent' ? K.name : K.short)}</small>`;
      const bg = K.type === 'money' ? MONEY_COLOR[K.value] : '#fff';
      return `<div class="bank-chip${view.actions.has(key) ? ' clickable' : ''}" data-key="${key}" title="${esc(K.name)}" style="background:${bg}">
        <b>$${K.value}M</b>${name}${count > 1 ? `<span class="times">×${count}</span>` : ''}</div>`;
    }).join('');
  return `<div class="bank-box open"><div class="bank-title">Bank <b>$${p.bankValue}M</b></div><div class="bank-list">${chips}</div></div>`;
}

// ---------------------------------------------------------------- centre, status, log

export function centerHTML(S, view) {
  const top = S.discardTop !== null ? view.cards.html({ k: S.discardTop }, { size: 'sm' }) : '<div class="card sm empty-slot"></div>';
  const pips = S.phase === 'main'
    ? `<div class="pips">Plays ${[0, 1, 2].map((i) => `<span class="pip ${i < S.playsLeft ? 'full' : ''}"></span>`).join('')}</div>`
    : '';
  const buttons = S.legal.filter((a) => RESPONSE_BUTTONS[a.verb])
    .map((a) => `<button class="${RESPONSE_BUTTONS[a.verb][1]}" data-act="${a.i}">${RESPONSE_BUTTONS[a.verb][0]}</button>`).join('');
  return `<div class="pile-slot">${view.cards.back('sm')}Deck · ${S.deck}</div>
    <div class="pile-slot">${top}Discard · ${S.discard}</div>
    <div class="center-info"><div class="phase-text">${esc(S.phaseText)}</div>${pips}</div>
    <div class="action-bar">${buttons}</div>`;
}

export function statusHTML(S, view, seed) {
  const over = S.current === null;
  return `${over ? `<div class="winner-banner">${esc(S.phaseText)}</div>` : `<div class="status-phase">${esc(S.phaseText)}</div>`}
    <div class="status-hint">${esc(hint(S, view))}</div>
    <div class="status-meta"><span>Turn <b>${S.turn + 1}</b></span><span>Players <b>${S.n}</b></span><span>Seed <b>${seed}</b></span></div>`;
}

function hint(S, view) {
  if (S.current === null) return '';
  if (view.human === null) return 'Bots are playing. Use Pause and Step to go one decision at a time.';
  if (S.current !== view.human) return 'Waiting for the bots…';
  return {
    main: 'Click a highlighted card in your hand to bank it or play it. Click a wild on your table to move it (that uses a play).',
    target: 'Click a highlighted player, property or set.',
    pay: 'Click cards in your bank or on your table to pay. No change is given.',
    respond: 'You hold a Just Say No. Click it in your hand (or the button) to cancel, or accept.',
    double: 'Click the Double The Rent in your hand (or the button) to double the charge. It uses a play.',
    discard: 'You have more than 7 cards. Click cards in your hand to discard them.',
  }[S.phase] || '';
}

export function logHTML(S) {
  return S.log.map((e) => (e.p < 0
    ? `<li class="sys">${esc(e.t)}</li>`
    : `<li style="--lc:${PLAYER_COLOR(e.p)}"><span class="who">P${e.p}</span>${esc(e.t)}</li>`)).join('');
}
