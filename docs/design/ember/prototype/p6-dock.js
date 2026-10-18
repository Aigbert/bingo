/* ---------- dock: the one place you act; it changes shape, it never stacks ---------- */
const DOCK_CAP = .66;                      /* the dock never takes more of the screen than this */
const LIST_MAX = 8;                        /* rows a list shows before it scrolls */
const GUARD_MS = 400;                      /* an ask ignores keys this long after it opens */

function dockLines(now, W, H) {
  const cap = Math.max(6, Math.floor(H * DOCK_CAP) - 2), d = S.dock;
  const body = !d ? composerLines(now, W, cap) : d.kind === 'ask' ? askLines(d, now, W)
    : d.kind === 'question' ? questionLines(d, now, W) : d.kind === 'find' ? findLines(d, now, W) : sheetLines(d, now, W, cap);
  const all = [edgeLine(now, W), ...body.slice(-cap), ln()];
  for (const l of all) if (!l.fill) l.fill = 'you';
  return growDock(all, now);
}
/* a dock that grows rises a row at a time; one that shrinks lets go at once */
function growDock(all, now) {
  const G = S.grow || (S.grow = { from: all.length, to: all.length, at: 0 });
  if (all.length !== G.to) { G.from = all.length > G.to ? shownRows(G, now) : all.length; G.to = all.length; G.at = now; }
  const h = shownRows(G, now);
  return h >= all.length ? all : [all[0], ...all.slice(all.length - h + 1)];
}
const shownRows = (G, now) => SNAP || S.reduced ? G.to : Math.round(G.from + (G.to - G.from) * easeOut((now - G.at) / Math.max(1, Math.abs(G.to - G.from) * 14)));

/* the dock's top edge says what bingo is doing; it is the only rule on the screen */
function edgeLine(now, W) {
  const l = ln(), d = S.dock, t = liveTurn(), E = g('edge');
  if (d && (d.kind === 'ask' || d.kind === 'question')) {
    const ink = d.danger ? 'bad' : mix('amberGlow', 'you', .35 * breathe(now, 1400));
    l.p.push([0, [s(E.repeat(W), ink)]]);
    return l;
  }
  const ty = (viewOf() || {}).typing;
  if (ty) { l.p.push([0, cometSegs(now, W, ty.at, crewInk(ty.name))]); return l; }
  if (t && t.state === 'live' && !t.retry) { l.p.push([0, cometSegs(now, W, t.t0, viewInk())]); return l; }
  if (t && t.retry) { l.p.push([0, [s(E.repeat(W), mix('amber', 'you', .45))]]); return l; }
  const f = S.edge;
  if (f && f.at != null && f.kind !== 'none') {
    const ms = f.kind === 'mode' ? 900 : 1100, k = (now - f.at) / ms;
    if (k < 1) l.p.push([0, [s(E.repeat(W), mix(f.ink || (f.kind === 'bad' ? 'bad' : 'good'), 'you', S.reduced ? .2 : easeInOut(k)))]]);
  }
  return l;
}
/* busy: an ember comet runs along a faint line, head bright, tail cooling */
function cometSegs(now, W, at, ink = 'glow') {
  const E = g('edge'), len = Math.min(22, Math.round(W * .22)), per = 2200;
  if (S.reduced) return [s(E.repeat(W), mix('faint', ink, .4))];
  const k = ((now - at) % per) / per, head = Math.round(easeInOut(k) * (W + len)) - 1, out = [];
  for (let x = 0; x < W; x++) {
    const dd = head - x, a = dd >= 0 && dd < len ? Math.pow(1 - dd / len, 1.7) : 0;
    out.push(s(E, a > 0 ? mix('faint', ink, a) : 'faint'));
  }
  return out;
}

