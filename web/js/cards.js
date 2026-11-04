// Card faces. Every card is drawn from the engine's catalog; an optional
// web/cards/skin.json swaps in local images by card slug.

export const COLOR_HEX = ['#9a5b34', '#7fd0f0', '#ec4fa0', '#ff8f1f', '#ef3b3b', '#ffd23f', '#1fae5b', '#2457c5', '#353a48', '#a9d18e'];
export const MONEY_COLOR = { 1: '#fff0a0', 2: '#ffc4b2', 3: '#c3efc6', 4: '#bfe2ff', 5: '#dccbff', 10: '#ffcf5c' };
const COLOR_INK = ['#fff', '#0b3550', '#fff', '#fff', '#fff', '#4a3a00', '#fff', '#fff', '#fff', '#21401a'];

// Action card art: [icon, background, one-line rules reminder].
const ACTION_ART = {
  'action-deal-breaker': ['💥', '#7b3fe4', 'Steal a complete set'],
  'action-just-say-no': ['✋', '#e5383b', 'Cancel an action against you'],
  'action-sly-deal': ['🦊', '#ff7b00', 'Steal one property'],
  'action-forced-deal': ['🔁', '#1f8ef1', 'Swap a property with someone'],
  'action-debt-collector': ['💼', '#0fa37f', 'One player pays you $5M'],
  'action-it-s-my-birthday': ['🎂', '#f0479a', 'Everyone pays you $2M'],
  'action-double-the-rent': ['×2', '#f4a100', 'Play with a rent card'],
  'action-house': ['🏠', '#2bb24c', '+$3M rent on a full set'],
  'action-hotel': ['🏨', '#d6336c', '+$4M rent on a full set'],
  'action-pass-go': ['🎲', '#ff5a5f', 'Draw 2 cards'],
};

export const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

/**
 * Card drawing for one catalog. `html(card, opts)` draws a face:
 *   card: { k: kind index, id?: card id }
 *   opts: { size: '' | 'sm' | 'xs', key: interaction key, clickable, fresh }
 */
export function createCards(catalog, skin = {}) {
  const kinds = catalog.kinds;
  const colors = catalog.colors;
  const bySlug = new Map(kinds.map((K, i) => [K.slug, i]));
  const label = (c) => colors[c].label;

  const coin = (v) => `<div class="coin">${v}<small>M</small></div>`;
  const splitBg = (cs) => (cs.length > 2
    ? `linear-gradient(90deg, ${COLOR_HEX.map((h, i) => `${h} ${i * 10}% ${(i + 1) * 10}%`).join(',')})`
    : `linear-gradient(115deg, ${COLOR_HEX[cs[0]]} 50%, ${COLOR_HEX[cs[1]]} 50%)`);

  function rentRows(c) {
    const { size, rent } = colors[c];
    return rent.map((amount, i) => {
      const dots = Array.from({ length: size }, (_, j) => `<i class="${j <= i ? 'on' : ''}"></i>`).join('');
      return `<div class="row"><span class="dots">${dots}</span><b>$${amount}M</b></div>`;
    }).join('');
  }

  // Each face returns [inner html, inline style].
  const faces = {
    property(K) {
      const c = K.colors[0];
      const tall = colors[c].size > 3; // railroads: no room for the "Rent" label
      return [`<div class="head"><span>${label(c)}</span></div>
        <div class="body${tall ? ' tight' : ''}">${tall ? '' : '<div class="tag">Rent</div>'}${rentRows(c)}</div>${coin(K.value)}`,
      `--cc:${COLOR_HEX[c]};--ci:${COLOR_INK[c]}`];
    },
    wild(K) {
      const any = K.colors.length > 2;
      const body = any
        ? '<div class="anyc">Any colour</div><div class="tag">No cash value</div>'
        : K.colors.map((c) => `<div class="chip" style="background:${COLOR_HEX[c]};color:${COLOR_INK[c]}">${label(c)}<small>$${colors[c].rent.join('·')}M</small></div>`).join('');
      return [`<div class="head wildhead" style="background:${splitBg(K.colors)}"><span class="sticker">WILD</span></div>
        <div class="body">${body}</div>${any ? '' : coin(K.value)}`, ''];
    },
    action(K) {
      const [icon, color, tip] = ACTION_ART[K.slug] || ['?', '#555', ''];
      const textIcon = /^[\x20-\x7e×]+$/.test(icon) ? ' txt' : '';
      return [`<div class="burst${textIcon}"><span>${icon}</span></div><div class="ribbon">${K.short}</div><div class="tip">${tip}</div>${coin(K.value)}`,
        `--ac:${color}`];
    },
    rent(K) {
      const cs = K.colors, any = cs.length > 2;
      const disc = any ? `conic-gradient(${COLOR_HEX.join(',')}, ${COLOR_HEX[0]})` : splitBg(cs);
      const tip = any ? 'Any colour · one player pays' : `${label(cs[0])} or ${label(cs[1])} · all players pay`;
      return [`<div class="burst disc" style="background:${disc}">${any ? '<span>ANY</span>' : ''}</div>
        <div class="ribbon">${K.short}</div><div class="tip">${tip}</div>${coin(K.value)}`,
      `--ac:${any ? '#30364f' : '#fff7e6'}${any ? ';--tip:#fff' : ''}`];
    },
    money(K) {
      return [`<div class="note"><span class="corner tl">${K.value}</span><span class="corner br">${K.value}</span>
        <div class="medal">$${K.value}<small>M</small></div></div>`, `--mc:${MONEY_COLOR[K.value]}`];
    },
  };

  function html(card, { size = '', key = null, clickable = false, fresh = false } = {}) {
    const K = kinds[card.k];
    const cls = `${size}${clickable ? ' clickable' : ''}${fresh ? ' fresh' : ''}`;
    const attrs = `${key ? `data-key="${key}"` : ''} title="${esc(K.name)}"`;
    const img = skin[K.slug];
    if (img) return `<div class="card skin ${cls}" ${attrs}><img src="cards/${esc(img)}" alt="${esc(K.name)}"></div>`;
    const [inner, style] = faces[K.type](K);
    return `<div class="card ${K.type} ${cls}" style="${style}" ${attrs}>${inner}</div>`;
  }

  function back(size = '') {
    const face = skin.back ? `<img src="cards/${esc(skin.back)}" alt="">` : '<span>deal</span>';
    return `<div class="card back ${skin.back ? 'skin ' : ''}${size}">${face}</div>`;
  }

  return { html, back, kinds, colors, kindOf: (slug) => bySlug.get(slug) };
}
