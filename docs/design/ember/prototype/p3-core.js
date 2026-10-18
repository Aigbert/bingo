
/* ---------- core: colour, glyphs, width, segments, rows, time, motion ---------- */
const $ = (q, el = document) => el.querySelector(q);
const $$ = (q, el = document) => Array.from(el.querySelectorAll(q));
const esc = t => String(t).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
let S = null;
let SNAP = false;

/* Ember tokens: the user's terminal ground and ink, one accent for attention */
const BASE = {
  light: { bg: '#f9f4f2', fg: '#3a342e', ember: '#b04c14', glow: '#ef7a2c', coal: '#b04c14', amber: '#9a6200', amberGlow: '#e0a01c',
    good: '#2b7d3c', bad: '#b3261e', plan: '#2f62a8', auto: '#7445c0', code: '#8a5520',
    kw: '#8a3fa8', str: '#4d7a1c', num: '#b04c14', fn: '#2f62a8', ty: '#11806c',
    crew: ['#5d7a0e', '#11806c', '#6a4fc2', '#a23a9e', '#b8456e', '#2a72b5'] },
  dark: { bg: '#2c2a2e', fg: '#e6e2dc', ember: '#f39050', glow: '#ff9b4a', coal: '#d8622e', amber: '#f2b84b', amberGlow: '#f2b84b',
    good: '#7fd38a', bad: '#f0706a', plan: '#8ab4f0', auto: '#b89cf5', code: '#e9c48f',
    kw: '#d49cf0', str: '#b5d98a', num: '#f3a06a', fn: '#8ab4f0', ty: '#6fd0bd',
    crew: ['#c2d86a', '#5cc8b4', '#b39cf2', '#dc8cf0', '#f08cb8', '#7cb8f0'] },
};
function rgb(h) { const n = parseInt(h.slice(1), 16); return [n >> 16 & 255, n >> 8 & 255, n & 255]; }
function hex(a) { return '#' + a.map(v => Math.round(clamp(v, 0, 255)).toString(16).padStart(2, '0')).join(''); }
let PAL = null;
function col(c) {
  if (!c) return PAL.fg;
  if (c[0] === '#') return c;
  if (c.startsWith('crew')) return PAL.crew[(+c.slice(4) || 0) % PAL.crew.length];
  return PAL[c] || PAL.fg;
}
function mix(a, b, t) {
  t = clamp(t, 0, 1);
  const A = rgb(col(a)), B = rgb(col(b));
  return hex(A.map((v, i) => v + (B[i] - v) * t));
}
function palette(n) {
  const p = Object.assign({}, BASE[n], { crew: BASE[n].crew.slice() });
  const prev = PAL; PAL = p;
  const dk = n === 'dark';
  p.muted = mix(p.fg, p.bg, dk ? .30 : .32);
  p.dim = mix(p.fg, p.bg, dk ? .52 : .55);
  p.faint = mix(p.fg, p.bg, dk ? .80 : .84);
  p.you = mix(p.bg, p.fg, dk ? .065 : .052);
  p.well = mix(p.bg, p.fg, dk ? .038 : .03);
  p.wellDeep = mix(p.bg, p.fg, dk ? .085 : .07);
  p.pick = mix(p.you, p.glow, dk ? .20 : .17);
  p.pickBg = mix(p.bg, p.glow, dk ? .16 : .13);
  p.addTint = mix(p.bg, p.good, dk ? .14 : .12);
  p.delTint = mix(p.bg, p.bad, dk ? .14 : .10);
  p.addWord = mix(p.bg, p.good, dk ? .30 : .26);
  p.delWord = mix(p.bg, p.bad, dk ? .30 : .22);
  p.badTint = mix(p.bg, p.bad, dk ? .12 : .08);
  p.amberTint = mix(p.you, p.amberGlow, dk ? .12 : .12);
  p.glowTint = mix(p.bg, p.glow, dk ? .12 : .10);
  PAL = prev;
  return p;
}
const PALS = { light: palette('light'), dark: palette('dark') };
PAL = PALS.light;

/* glyphs: one table, one ASCII twin; embers are single braille dots so they move a quarter cell at a time */
const GU = { lamp: '●', bullet: '•', sep: ' · ', ok: '✓', still: '◌', edge: '▔', base: '▁', thin: '─', vbar: '│', quote: '▎',
  caret: '▍', ell: '…', arrow: '→', up: '↑', down: '↓', minus: '−', times: '×', dot: '·', ret: '⏎', pin: '◆', side: '▍',
  think: '≈', spin: '⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏' };
const GA = { lamp: '*', bullet: '-', sep: ' - ', ok: '+', still: 'o', edge: '-', base: '_', thin: '-', vbar: '|', quote: '|',
  think: ':', caret: '|', ell: '...', arrow: '->', up: '^', down: 'v', minus: '-', times: 'x', dot: '.', ret: 'enter', pin: '*', side: '|',
  spin: '-\\|/' };
const g = k => (S && S.ascii ? GA : GU)[k];
const BRAILLE_COL = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
const dotAt = (sx, sy) => String.fromCodePoint(0x2800 + BRAILLE_COL[sx & 1][clamp(sy, 0, 3)]);
const KEYA = [['⇧⏎', 'shift+enter'], ['⇧tab', 'shift+tab'], ['⏎', 'enter'], ['↑↓', 'up/down'], ['←→', 'left/right'], ['↑', 'up'], ['↓', 'down'], ['⌫', 'backspace']];
function keyText(k) {
  if (!S || !S.ascii) return k;
  for (const [a, b] of KEYA) k = k.split(a).join(b);
  return k;
}

/* display width */
function wcw(cp) {
  return (cp >= 0x1100 && cp <= 0x115f) || (cp >= 0x2e80 && cp <= 0xa4cf) || (cp >= 0xac00 && cp <= 0xd7a3) ||
    (cp >= 0xf900 && cp <= 0xfaff) || (cp >= 0xfe30 && cp <= 0xfe4f) || (cp >= 0xff00 && cp <= 0xff60) ||
    (cp >= 0xffe0 && cp <= 0xffe6) || (cp >= 0x20000 && cp <= 0x3fffd) ? 2 : 1;
}
function sw(t) { let w = 0; for (const ch of t) w += wcw(ch.codePointAt(0)); return w; }