/* ----- the composer: what you type, what waits, what the slash and the at offer ----- */
function composerLines(now, W, cap) {
  const out = [], pal = paletteOf(), men = !pal && mentionOf();
  if (pal) out.push(...paletteRows(pal, now, W, cap - 3));
  else if (men) out.push(...mentionRows(men, now, W));
  else out.push(...queueRows(now, W));
  out.push(...inputRows(now, W));
  return out;
}
function inputSegs(text) {
  const out = [], m = text.match(/^([/!])(\S*)/);
  let rest = text;
  if (m) { out.push(s(m[1], m[1] === '!' ? 'code' : 'glow'), s(m[2], m[1] === '!' ? 'code' : 'fg', m[1] === '/' ? 'b' : '')); rest = text.slice(m[0].length); }
  for (const part of rest.split(/(@[\w./-]+)/)) if (part) out.push(s(part, part[0] === '@' && part.length > 1 ? 'plan' : 'fg'));
  return out;
}
function placeholder() {
  if (S.placeholder) return S.placeholder;
  const v = viewOf(), t = liveTurn(), who = worker();
  if (v && v.kind === 'room') return v.closed ? 'This room is closed' : `Post to ${v.name}, @ to call on someone`;
  if (t && t.state === 'asking') return `Answer above, or type to tell ${who} something else`;
  if (t) return v ? `Steer ${who}, or queue what comes next` : 'Steer, or queue what comes next';
  if (v) return `Message ${who}`;
  return S.log.length ? 'Reply, or / for commands' : 'Ask anything, or / for commands';
}
function inputRows(now, W) {
  const w = W - TX - RM, typing = S.focus === 'input' && (!S.dock || S.dock.kind !== 'sheet');
  const caret = typing ? s(g('caret'), caretInk(now)) : s(' ');
  if (!S.input && !S.chips.length) return [L(TX, [caret, s(placeholder(), 'dim')])];
  const rows = wrapSegs([...chipSegs(S.chips), ...inputSegs(S.input)], w - 1);
  rows[rows.length - 1].push(caret);
  const shown = rows.slice(-8), out = [];
  if (rows.length > 8) out.push(L(TX, [s(`${g('ell')} ${rows.length - 8} lines above`, 'dim')]));
  shown.forEach(r => out.push(L(TX, r)));
  return out;
}
/* what you sent while bingo worked: steering lands at the next step, queued lines start the next turn */
function queueRows(now, W) {
  const waits = S.queued.filter(q => !q.steer);
  return S.queued.map(q => {
    const right = q.steer ? [s('at the next step', 'dim')] : [s(`queued${waits.length > 1 ? ' ' + (waits.indexOf(q) + 1) : ''}`, 'dim')];
    const k = prog(now, q.at || 0, 260);
    return L(TX, fitSegs([s(q.text, k < 1 ? mix('glow', 'muted', k) : 'muted')], W - TX - RM - segsW(right) - 4), { r: right });
  });
}

/* ----- the slash palette: commands and skills, filtered by what you type ----- */
function fuzzy(q, t) {
  if (!q) return { score: 0, hits: [] };
  const lo = t.toLowerCase(), i0 = lo.indexOf(q);
  if (i0 === 0) return { score: 300 - t.length, hits: [...Array(q.length).keys()] };
  if (i0 > 0) return { score: 200 - i0, hits: [...Array(q.length).keys()].map(k => k + i0) };
  const hits = []; let j = 0;
  for (let i = 0; i < lo.length && j < q.length; i++) if (lo[i] === q[j]) { hits.push(i); j++; }
  return j === q.length ? { score: 100 - (hits[hits.length - 1] - hits[0]), hits } : null;
}
function paletteOf() {
  const m = S.focus === 'input' && !S.dock && S.input.match(/^\/(\S*)$/);
  if (!m) return null;
  const q = m[1].toLowerCase();
  const here = c => inRoom() ? c.kind !== 'skill' && (!c.room || roomOpen()) : !c.room;
  const items = COMMANDS.filter(here).map(c => ({ c, f: fuzzy(q, c.n) })).filter(x => x.f)
    .sort((a, b) => b.f.score - a.f.score || +!!b.c.room - +!!a.c.room || COMMANDS.indexOf(a.c) - COMMANDS.indexOf(b.c));
  S.pal.sel = clamp(S.pal.sel, 0, Math.max(0, items.length - 1));
  return { q, items };
}
const hitSegs = (t, hits, fg, hiFg = 'glow') => [...t].map((ch, i) => s(ch, hits.includes(i) ? hiFg : fg, hits.includes(i) ? 'b' : ''));
function pickLine(sel, now, W, segs, o = {}) {
  const l = ln(Object.assign({ p: [[TX, segs]] }, o));
  if (sel) {
    const k = o.guard != null ? prog(now, o.guard, GUARD_MS) : 1;
    l.bands = [[TX - 2, W - RM + 1, k < 1 ? mix('you', 'pick', k) : 'pick']];
    l.p.unshift([TX - 2, [s(g('quote'), k < 1 ? mix('you', 'glow', k) : 'glow')]]);
  }
  return l;
}
function listWindow(n, sel, max = LIST_MAX) {
  const top = clamp(sel - Math.floor(max / 2), 0, Math.max(0, n - max));
  return [top, Math.min(n, top + max)];
}
function paletteRows(P, now, W, cap) {
  const max = Math.min(LIST_MAX, Math.max(3, cap)), [a, b] = listWindow(P.items.length, S.pal.sel, max), out = [];
  if (!P.items.length) return [L(TX, [s(`no command matches /${P.q}`, 'dim')]), BLANK()];
  const nameW = Math.min(18, Math.max(...P.items.map(x => sw(x.c.n))) + 3);
  for (let i = a; i < b; i++) {
    const { c, f } = P.items[i], sel = i === S.pal.sel;
    const right = c.kind === 'skill' ? [s(`skill${g('sep')}${c.fam}`, 'dim')] : c.k ? [s(keyText(c.k), 'dim')] : [];
    const segs = [s('/', sel ? 'glow' : 'dim'), ...hitSegs(c.n, f.hits, sel ? 'fg' : 'muted')];
    const l = pickLine(sel, now, W, segs, { r: right, click: { act: 'pal', i } });
    l.p.push([TX + nameW, fitSegs([s(c.d, sel ? 'muted' : 'dim')], W - TX - nameW - RM - segsW(right) - 3)]);
    out.push(l);
  }
  if (P.items.length > max) out.push(L(TX, [s(`${b - a} of ${P.items.length}`, 'dim')]));
  out.push(BLANK());
  return out;
}

/* ----- @: files in the repo, then the agents you can address ----- */
function mentionOf() {
  const m = S.focus === 'input' && !S.dock && S.input.match(/(?:^|\s)@([\w./-]*)$/);
  if (!m) return null;
  const q = m[1].toLowerCase();
  const pool = mentionPool();
  const items = pool.map(x => ({ x, f: fuzzy(q, x.t) })).filter(y => y.f).sort((p, r) => r.f.score - p.f.score).slice(0, 40);
  S.pal.sel = clamp(S.pal.sel, 0, Math.max(0, items.length - 1));
  return { q, items };
}
function mentionRows(M, now, W) {
  const [a, b] = listWindow(M.items.length, S.pal.sel, 6), out = [];
  if (!M.items.length) return [L(TX, [s(`nothing matches @${M.q}`, 'dim')]), BLANK()];
  for (let i = a; i < b; i++) {
    const { x, f } = M.items[i], sel = i === S.pal.sel;
    let segs, right;
    if (x.kind === 'agent') { segs = hitSegs(x.t, f.hits, x.a.c, x.a.c); right = [s(`agent${g('sep')}${x.a.state}`, 'dim')]; }
    else if (x.kind === 'all') { segs = hitSegs(x.t, f.hits, 'plan', 'plan'); right = [s('everyone seated', 'dim')]; }
    else {
      const cut = x.t.lastIndexOf('/') + 1;
      segs = hitSegs(x.t, f.hits, sel ? 'fg' : 'muted').map((y, j) => j < cut && y.fg !== 'glow' ? Object.assign(y, { fg: 'dim' }) : y);
      right = x.f.st ? [s(x.f.st, x.f.st === 'modified' ? 'amber' : 'good')] : [];
    }
    out.push(pickLine(sel, now, W, segs, { r: right, click: { act: 'men', i } }));
  }
  out.push(BLANK());
  return out;
}