/* styled segments: {t, fg, b, i, bg, d} — d marks decoration that a reveal does not count */
function s(t, fg, o) {
  const x = { t: String(t), fg };
  if (typeof o === 'string') { if (o.includes('b')) x.b = 1; if (o.includes('i')) x.i = 1; }
  else if (o) Object.assign(x, o);
  return x;
}
const segsW = segs => segs.reduce((w, x) => w + sw(x.t), 0);
const CJK = '\\u1100-\\u115f\\u2e80-\\ua4cf\\uac00-\\ud7a3\\uf900-\\ufaff\\ufe30-\\ufe4f\\uff00-\\uff60\\uffe0-\\uffe6';
const TOK = new RegExp(`\\n|[ \\t]+|[${CJK}]|[^\\s${CJK}]+`, 'gu');
function trimEnd(L) {
  while (L.length) { const x = L[L.length - 1]; x.t = x.t.replace(/[ \t]+$/, ''); if (x.t) break; L.pop(); }
}
/* a line never starts with closing punctuation or ends with opening punctuation */
const NO_START = new Set([...'，。、；：！？）」』】》〉〕］｝”’,.;:!?)]}%']);
const NO_END = new Set([...'（「『【《〈〔［｛“‘([{']);
const ONE_CJK = new RegExp(`^[${CJK}]$`, 'u');
/* words that must travel together: a Latin run split only by styling, punctuation and its neighbour */
function units(segs) {
  const out = []; let cur = null, prev = null;
  for (const seg of segs) for (const m of seg.t.matchAll(TOK)) {
    const tok = m[0];
    if (tok === '\n' || tok[0] === ' ' || tok[0] === '\t') { out.push({ sp: tok, seg }); cur = prev = null; continue; }
    const glue = prev != null && (NO_START.has([...tok][0]) || NO_END.has([...prev].pop()) || (!ONE_CJK.test(tok) && !ONE_CJK.test(prev)));
    if (!glue) out.push(cur = { parts: [], w: 0 });
    cur.parts.push({ t: tok, seg }); cur.w += sw(tok); prev = tok;
  }
  return out;
}
function wrapSegs(segs, w) {
  if (w < 2) return [segs];
  const lines = [[]]; let cw = 0, soft = false;
  const push = (t, seg) => { lines[lines.length - 1].push(Object.assign({}, seg, { t })); cw += sw(t); };
  const brk = isSoft => { trimEnd(lines[lines.length - 1]); lines.push([]); cw = 0; soft = isSoft; };
  /* only a unit wider than the whole line is cut, wherever the line ends */
  const cut = (tok, seg) => {
    while (sw(tok) > w - cw) {
      let head = '', hw = 0;
      for (const ch of tok) { const c = wcw(ch.codePointAt(0)); if (hw + c > w - cw) break; head += ch; hw += c; }
      if (!head) { if (cw > 0) { brk(true); continue; } head = [...tok][0]; }
      push(head, seg); tok = tok.slice(head.length);
      if (tok) brk(true);
    }
    if (tok) push(tok, seg);
  };
  for (const u of units(segs)) {
    if (u.sp === '\n') { brk(false); continue; }
    if (u.sp != null) {
      if (cw === 0 && soft) continue;
      if (cw + sw(u.sp) > w) { brk(true); continue; }
      push(u.sp, u.seg); continue;
    }
    if (u.w > w - cw && cw > 0 && u.w <= w) brk(true);
    for (const p of u.parts) cut(p.t, p.seg);
  }
  trimEnd(lines[lines.length - 1]);
  return lines;
}
function fitSegs(segs, w) {
  if (w <= 0) return [];
  if (segsW(segs) <= w) return segs;
  const ell = g('ell'), ew = sw(ell), out = []; let cw = 0;
  for (const x of segs) {
    let t = '';
    for (const ch of x.t) {
      const c = wcw(ch.codePointAt(0));
      if (cw + c > w - ew) { out.push(Object.assign({}, x, { t: t.replace(/ +$/, '') }), s(ell, 'dim')); return out; }
      t += ch; cw += c;
    }
    out.push(Object.assign({}, x, { t }));
  }
  return out;
}
const padTo = (segs, w) => { const n = w - segsW(segs); return n > 0 ? segs.concat([s(' '.repeat(n))]) : fitSegs(segs, w); };
/* inline markdown: `code` and **bold** */
function inl(str, fg, codeFg = 'code') {
  const out = [], re = /`([^`]+)`|\*\*([^*]+)\*\*/g; let i = 0, m;
  while ((m = re.exec(str))) {
    if (m.index > i) out.push(s(str.slice(i, m.index), fg));
    out.push(m[1] != null ? s(m[1], codeFg) : s(m[2], fg, 'b'));
    i = re.lastIndex;
  }
  if (i < str.length) out.push(s(str.slice(i), fg));
  return out;
}

/* a row of cells; box glyphs draw as CSS so they meet edge to edge */
const BOX = { '▎': 'v25', '▍': 'v38', '▌': 'v50', '█': 'full', '▀': 'up', '▄': 'dn', '▕': 'r25', '▔': 'e8', '▁': 'b8', '━': 'hh', '─': 'h', '│': 'vl' };
class Row {
  constructor(w, fill) { this.w = w; this.c = Array.from({ length: w }, () => ({ ch: ' ', fg: null, bg: fill || null })); }
  put(x, segs) {
    for (const seg of segs) {
      for (const ch of seg.t) {
        const cw = wcw(ch.codePointAt(0));
        if (x < 0) { x += cw; continue; }
        if (x + cw > this.w) return x;
        const c = this.c[x];
        c.ch = ch; c.fg = seg.fg || null; c.b = seg.b; c.i = seg.i; c.u = seg.u;
        if (seg.bg) c.bg = seg.bg;
        if (cw === 2) { const n = this.c[x + 1]; n.ch = ''; n.fg = c.fg; if (seg.bg) n.bg = seg.bg; }
        x += cw;
      }
    }
    return x;
  }
  right(segs, m = 2) { return this.put(this.w - m - segsW(segs), segs); }
  band(a, b, bg) { if (b <= 0) b = this.w + b; for (let x = Math.max(0, a); x < Math.min(this.w, b); x++) this.c[x].bg = bg; }
  html() {
    const C = this.c, W = this.w; let out = '', i = 0;
    while (i < W) {
      const c = C[i], bx = BOX[c.ch];
      if (bx) {
        let j = i + 1;
        while (j < W && C[j].ch === c.ch && C[j].fg === c.fg && C[j].bg === c.bg) j++;
        out += `<i class="bx ${bx}" style="width:${j - i}ch;color:${col(c.fg || 'fg')}${c.bg ? ';background-color:' + col(c.bg) : ''}"></i>`;
        i = j; continue;
      }
      let j = i, key = null, first = null;
      while (j < W) {
        const d = C[j];
        if (d.bg !== c.bg || BOX[d.ch]) break;
        if (d.ch !== ' ' && d.ch !== '') {
          const k = (d.fg || '') + '|' + (d.b ? 1 : 0) + (d.i ? 1 : 0) + (d.u ? 1 : 0);
          if (key === null) { key = k; first = d; } else if (k !== key) break;
        }
        j++;
      }
      let txt = '';
      for (let k = i; k < j; k++) txt += cellH(C, k, i);
      out += `<span style="width:${j - i}ch${sty(first, c.bg)}">${txt}</span>`;
      i = j;
    }
    return out;
  }
}
function cellH(C, k, start) {
  const ch = C[k].ch;
  if (ch === ' ') return ' ';
  if (ch === '') return k > start && C[k - 1].ch && wcw(C[k - 1].ch.codePointAt(0)) === 2 ? '' : ' ';
  const cp = ch.codePointAt(0), e = ch === '&' ? '&amp;' : ch === '<' ? '&lt;' : ch === '>' ? '&gt;' : ch;
  if (wcw(cp) === 2) return `<i class="w2">${e}</i>`;
  return cp > 126 ? `<i class="w1">${e}</i>` : e;
}
function sty(c, bg) {
  let o = '';
  if (c) { o += ';color:' + col(c.fg || 'fg'); if (c.b) o += ';font-weight:700'; if (c.i) o += ';font-style:italic'; if (c.u) o += ';text-decoration:underline;text-underline-offset:3px;text-decoration-thickness:1px'; }
  if (bg) o += ';background-color:' + col(bg);
  return o;
}

/* a line before it becomes a row: placed segments, a right side, bands, a fill */
const ln = (o = {}) => Object.assign({ p: [], r: null, rm: 2, bands: [], fill: null, click: null, cur: false }, o);
const L = (x, segs, o) => ln(Object.assign({ p: [[x, segs]] }, o));
const BLANK = () => ln();
function rowOf(l, W) {
  const R = new Row(W, l.fill);
  for (const [a, b, bg] of l.bands) R.band(a, b, bg);
  for (const [x, segs] of l.p) R.put(x, segs);
  if (l.r) R.right(l.r, l.rm);
  return R;
}

/* time */
let NOWFN = () => performance.now();
const NOW = () => NOWFN();
function fmtDur(ms) {
  const t = Math.max(0, Math.floor(ms / 1000));
  if (t < 60) return t + 's';
  if (t < 3600) return Math.floor(t / 60) + 'm ' + String(t % 60).padStart(2, '0') + 's';
  return Math.floor(t / 3600) + 'h ' + String(Math.floor(t / 60) % 60).padStart(2, '0') + 'm';
}
const fmtK = n => n < 1000 ? String(Math.round(n)) : (n < 10000 ? (n / 1000).toFixed(1) : Math.round(n / 1000)) + 'k';
const plural = (n, w) => `${n} ${w}${n === 1 ? '' : 's'}`;

/* motion: pure functions of the clock */
const breathe = (now, per = 1600) => S && S.reduced ? 0 : (1 - Math.cos(now / per * 2 * Math.PI)) / 2;
function spinG(now, at) {
  if (S.reduced) return g('still');
  const f = [...g('spin')], iv = S.ascii ? 120 : 80;
  return f[Math.floor(Math.max(0, now - at) / iv) % f.length];
}
function shimmer(text, now, fg = 'fg') {
  if (S.reduced) return [s(text, fg)];
  const ch = [...text], n = ch.length, pos = (now % 2400) / 2400 * (n + 8) - 4;
  return ch.map((c, i) => { const k = Math.max(0, 1 - Math.abs(i - pos) / 3); return s(c, k > 0 ? mix(fg, 'glow', k) : fg); });
}

const easeOut = t => 1 - Math.pow(1 - clamp(t, 0, 1), 3);
const easeInOut = t => { t = clamp(t, 0, 1); return t < .5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2; };
/* 0 → 1 over ms after at; 1 at once when motion is reduced */
const prog = (now, at, ms) => S && S.reduced ? 1 : clamp((now - at) / ms, 0, 1);
/* the caret: a soft fade, never a hard blink */
const caretInk = (now, base = 'glow', ground = 'you') => S.reduced ? col(base) : mix(base, ground, .78 * (1 - Math.cos(now / 1200 * 2 * Math.PI)) / 2);