/* ----- asks: what bingo wants to do, then a plain list of answers ----- */
function askLines(d, now, W) {
  const out = [], A = d.spec;
  out.push(L(TX, [s(A.title, d.danger ? 'bad' : 'fg', 'b')], { r: A.right }));
  out.push(BLANK());
  out.push(...A.body(W));
  out.push(BLANK());
  A.options.forEach((o, i) => out.push(optionLine(d, o, i, now, W)));
  return out;
}
function optionLine(d, o, i, now, W) {
  const sel = d.cur === i;
  let segs = [s(o.label, sel ? (o.tone || 'fg') : 'muted', sel ? 'b' : '')];
  if (o.scope) segs.push(s(' '), s(o.scope, sel ? 'code' : 'dim'), s(o.tail || '', sel ? (o.tone || 'fg') : 'muted', sel ? 'b' : ''));
  if (o.free && d.typing != null && sel) segs = [s(`${o.label}: `, 'muted'), s(d.typing, 'fg'), s(g('caret'), caretInk(now, 'glow', 'pick'))];
  else if (o.free && sel) segs.push(s(`  ${o.hint}`, 'dim'));
  return pickLine(sel, now, W, segs, { guard: d.at, click: { act: 'opt', i } });
}
/* previews inside an ask: a command, a hunk, an address */
/* an ask shows the whole command, wrapped, never cut */
function cmdBody(cmd, why) {
  const w = W => W - TX - RM - 2;
  return W => [...wrapSegs([s('$ ', 'dim'), s(cmd, 'code')], w(W)).map(r => L(TX + 2, r)), ...(why ? wrapSegs([s(why, 'muted')], w(W)).map(r => L(TX + 2, r)) : [])];
}
function diffBody(diff, max = 8) {
  return W => {
    const shown = diff.slice(0, max), rows = diffWell(shown, W).slice(1, -1);
    for (const r of rows) r.bands = r.bands.map(([a, b, bg]) => [a, b, bg === 'well' ? 'you' : bg]);
    if (diff.length > max) rows.push(L(WX + 2, [s(`${g('ell')} ${diff.length - max} more lines`, 'dim')]));
    return rows;
  };
}

/* ----- a question from bingo: tabs when there are several, choices with a reason each ----- */
function questionLines(d, now, W) {
  const out = [], Q = d.qs[d.tab], multi = d.qs.length > 1;
  if (multi) out.push(tabRow(d, W));
  out.push(L(TX, [s(Q.question, 'fg', 'b')], { r: multi ? null : [s(Q.header, 'dim')] }));
  out.push(BLANK());
  const opts = [...Q.options, { label: 'Something else', free: true }], cur = d.cur[d.tab];
  const prev = opts[cur] && opts[cur].preview, side = prev && W >= 90, labW = Math.max(...opts.map(o => sw(o.label))) + 3;
  const listW = side ? Math.floor((W - TX - RM) * .5) : W - TX - RM;
  opts.forEach((o, i) => {
    const sel = i === cur, on = Q.multi && (d.ticks[d.tab] || []).includes(i);
    const mark = Q.multi && !o.free ? [s(on ? g('ok') : g('dot'), on ? 'good' : 'dim'), s(' ')] : [];
    let segs = [...mark, s(o.label, sel ? 'fg' : 'muted', sel ? 'b' : '')];
    if (o.free && d.typing != null && sel) segs = [s('Something else: ', 'muted'), s(d.typing, 'fg'), s(g('caret'), caretInk(now, 'glow', 'pick'))];
    const l = pickLine(sel, now, W, segs, { guard: d.at, click: { act: 'opt', i } });
    if (o.description) l.p.push([TX + labW + mark.length * 2, fitSegs([s(o.description, sel ? 'muted' : 'dim')], listW - labW - 3)]);
    if (side) l.bands = l.bands.map(([a, , bg]) => [a, TX + listW, bg]);
    out.push(l);
  });
  if (side) previewPane(out, prev, out.length - opts.length, TX + listW + 3, W);
  return out;
}
function tabRow(d, W) {
  const p = []; let x = TX;
  d.qs.forEach((q, i) => {
    const on = i === d.tab, done = d.answers[i] != null;
    const segs = [s(done ? g('ok') + ' ' : '', 'good'), s(q.header, on ? 'fg' : done ? 'muted' : 'dim', on ? 'b' : '')];
    p.push([x, segs]);
    x += segsW(segs) + 4;
  });
  return ln({ p, r: [s(`${d.tab + 1} of ${d.qs.length}`, 'dim')] });
}
/* the preview may run below the last choice; it is the thing you are choosing */
const PREVIEW_MAX = 10;
function previewPane(out, prev, top, x0, W) {
  const lines = prev.split('\n').slice(0, PREVIEW_MAX);
  while (out.length < top + lines.length) out.push(ln());
  lines.forEach((t, j) => {
    const l = out[top + j];
    l.bands.push([x0 - 1, W - RM + 1, 'wellDeep']);
    l.p.push([x0 + 1, fitSegs([s(t, 'code')], W - RM - x0 - 2)]);
  });
}

/* ----- find in the conversation: the dock becomes the search field ----- */
function findLines(d, now, W) {
  const n = d.hits ? d.hits.length : 0;
  const right = !d.q ? [] : n ? [s(`${d.cur + 1} of ${n}`, 'muted')] : [s('no match', 'dim')];
  const caret = s(g('caret'), caretInk(now));
  return [L(TX, [s('find', 'dim'), s('  '), ...(d.q ? [s(d.q, 'fg'), caret] : [caret, s('in this conversation', 'dim')])], { r: right })];
}
/* every match in the whole stream, newest last, counted before the dock that names them is drawn */
function findHits(d, stream, W) {
  const q = [...d.q.toLowerCase()];
  d.hits = [];
  if (q.length) stream.forEach((l, i) => {
    const R = rowOf(l, W), cells = rowText(R);
    for (let j = 0; j + q.length <= cells.length; j++) {
      if (!q.every((ch, k) => cells[j + k].ch.toLowerCase() === ch)) continue;
      const xe = j + q.length < cells.length ? cells[j + q.length].x : R.w;
      d.hits.push({ i, x: cells[j].x, w: xe - cells[j].x });
      j += q.length - 1;
    }
  });
  if (d.cur == null || d.cur >= d.hits.length) d.cur = d.hits.length - 1;
}
/* a jump centres the current match in the rows the stream has */
function centreFind(d, stream, h) {
  if (d.want && d.hits.length) S.scroll = clamp(stream.length - d.hits[d.cur].i - Math.ceil(h / 2), 0, S.maxScroll);
  d.want = false;
}
/* a row's characters with the cell each starts at; the second half of a wide character is skipped */
const rowText = R => R.c.flatMap((c, x) => c.ch === '' ? [] : [{ ch: c.ch || ' ', x }]);

/* ----- sheets: one anatomy for every pane — a head, a list, a detail ----- */
function sheetLines(d, now, W, cap) {
  const sh = SHEETS[d.sheet], out = [];
  const head = sh.head(d, W);
  out.push(L(TX, head.l, { r: head.r }));
  out.push(BLANK());
  if (sh.top) out.push(...sh.top(d, now, W));
  const rows = sh.rows(d, now, W), max = Math.min(sh.max || LIST_MAX, Math.max(3, cap - 4 - (sh.top ? 2 : 0)));
  if (sh.pick === false) d.sel = clamp(d.sel, 0, Math.max(0, rows.length - max));
  const [a, b] = sh.pick === false ? [d.sel, Math.min(rows.length, d.sel + max)] : listWindow(rows.length, d.sel, max);
  for (let i = a; i < b; i++) out.push(sh.pick === false ? rows[i] : pickLine(i === d.sel, now, W, rows[i].segs, Object.assign({ click: { act: 'sheet', i } }, rows[i])));
  if (rows.length > max) out.push(L(TX, [s(`${b - a} of ${rows.length}`, 'dim')]));
  if (sh.foot) { out.push(BLANK()); out.push(...sh.foot(d, now, W)); }
  return out;
}

/* ----- the footer: ambient facts on the left, the keys that work now on the right ----- */
function footerLine(now, W) {
  const right = hintSegs(keyHints()), room = W - TX - RM - segsW(right) - 3;
  return L(TX, fitSegs(toastSegs(now) || ambientSegs(now), room), { r: right });
}
function toastSegs(now) {
  const T = S.toast;
  if (!T) return null;
  const age = now - T.at, k = S.reduced ? 1 : Math.min(prog(now, T.at, 160), 1 - prog(now, T.at + (T.ms || 2400), 300));
  if (age > (T.ms || 2400) + 300) return null;
  return T.segs.map(x => Object.assign({}, x, { fg: mix('bg', x.fg || 'fg', k) }));
}
function ambientSegs(now) {
  const v = viewOf(), m = MODES[S.mode], need = needSegs();
  const model = [s(S.model.id, 'muted'), s(' ' + S.model.eff, 'dim')];
  const parts = v ? [viewSegs(v, now), v.kind === 'agent' ? model : null, need] : [need, model];
  parts.push(S.mode ? [s(m.n, m.c)] : null,
    S.ctx >= .7 ? [s(`context ${Math.round(S.ctx * 100)}%`, S.ctx >= .8 ? 'amber' : 'muted')] : null,
    !v && !need ? crewSegs(now) : null,
    S.jobs.length ? [s(plural(S.jobs.length, 'shell'), 'muted')] : null,
    S.wake ? [s(`wakes in ${fmtDur(S.wake.at - now)}`, 'muted')] : null);
  return parts.filter(p => p && p.length).flatMap((p, i) => [...(i ? [s(g('sep'), 'faint')] : []), ...p]);
}
/* someone else is waiting on you: the one fact worth the first place in the footer */
function needSegs() {
  const ns = needers();
  if (!ns.length) return null;
  const name = id => id === 'main' ? s('bingo', 'glow') : s(id, crewInk(id));
  const who = ns.length > 2 ? [s(`${ns.length}`, 'amber')] : ns.flatMap((id, i) => [...(i ? [s(', ', 'amber')] : []), name(id)]);
  return [s(`${g('pin')} `, 'amberGlow'), ...who, s(ns.length > 1 ? ' need you' : ' needs you', 'amber')];
}
/* whose session this is: a teammate's name, or a room and who sits in it (the writer breathes) */
function viewSegs(v, now) {
  if (v.kind === 'agent') return [s(v.name, v.c, 'b')];
  if (v.closed) return [s(v.name, v.c, 'b'), s(g('sep'), 'faint'), s('closed', 'dim')];
  return [s(v.name, v.c, 'b'), s(' '), ...seated(v).flatMap(n => [s(' '), s(n, v.typing && v.typing.name === n ? mix(crewInk(n), 'bg', .45 * breathe(now, 1100)) : crewInk(n))])];
}
/* agents at work while you are home: one breathing lamp each */
function crewSegs(now) {
  const busy = crewList().filter(a => a.state === 'working');
  if (!busy.length) return null;
  return [...busy.slice(0, 4).map((a, i) => s(g('lamp'), mix(a.c, 'bg', .35 * breathe(now + i * 450, 1800)))), s(` ${plural(busy.length, 'agent')}`, 'muted')];
}
function hintSegs(pairs) {
  const out = [];
  pairs.forEach(([k, a], i) => { if (i) out.push(s('   ')); out.push(s(keyText(k), 'muted'), s(' ' + a, 'dim')); });
  return out;
}
function keyHints() {
  const d = S.dock, t = liveTurn(), stop = viewOf() ? 'back' : 'stop';
  if (d && d.kind === 'ask') return d.typing != null ? [['⏎', 'send'], ['esc', 'back']] : [['↑↓', 'choose'], ['⏎', 'confirm'], ['esc', stop]];
  if (d && d.kind === 'question') return d.typing != null ? [['⏎', 'send'], ['esc', 'back']]
    : [...(d.qs.length > 1 ? [['←→', 'questions']] : []), ...(d.qs[d.tab].multi ? [['space', 'tick']] : []), ['⏎', 'choose'], ['esc', stop]];
  if (d && d.kind === 'find') return [['↑↓', 'matches'], ['⏎', 'jump'], ['esc', 'close']];
  if (d && d.kind === 'sheet') return SHEETS[d.sheet].keys(d);
  if (S.focus === 'ledger') return [['↑↓', 'step'], ['⏎', 'open'], ['esc', 'close']];
  if (S.focus === 'welcome') return [['↑↓', 'recent'], ['⏎', 'resume'], ['esc', 'back']];
  if (paletteOf()) return [['↑↓', 'choose'], ['⏎', 'run'], ['tab', 'complete']];
  if (mentionOf()) return [['↑↓', 'choose'], ['tab', 'insert']];
  if (S.scroll > 0) return [['end', 'latest'], ['ctrl+f', 'find']];
  if (viewOf()) return viewHints(t);
  const ks = homeHints(t);
  return needers().length && !S.input ? [['ctrl+g', 'answer'], ...ks.slice(-2)] : ks;
}
/* at home: what the turn and the draft allow */
function homeHints(t) {
  if (t && S.input) return [['⏎', 'steer'], ['tab', 'queue'], ['esc', 'stop']];
  if (t) {
    const bash = steps(t).some(x => x.st === 'run' && x.tool === 'bash');
    return [...(S.queued.length ? [['↑', 'edit queued']] : []), ...(bash ? [['ctrl+b', 'background']] : []), ['esc', 'stop']];
  }
  if (S.input) return [['⏎', 'send'], ['⇧⏎', 'newline']];
  if (S.welcome && !S.welcome.out) return [['↑', 'recent'], ['/', 'commands'], ['?', 'keys']];
  if (S.log.length) return [['ctrl+o', 'details'], ['/', 'commands'], ['?', 'keys']];
  return [['/', 'commands'], ['@', 'files'], ['?', 'keys']];
}
/* in a teammate's session or a room: esc is the way home, ctrl+c the way to stop */
function viewHints(t) {
  if (S.input || S.chips.length) return t ? [['⏎', 'steer'], ['tab', 'queue'], ['esc', 'clear']] : [['⏎', inRoom() ? 'post' : 'send'], ['esc', 'clear']];
  return [['esc', 'back'], ...(needers().length ? [['ctrl+g', 'answer']] : []), ...(t ? [['ctrl+c', 'stop']] : []), roomOpen() ? ['@', 'call on'] : ['/', 'commands']].slice(0, 3);
}

/* ---------- the frame: stream above, dock below, footer last ---------- */
function frame(now) {
  if (S.exited) return exitFrame(now);
  const W = S.W, H = S.H;
  const stream = S.log.length ? [...streamLines(now), BLANK()] : [];
  if (S.dock && S.dock.kind === 'find') findHits(S.dock, stream, W);
  const dock = dockLines(now, W, H), foot = footerLine(now, W);
  const h = Math.max(1, H - dock.length - 1), view = viewport(now, stream, W, h);
  const lines = [...view, ...dock, foot], rows = lines.map(l => rowOf(l, W));
  if (S.dock && S.dock.kind === 'find') markFinds(rows, h);
  lines.forEach((l, y) => { if (l.fx) effects(rows, l.fx, y, now); });
  scrollbar(rows, h, now, W);
  return { lines, rows };
}
function viewport(now, stream, W, h) {
  S.streamN = stream.length; S.viewH = h;
  S.maxScroll = Math.max(0, stream.length - h);
  if (S.dock && S.dock.kind === 'find') centreFind(S.dock, stream, h);
  if (S.focus === 'ledger') keepSelVisible(stream, h);
  S.scroll = clamp(S.scroll, 0, S.maxScroll);
  const end = stream.length - S.scroll, top = Math.max(0, end - h), v = stream.slice(top, end);
  S.vmap = { pad: h - v.length, top };
  const welcome = S.welcome ? welcomeLines(now, W, h) : [];
  const out = Array.from({ length: h - v.length }, (_, i) => welcome[i] || BLANK()).concat(v);
  if (S.scroll > 0) out[h - 1] = pillLine(W, S.scroll);
  return out;
}
function pillLine(W, n) {
  const label = ` ${g('down')} ${plural(n, 'line')} below `, x = Math.floor((W - sw(label)) / 2);
  return ln({ p: [[x, [s(label, 'fg', { bg: 'you' })]]], click: { act: 'end' } });
}
function scrollbar(rows, h, now, W) {
  const N = S.streamN || 0, shown = S.scroll > 0 || now - (S.scrollAt || -1e9) < 900;
  if (!shown || N <= h) return;
  const size = Math.max(1, Math.round(h * h / N)), top = Math.round((N - h - S.scroll) / Math.max(1, N - h) * (h - size));
  const ink = now - (S.scrollAt || -1e9) < 900 ? 'dim' : 'faint';
  for (let y = top; y < top + size && y < h; y++) { const c = rows[y].c[W - 1]; c.ch = '▕'; c.fg = ink; }
}
/* the find bar paints every match it can see; the current one burns ember */
function markFinds(rows, h) {
  const d = S.dock, V = S.vmap;
  if (!d.q || !d.hits || !V) return;
  d.hits.forEach((hit, n) => {
    const y = hit.i - V.top + V.pad;
    if (y < V.pad || y >= h || (S.scroll > 0 && y === h - 1)) return;
    const R = rows[y], cur = n === d.cur;
    for (let x = hit.x; x < Math.min(R.w, hit.x + hit.w); x++) { R.c[x].bg = cur ? 'glow' : 'glowTint'; if (cur) R.c[x].fg = 'bg'; }
  });
}
/* the row detail has selected stays on screen as you move through it */
function keepSelVisible(stream, h) {
  const sel = S.sel;
  if (!sel || !S.wantSel) return;
  const i = stream.findIndex(l => l.click && l.click.act === 'row' && l.click.t === sel.t && l.click.i === sel.i);
  S.wantSel = false;
  if (i < 0) return;
  const end = stream.length - S.scroll, top = end - h;
  if (i < top + 1) S.scroll = clamp(stream.length - i - h + 1, 0, S.maxScroll);
  else if (i >= end - 2) S.scroll = clamp(stream.length - i - 3, 0, S.maxScroll);
}

/* ---------- effects: sparks off the edge light, embers off the cursor; they only ever land on empty cells ---------- */
function effects(rows, fx, y, now) {
  if (fx.sparks != null) sparks(rows, y, fx.x, fx.sparks, now);
  if (fx.embers != null) embers(rows, y, fx.x, fx.embers, now);
  if (fx.dissolve != null) dissolve(rows, y, fx, now);
}
function dot(rows, px, py, ink) {
  const cy = Math.floor(py), cx = Math.floor(px);
  if (cy < 0 || cy >= rows.length || cx < 0) return;
  const cell = rows[cy].c[cx];
  if (!cell || cell.ch !== ' ') return;
  cell.ch = dotAt(Math.floor((px - cx) * 2), Math.floor((py - cy) * 4));
  cell.fg = ink(cell.bg || 'bg');
}
function sparks(rows, y, x, at, now) {
  if (S.reduced || S.ascii) return;
  const k = (now - at) / 700;
  if (k < 0 || k >= 1) return;
  const e = easeOut(k);
  [[.6, 1.15], [1.5, 1.4], [2.4, .95]].forEach(([dx, dy]) =>
    dot(rows, x + .5 + dx * 2.6 * e, y + .45 - dy * 2.2 * e, bg => mix('glow', bg, Math.pow(k, 1.3))));
}
function embers(rows, y, x, at, now) {
  if (S.reduced || S.ascii) return;
  const N = 7, per = 2800, age = now - at;
  for (let j = 0; j < N; j++) {
    if (age < j * per / N) continue;
    const k = ((age / per) + j / N) % 1, wob = Math.sin((k * 2.3 + j * 1.9) * Math.PI) * .55;
    dot(rows, x + .35 + wob + ((j * 5) % 3 - 1) * .3, y + .15 - k * 4.4, bg => mix('glow', bg, Math.pow(k, 1.25)));
  }
}
/* the welcome lets go of its letters: each cell becomes a dot that drifts up and cools */
function dissolve(rows, y, fx, now) {
  const k = (now - fx.dissolve) / 520, R = rows[y];
  if (k <= 0) return;
  for (let x = 0; x < R.w; x++) {
    const c = R.c[x];
    if (c.ch === ' ' || c.ch === '') continue;
    const r = ((x * 73 + y * 151) % 97) / 97;
    if (k * 1.4 > r) { c.ch = k < .55 + r * .3 ? dotAt(x & 1, 3 - Math.min(3, Math.floor(k * 5))) : ' '; c.fg = mix(c.fg || 'fg', 'bg', clamp(k * 1.6, 0, 1)); }
  }
}

/* ---------- after quitting: the shell comes back, bingo leaves one receipt and fish suggests the way back ---------- */
function exitFrame(now) {
  const W = S.W, H = S.H, turns = S.log.filter(e => e.kind === 'turn'), k = prog(now, S.exited.at, 360);
  const fade = segs => segs.map(x => Object.assign({}, x, { fg: k < 1 ? mix('bg', x.fg || 'fg', k) : x.fg }));
  const prompt = [s(HERE.dir, 'muted'), s(' on ', 'dim'), s(HERE.branch, 'plan'), s(' ❯ ', 'good')];
  const lines = [L(0, [...prompt, s('bingo', 'fg')]), BLANK()];
  if (turns.length) {
    const files = new Set(turns.flatMap(t => steps(t).filter(x => edits(x) && x.st === 'ok').map(x => x.arg)));
    const took = S.exited.at - turns[0].t0;
    lines.push(L(0, fade([s(g('lamp'), 'good'), s(' bingo', 'fg', 'b'), s(g('sep'), 'faint'), s(S.title || 'this session', 'muted')])));
    lines.push(L(2, fade([s([plural(turns.length, 'turn'), fmtDur(took), ...(files.size ? [`edited ${plural(files.size, 'file')}`] : [])].join(g('sep')), 'dim')])));
    lines.push(L(2, fade([s('resume with ', 'dim'), s(RESUME, 'code')])), BLANK());
  }
  const caret = s(g('caret'), caretInk(now, 'fg', 'bg'));
  lines.push(L(0, [...prompt, caret, s(turns.length ? RESUME : 'bingo', 'faint')]));
  while (lines.length < H) lines.push(BLANK());
  return { lines, rows: lines.map(l => rowOf(l, W)) };
}
