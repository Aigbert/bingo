/* ---------- stream: your words on a band, each step on a row of its own, then the answer ---------- */
const TX = 3, RM = 2, MEASURE = 100, VW = 8, AX = TX + 2 + VW;
const textW = W => Math.min(W - TX - RM, MEASURE);
const baseName = p => p.split('/').pop();
const cmdName = c => c.split(/\s+/).filter(w => !w.startsWith('-')).slice(0, 2).join(' ');

function streamLines(now) {
  const W = S.W, out = [];
  const last = S.log.filter(e => e.kind === 'turn').pop(), newest = last && !isLive(last) ? last : null;
  S.log.forEach((e, i) => {
    const join = sameVoice(S.log[i - 1], e);
    if (i > 0 && !join) out.push(BLANK());
    if (e.kind === 'turn') out.push(...turnLines(e, now, W, e === newest));
    else if (e.kind === 'notice') out.push(...noticeLines(e, now, W));
    else if (e.kind === 'post') out.push(...postLines(e, i, now, W, join));
  });
  out.push(...typingLines(now, W));
  return out;
}

function turnLines(t, now, W, newest) {
  const out = bandLines(t, now, W, newest);
  if (t.shell) return out.concat(shellLines(t, now, W));
  out.push(...stepLines(t, now, W));
  if (isLive(t)) out.push(...liveLines(t, now, W));
  const ans = answerLines(t, now, W);
  if (ans.length) out.push(BLANK(), ...ans);
  if (t.error) out.push(...errorLines(t, now, W));
  return out;
}

/* ----- the band: your words; its left edge carries the turn, its first row the turn's clock ----- */
/* an edge light breathes between its colour and the coal under it */
const coalOf = ink => ink === 'glow' ? 'coal' : mix(ink, '#000000', .28);
const breatheInk = (ink, now) => mix(ink, coalOf(ink), breathe(now, 1800));
function sideInk(t, now, newest) {
  switch (t.state) {
    case 'live': return breatheInk(viewInk(), now);
    case 'asking': return 'amberGlow';
    case 'done': {
      if (!newest) return null;
      const k = prog(now, t.endAt, 320);
      return k < 1 ? mix(viewInk(), 'good', easeOut(k)) : 'good';
    }
    case 'failed': return newest ? 'bad' : null;
    default: return newest ? 'dim' : null;
  }
}
const chipSegs = chips => chips.flatMap(c => [s(`[${c}]`, 'plan'), s(' ')]);
const promptSegs = t => t.shell ? [s('!', 'glow', 'b'), s(' '), s(t.prompt.replace(/^!\s*/, ''), 'code')]
  : t.wake ? [s('wake-up', 'glow'), s('  '), ...inl(t.prompt, 'muted', 'code')]
  : t.from ? [s(t.from, 'glow'), s('  '), ...inl(t.prompt, 'fg', 'code')]
  : [...chipSegs(t.chips), ...inl(t.prompt, 'fg', 'code')];
/* time spent waiting for you is not bingo's time; a stopped turn says so where its clock is */
function clockSegs(t, now) {
  if (t.shell) return [];
  const dur = elapsedOf(t, now), time = dur >= 1000 ? [s(fmtDur(dur), isLive(t) ? 'muted' : 'dim')] : [];
  if (t.state !== 'stopped') return time;
  return [s('stopped', 'muted'), ...(time.length ? [s(g('sep'), 'faint'), ...time] : [])];
}
/* the clock keeps one fixed room while it ticks, so your words never rewrap under it */
function bandRows(t, clock, w) {
  const room = isLive(t) || clock.length ? Math.max(8, segsW(clock)) + 2 : 0, rows = wrapSegs(promptSegs(t), w);
  return room && segsW(rows[0]) > w - room ? wrapSegs(promptSegs(t), w - room) : rows;
}
function bandLines(t, now, W, newest) {
  const clock = clockSegs(t, now);
  let rows = bandRows(t, clock, W - TX - RM), more = 0;
  if (rows.length > 4) { more = rows.length - 3; rows = rows.slice(0, 3); }
  const out = rows.map((r, i) => L(TX, r, { fill: 'you', r: !i && clock.length ? clock : null }));
  if (more) out.push(L(TX, [s(`${g('ell')} ${more} more lines`, 'dim')], { fill: 'you' }));
  if (!t.shell) out.push(...saidLines(t, now, W));
  const ink = sideInk(t, now, newest);
  if (ink) for (const l of out) l.p.unshift([0, [s(g('side'), ink)]]);
  if (t.state === 'done' && newest && !S.reduced && now - t.endAt < 700) out[0].fx = { sparks: t.endAt, x: 0 };
  return out;
}

/* ----- the steps: one row each, where they happened, bingo's words between them ----- */
/* in detail (ctrl+o) the thoughts and what you added go back to where they happened */
function stepLines(t, now, W) {
  const out = [], detail = t.open && !isLive(t);
  t.items.forEach((it, i) => {
    const band = S.sel && S.sel.t === t && S.sel.i === i ? { fill: 'pickBg' } : {};
    if (it.k === 'step') out.push(...stepRows(t, it, i, now, W, band));
    else if (it.k === 'narr') out.push(...narrLines(t, it, i, now, W));
    else if (detail && it.k === 'think') out.push(...thoughtRows(t, it, i, W, band));
    else if (detail && it.k === 'you') out.push(youLine(it, W, !!band.fill, now));
  });
  return out;
}
function stepRows(t, x, i, now, W, band) {
  const out = [stepRow(t, x, i, now, W, band)];
  if (x.st === 'run' && x.tail.length) out.push(tailLine(x, W));
  if (t.well === i) out.push(...wellFor(x, W));
  else if (failsTurn(t, x)) out.push(...failTail(x, W));
  return out;
}
/* mark at 3, verb at 5, target at 13, outcome right: a running row and a finished one are the same row */
function stepRow(t, x, i, now, W, band) {
  const right = stepRight(x, now), ax = argX(x), w = W - ax - RM - (right.length ? segsW(right) + 2 : 0);
  const settled = x.st !== 'run' && x.st !== 'ask';
  return ln(Object.assign({ p: [[TX, [markSeg(x, now)]], [TX + 2, [verbSeg(x)]], [ax, argSegs(x, w)]], r: right.length ? right : null,
    click: settled ? { act: 'row', t, i } : null }, band));
}
const argX = x => Math.max(AX, TX + 2 + sw(verbOf(x)) + 2);
const quiet = x => x.st === 'skip' || x.st === 'cut';
/* the mark: a spinner while it runs, the tool's own mark once it lands, warm for a moment */
function markSeg(x, now) {
  if (x.st === 'run') return s(spinG(now, x.at), x.agent ? x.agent.c : viewInk());
  if (x.st === 'ask') return s(g('pin'), mix('amberGlow', 'you', .65 * breathe(now, 1100)));
  return s(markOf(x), landInk(x, markInk(x), now));
}
const markInk = x => x.st === 'fail' ? 'bad' : quiet(x) ? 'faint' : x.agent ? x.agent.c : looks(x) ? 'muted' : 'fg';
const landInk = (x, ink, now) => { const k = prog(now, x.lit ?? x.end ?? -1e9, 420); return k < 1 ? mix(viewInk(), ink, easeOut(k)) : ink; };
/* a lookup reads a shade quieter than a step that changes something */
const verbSeg = x => s(verbOf(x), x.st === 'fail' ? 'bad' : x.agent ? x.agent.c : quiet(x) || looks(x) ? 'dim' : 'muted');
/* the target: a path keeps its file name and gives up leading folders first; a command reads as code */
function argSegs(x, w) {
  const ink = quiet(x) ? 'dim' : looks(x) ? 'muted' : 'fg', how = toolOf(x).arg;
  if (x.agent && x.st === 'run') return fitSegs([s(x.agent.doing || x.arg, ink)], w);
  if (how === 'path') return pathSegs(x.arg, w, ink);
  return fitSegs([s(x.arg, how === 'cmd' && !quiet(x) ? 'code' : ink)], w);
}
function pathSegs(p, w, ink) {
  const cut = p.lastIndexOf('/') + 1, base = p.slice(cut);
  let dirs = p.slice(0, cut).split('/').filter(Boolean), lead = p.startsWith('/') ? '/' : '';
  const dirText = () => dirs.length ? `${lead}${dirs.join('/')}/` : lead;
  while (dirs.length && sw(dirText()) + sw(base) > w) { dirs = dirs.slice(1); lead = `${g('ell')}/`; }
  const head = dirText();
  return fitSegs([...(head ? [s(head, 'dim')] : []), s(base, ink)], w);
}
/* the right side: how long it has run, that it needs you, or what came of it */
function stepRight(x, now) {
  if (x.st === 'ask') return [s('needs you', 'amber')];
  if (x.st !== 'run') return [...resultSegs(x), ...whySegs(x)];
  const time = now - x.at >= 1000 ? [s(fmtDur(now - x.at), 'dim')] : [];
  return x.prog ? [s(x.prog(now), 'muted'), ...(time.length ? [s(g('sep'), 'faint'), ...time] : [])] : time;
}
function resultSegs(x) {
  if (quiet(x)) return [s(x.res || 'skipped', 'muted')];
  if (x.st === 'fail') return [s(x.res, 'bad')];
  if (edits(x)) return [...(x.add ? [s(`+${x.add}`, 'good')] : []), ...(x.add && x.del ? [s(' ')] : []), ...(x.del ? [s(`${g('minus')}${x.del}`, 'bad')] : [])];
  return x.res ? [s(x.res, looks(x) ? 'dim' : 'muted')] : [];
}
/* the tail is a peek at what the step is doing, so it keeps the target's column, not the tool's indent */
const tailLine = (x, W) => L(argX(x), fitSegs([s(x.tail[x.tail.length - 1].trimStart(), 'dim')], W - argX(x) - RM));
const failsTurn = (t, x) => x.st === 'fail' && !isLive(t) && steps(t).pop() === x;

/* bingo's own words between the steps; the newest types itself out until something follows it */
function narrLines(t, n, i, now, W) {
  const followed = !isLive(t) || t.reply || t.items.slice(i + 1).some(it => it.k === 'step' || it.k === 'narr');
  const text = followed ? n.text : n.text.slice(0, Math.floor(Math.max(0, now - n.at) / 1000 * RATE_N));
  return text ? wrapSegs([s(text, 'muted', 'i')], textW(W)).map(r => L(TX, r)) : [];
}
/* a thought, in detail only: its first words where it happened, all of it in its well */
function thoughtRows(t, it, i, W, band) {
  const right = it.ms >= 1000 ? [s(fmtDur(it.ms), 'dim')] : [], w = W - AX - RM - (right.length ? segsW(right) + 2 : 0);
  const row = ln(Object.assign({ p: [[TX, [s(g('think'), 'dim')]], [TX + 2, [s('think', 'dim')]], [AX, fitSegs([s(it.text, 'dim', 'i')], w)]],
    r: right.length ? right : null, click: { act: 'row', t, i } }, band));
  return t.well === i ? [row, ...thoughtWell(it, W)] : [row];
}

/* ----- while it works: thinking, or that it is still at it, and a retry ----- */
function liveLines(t, now, W) {
  const out = [], busy = steps(t).some(x => x.st === 'run' || x.st === 'ask');
  const th = t.items.filter(it => it.k === 'think' && now < it.at + it.ms).pop();
  if (th && !busy) out.push(L(TX, [s(spinG(now, th.at), viewInk()), s(' '), ...shimmer('thinking', now, 'muted')], { r: now - th.at >= 1000 ? [s(fmtDur(now - th.at), 'dim')] : null }));
  else if (!th && !busy && stalled(t, now)) out.push(L(TX, [s(spinG(now, t.t0), viewInk()), s(' '), ...shimmer('working', now, 'muted')]));
  if (t.retry) out.push(retryRow(t, now, W));
  return out;
}
/* only a pause longer than a beat says so, or the word would flicker between steps */
function stalled(t, now) {
  if (t.state !== 'live' || t.reply || t.retry) return false;
  const last = t.items.reduce((m, it) => Math.max(m, it.end ?? it.at, it.k === 'think' ? it.at + it.ms : 0,
    it.k === 'narr' ? it.at + revealMs(it.text.length, RATE_N) : 0), t.t0 + t.pausedMs);
  return now - last > 500;
}
function retryRow(t, now, W) {
  const r = t.retry, left = Math.max(0, Math.ceil((r.until - now) / 1000));
  return L(TX, [s(spinG(now, r.at), 'amber'), s(' '), s(r.why, 'amber'), s(g('sep'), 'dim'), s(`retrying in ${left}s`, 'muted')],
    { r: [s(`attempt ${r.n} of ${r.of}`, 'dim')] });
}
/* what you added while it worked stays with your words; in detail it goes back to where it was heard */
const saidLines = (t, now, W) => t.open && !isLive(t) ? [] : t.items.filter(it => it.k === 'you').map(y => youLine(y, W, false, now));
function youLine(y, W, sel, now) {
  const age = now - y.at, right = [s(y.note, age < 420 && !S.reduced ? mix('glow', 'dim', age / 420) : 'dim')];
  return L(TX, fitSegs(inl(y.text, 'fg', 'code'), W - TX - RM - segsW(right) - 2), { r: right, fill: sel ? 'pickBg' : 'you' });
}
/* why a step ran without asking, or that you changed it first */
const whySegs = x => [...(x.changed ? [s(g('sep'), 'faint'), s('changed by you', 'dim')] : []), ...(x.auto ? [s(g('sep'), 'faint'), s(x.auto, 'dim')] : [])];

/* ----- wells: output and diffs, inset on a deeper ground ----- */
const WX = TX + 2;
function wellRows(rows, W, o = {}) {
  const x0 = o.x0 ?? WX, x1 = W - RM, out = [];
  out.push(ln({ bands: [[x0, x1, 'well']] }));
  for (const r of rows) {
    const l = ln({ bands: [[x0, x1, r.bg || 'well']], p: [[x0 + 2, fitSegs(r.segs, x1 - x0 - 4)]] });
    if (r.r) { l.r = r.r; l.rm = RM + 2; }
    out.push(l);
  }
  out.push(ln({ bands: [[x0, x1, 'well']] }));
  return out;
}
const WELL_ROWS = 12;
function wellFor(x, W) {
  if (x.diff) return diffWell(x.diff, W, x.arg);
  if (x.code) return codeWell(x.code, W);
  if (x.qa) return wellRows(x.qa.flatMap(([q, a], i) => [...(i ? [{ segs: [] }] : []), { segs: [s(q, 'muted')] }, { segs: [s(a, 'fg')] }]), W);
  const lines = x.out || x.tail || [];
  if (!lines.length) return wellRows([{ segs: [s(x.st === 'skip' ? 'did not run' : 'no output', 'dim')] }], W);
  return outWell(lines, looks(x), W);
}
/* a command's verdict is at its end, a lookup's answer at its start */
function outWell(lines, head, W) {
  const shown = head ? lines.slice(0, WELL_ROWS) : lines.slice(-WELL_ROWS), more = lines.length - shown.length;
  const rows = shown.map(t => ({ segs: [s(t, /error|failed|panicked/i.test(t) ? 'bad' : 'muted')] }));
  if (!more) return wellRows(rows, W);
  const note = { segs: [s(`${g('ell')} ${more} ${head ? 'more' : 'earlier'} lines`, 'dim')] };
  return wellRows(head ? [...rows, note] : [note, ...rows], W);
}
/* code a step read, numbered from where the read began so it lines up with a diff */
function codeWell(c, W) {
  const shown = c.lines.slice(0, WELL_ROWS), more = c.of - c.start + 1 - shown.length;
  const rows = shown.map((t, i) => ({ segs: [s(String(c.start + i).padStart(4), 'dim'), s('   '), s(t, 'muted')] }));
  if (more > 0) rows.push({ segs: [s(`${g('ell')} ${more} more lines`, 'dim')] });
  return wellRows(rows, W);
}
function thoughtWell(it, W) {
  const rows = wrapSegs([s(it.text, 'muted', 'i')], W - WX - RM - 4).map(r => ({ segs: r }));
  return wellRows(rows, W);
}
/* word emphasis between a removed line and the line that replaced it */
function wordSpans(a, b) {
  let p = 0; while (p < a.length && p < b.length && a[p] === b[p]) p++;
  let q = 0; while (q < a.length - p && q < b.length - p && a[a.length - 1 - q] === b[b.length - 1 - q]) q++;
  return [[p, a.length - q], [p, b.length - q]];
}
function diffSegs(text, span, fg, hi) {
  if (!span || span[1] <= span[0]) return [s(text, fg)];
  return [s(text.slice(0, span[0]), fg), s(text.slice(span[0], span[1]), fg, { bg: hi }), s(text.slice(span[1]), fg)];
}
function diffWell(diff, W, file) {
  const rows = [];
  for (let i = 0; i < diff.length; i++) {
    const d = diff[i], nx = diff[i + 1], pv = diff[i - 1];
    let span = null;
    if (d.s === '-' && nx && nx.s === '+') span = wordSpans(d.t, nx.t)[0];
    if (d.s === '+' && pv && pv.s === '-') span = wordSpans(pv.t, d.t)[1];
    const fg = d.s === '+' ? 'fg' : d.s === '-' ? 'fg' : 'muted';
    const bg = d.s === '+' ? 'addTint' : d.s === '-' ? 'delTint' : null;
    const sign = d.s === '+' ? s('+', 'good') : d.s === '-' ? s(g('minus'), 'bad') : s(' ');
    rows.push({ bg, segs: [s(String(d.n).padStart(4), 'dim'), s(' '), sign, s(' '), ...diffSegs(d.t, span, fg, d.s === '+' ? 'addWord' : 'delWord')] });
  }
  return wellRows(rows, W);
}
/* a shell turn is its command and its output, nothing else */
function shellLines(t, now, W) {
  const i = t.items.findIndex(it => it.k === 'step'), x = t.items[i];
  if (!x) return [];
  if (isLive(t)) return stepRows(t, x, i, now, W, {});
  const lines = x.out || x.tail, ink = x.st === 'fail' ? 'bad' : 'muted';
  const rows = lines.map(l => ({ segs: [s(l, ink)] }));
  if (x.st === 'fail') rows.push({ segs: [s(x.res, 'bad')] });
  return wellRows(rows, W);
}
/* a failure the turn never recovered from shows its last lines without asking */
function failTail(x, W) {
  const lines = (x.out || x.tail).slice(-4);
  return lines.length ? wellRows(lines.map(l => ({ segs: [s(l, /error|failed|panicked/i.test(l) ? 'bad' : 'muted')] })), W) : [];
}

/* ----- errors: what went wrong, then what to do ----- */
function errorLines(t, now, W) {
  const e = t.error, out = [];
  out.push(L(TX, fitSegs([s(e.head, 'bad'), s(g('sep'), 'dim'), s(e.msg, 'fg')], W - TX - RM)));
  if (e.hint) out.push(L(TX, e.hint()));
  return out;
}

/* ----- notices: session events drawn as a quiet divider ----- */
function noticeLines(e, now, W) {
  const label = [s(' ', 'muted'), ...(typeof e.segs === 'function' ? e.segs() : e.segs), s(' ', 'muted')];
  const k = prog(now, e.at || 0, 400);
  const l = L(TX, [s(g('thin').repeat(2), 'faint'), ...label.map(x => Object.assign({}, x, { fg: k < 1 ? mix('glow', x.fg, k) : x.fg }))]);
  const x = TX + 2 + segsW(label);
  l.p.push([x, [s(g('thin').repeat(Math.max(0, W - RM - x)), 'faint')]]);
  return [l];
}

/* ----- posts: a teammate's words, or yours in a room; a name column, then the words ----- */
const GUT = 11;
const sameVoice = (a, b) => !!a && a.kind === 'post' && b.kind === 'post' && a.who === b.who;
const postLines = (e, i, now, W, join) => e.who === 'you' ? yourPostLines(e, i, now, W, join) : theirPostLines(e, now, W, join);
function theirPostLines(e, now, W, join) {
  const ink = crewInk(e.who), k = prog(now, e.at || 0, 260), click = rosterOf(e.who) ? { act: 'view', id: e.who } : null;
  const rows = wrapSegs(inl(e.text, 'fg', 'code'), Math.min(W - TX - GUT - RM, MEASURE));
  return rows.map((r, j) => ln({
    p: [[0, [s(g('quote'), ink)]], ...(j || join ? [] : [[TX, [s(e.who, ink, 'b')]]]), [TX + GUT, k < 1 ? r.map(x => Object.assign({}, x, { fg: mix('bg', x.fg, k) })) : r]],
    click,
  }));
}
/* what you said in a room; until everyone you called on has answered, its edge waits in their colour */
const mentionSegs = text => text.split(/(@[\w-]+)/).filter(Boolean)
  .flatMap(p => /^@[\w-]+$/.test(p) ? [s(p, p === '@all' ? 'plan' : crewInk(p.slice(1)))] : inl(p, 'fg', 'code'));
function owedOn(e, i) {
  const v = viewOf(), spoke = S.log.slice(i + 1).filter(x => x.kind === 'post').map(x => x.who);
  return (e.calls || []).filter(n => canSpeak(v, n) && !spoke.includes(n));
}
function yourPostLines(e, i, now, W, join) {
  const owed = owedOn(e, i), right = owed.length ? [s(`waiting on ${owed.join(', ')}`, 'dim')] : null;
  const rows = wrapSegs(mentionSegs(e.text), Math.min(W - TX - GUT - RM - (right ? segsW(right) + 2 : 0), MEASURE));
  const out = rows.map((r, j) => ln({ p: [...(j || join ? [] : [[TX, [s('you', 'dim')]]]), [TX + GUT, r]], r: j ? null : right, fill: 'you' }));
  if (owed.length) { const ink = breatheInk(crewInk(owed[0]), now); for (const l of out) l.p.unshift([0, [s(g('side'), ink)]]); }
  return out;
}
/* someone you called on is writing */
function typingLines(now, W) {
  const v = viewOf(), ty = v && v.typing;
  if (!ty) return [];
  const ink = crewInk(ty.name);
  return [BLANK(), ln({ p: [[0, [s(g('quote'), mix(ink, 'bg', .45))]], [TX, [s(ty.name, ink, 'b')]], [TX + GUT, [s(spinG(now, ty.at), ink), s(' writing', 'dim')]]] })];
}

/* ----- the answer: markdown on a readable measure, ink dries behind the caret ----- */
function answerLines(t, now, W) {
  if (!t.reply) return [];
  const live = isLive(t) && !t.paused;
  const n = live ? Math.floor(Math.max(0, now - t.reply.at) / 1000 * RATE_R) : (t.reply.cut ?? Infinity);
  const out = mdLines(t.reply.blocks, W, n, live ? now : null);
  if (t.state === 'stopped' && t.reply.cut != null && out.length) {
    const last = out[out.length - 1], end = lineEnd(last);
    if (end + 10 < W - RM) last.p.push([end + 1, [s(`${g('minus')} stopped`, 'dim')]]);
    else out.push(L(TX, [s(`${g('minus')} stopped`, 'dim')]));
  }
  return out;
}
const lineEnd = l => l.p.reduce((m, [x, segs]) => Math.max(m, x + segsW(segs)), 0);
function recolorTail(segs, k, now, ink = 'glow') {
  if (S.reduced || now == null) return segs;
  const out = []; let left = k;
  for (let i = segs.length - 1; i >= 0; i--) {
    const x = segs[i], ch = [...x.t];
    if (left <= 0) { out.unshift(x); continue; }
    const keep = Math.max(0, ch.length - left);
    const tail = ch.slice(keep).map((c, j) => { const d = (ch.length - keep - 1 - j) + (k - left); return s(c, mix(ink, x.fg || 'fg', d / k), { b: x.b }); });
    out.unshift(...(keep ? [Object.assign({}, x, { t: ch.slice(0, keep).join('') })] : []), ...tail);
    left -= ch.length - keep;
  }
  return out;
}
function mdLines(blocks, W, budget, now) {
  const out = [];
  let left = budget, partial = false;
  for (let bi = 0; bi < blocks.length && left > 0; bi++) {
    const b = blocks[bi], n = blockChars([b]), cut = left < n ? left : null;
    left -= n;
    if (bi > 0 && !blocks[bi - 1].h) out.push(BLANK());
    out.push(...blockLines(b, W, cut, now));
    if (cut != null) { partial = true; break; }
  }
  if (now != null && out.length) {
    const last = out[out.length - 1], x = lineEnd(last);
    const caret = s(g('caret'), viewInk());
    if (x < W - RM) last.p.push([Math.max(TX, x), [caret]]); else out.push(L(TX, [caret]));
  }
  return out;
}
const cutText = (t, c) => c == null ? t : closeMarks(t.slice(0, c));
function closeMarks(t) {
  t = t.replace(/(^|[^*])\*$/, '$1');
  if ((t.match(/`/g) || []).length % 2) return /`$/.test(t) ? t.slice(0, -1) : t + '`';
  if ((t.match(/\*\*/g) || []).length % 2) return /\*\*$/.test(t) ? t.slice(0, -2) : t + '**';
  return t;
}
function blockLines(b, W, cut, now) {
  const w = textW(W), dry = segs => cut == null ? segs : recolorTail(segs, 10, now, viewInk());
  if (b.h) return wrapSegs(dry([s(cutText(b.h, cut), 'fg', 'b')]), w).map(r => L(TX, r));
  if (b.p) return wrapSegs(dry(inl(cutText(b.p, cut), 'fg')), w).map(r => L(TX, r));
  if (b.quote) return wrapSegs(dry(inl(cutText(b.quote, cut), 'muted')), w - 2).map(r => ln({ p: [[TX, [s(g('quote'), 'faint')]], [TX + 2, r]] }));
  if (b.ul || b.ol) {
    const items = b.ul || b.ol, out = []; let left = cut ?? Infinity;
    items.forEach((it, i) => {
      if (left <= 0) return;
      const txt = left < it.length ? closeMarks(it.slice(0, left)) : it; left -= it.length;
      const mark = b.ul ? s(g('bullet'), 'dim') : s(`${i + 1}.`, 'dim'), ind = b.ul ? 2 : 3;
      const rows = wrapSegs(left < 0 ? dry(inl(txt, 'fg')) : inl(txt, 'fg'), w - ind);
      rows.forEach((r, j) => out.push(ln({ p: j ? [[TX + ind, r]] : [[TX, [mark]], [TX + ind, r]] })));
    });
    return out;
  }
  if (b.code != null) return codeLines(cutText(b.code, cut), b.lang, W);
  if (b.table) return tableLines(b.table, W, cut);
  return [];
}

/* code: a well, the language at its corner, a few honest colours */
const KW = new Set('fn let mut pub use impl struct enum match if else for while loop return async await move ref self Self where trait type const static mod crate super as in dyn unsafe true false Some None Ok Err'.split(' '));
function highlight(line, lang) {
  if (lang === 'sh' || lang === 'bash') {
    if (/^\s*#/.test(line)) return [s(line, 'dim', 'i')];
    const m = line.match(/^(\s*\$?\s*)(\S+)(.*)$/);
    return m ? [s(m[1], 'dim'), s(m[2], 'fn'), s(m[3], 'fg')] : [s(line, 'fg')];
  }
  const out = [], re = /(\/\/.*$)|("(?:[^"\\]|\\.)*")|(\b\d[\d_]*(?:\.\d+)?\b)|([A-Za-z_][A-Za-z0-9_]*!?)|(\s+)|([^\sA-Za-z0-9_"]+)/g;
  let m;
  while ((m = re.exec(line))) {
    if (m[1]) out.push(s(m[1], 'dim', 'i'));
    else if (m[2]) out.push(s(m[2], 'str'));
    else if (m[3]) out.push(s(m[3], 'num'));
    else if (m[4]) out.push(s(m[4], KW.has(m[4]) ? 'kw' : m[4].endsWith('!') ? 'fn' : /^[A-Z]/.test(m[4]) ? 'ty' : /^[a-z_]+$/.test(m[4]) && line[re.lastIndex] === '(' ? 'fn' : 'fg'));
    else out.push(s(m[0], 'fg'));
  }
  return out;
}
function codeLines(code, lang, W) {
  const x0 = TX, x1 = Math.min(W - RM, TX + MEASURE), out = [];
  const lines = code.split('\n');
  out.push(ln({ bands: [[x0, x1, 'well']], r: lang ? [s(lang, 'dim')] : null, rm: W - x1 + 2 }));
  for (const t of lines) out.push(ln({ bands: [[x0, x1, 'well']], p: [[x0 + 2, fitSegs(highlight(t, lang), x1 - x0 - 4)]] }));
  out.push(ln({ bands: [[x0, x1, 'well']] }));
  return out;
}
function tableLines(rows, W, cut) {
  let left = cut ?? Infinity; const shown = [];
  for (const r of rows) { const n = r.join('').length; if (left < n && shown.length) break; shown.push(r); left -= n; }
  const cols = shown[0].length, wd = Array.from({ length: cols }, (_, c) => Math.max(...shown.map(r => sw(r[c] || ''))));
  const out = [], put = (r, head) => {
    const p = []; let x = TX;
    r.forEach((c, i) => { p.push([x, head ? [s(c, 'muted')] : inl(c, 'fg')]); x += wd[i] + 3; });
    return ln({ p });
  };
  out.push(put(shown[0], true));
  out.push(L(TX, [s(g('thin').repeat(wd.reduce((a, b) => a + b, 0) + 3 * (cols - 1)), 'faint')]));
  shown.slice(1).forEach(r => out.push(put(r)));
  return out;
}

/* ----- welcome: the cursor writes the name, embers rise off it ----- */
function markCells() {
  const rows = [];
  for (let y = 0; y < MARK.length; y += 2) {
    let line = '';
    for (let x = 0; x < MARK[0].length; x++) {
      const a = MARK[y][x], b = MARK[y + 1] && MARK[y + 1][x];
      line += a && b ? '█' : a ? '▀' : b ? '▄' : ' ';
    }
    rows.push(line);
  }
  return rows;
}
function welcomeLines(now, W, H) {
  const wl = S.welcome, mark = S.ascii ? MARK_ASCII : markCells(), mw = mark[0].length;
  const recents = RECENTS, gw = Math.max(mw + 4, 52), x0 = Math.max(TX, Math.floor((W - gw) / 2));
  const block = mark.length + 2 + 2 + 2 + 1 + recents.length;
  const top = Math.max(1, Math.floor((H - block) * .42));
  const out = Array.from({ length: top }, () => BLANK());
  const k = S.reduced ? 1 : easeOut((now - wl.at) / 900), cx = x0 + Math.round(k * mw) + 1;
  mark.forEach((row, y) => {
    const segs = [...row].map((ch, x) => {
      const ax = x0 + x;
      if (ax >= cx - 1) return s(' ');
      const d = cx - 1 - ax;
      return s(ch, d < 6 && k < 1 ? mix('glow', 'fg', d / 6) : 'fg');
    });
    const l = L(x0, segs);
    if (y >= 1) l.p.push([cx, [s(g('caret'), k < 1 ? 'glow' : caretInk(now, 'glow', 'bg'))]]);
    out.push(l);
  });
  out[top].fx = { embers: wl.at, x: cx, h: mark.length };
  out.push(BLANK());
  out.push(L(x0, [s(S.model.id, 'fg'), s(g('sep'), 'dim'), s(S.model.eff, 'muted'), s(g('sep'), 'dim'), s(MODES[S.mode].n, MODES[S.mode].c)]));
  out.push(L(x0, [s('~/bingo-improve', 'muted'), s(' on ', 'dim'), s('dev', 'muted'), s(g('sep'), 'dim'), s('3 changed', 'muted')]));
  out.push(BLANK());
  out.push(L(x0, [s('recent', 'dim')]));
  recents.forEach((r, i) => {
    const sel = S.focus === 'welcome' && wl.sel === i;
    const l = ln({ p: [[x0, [s(r.t, sel ? 'fg' : 'muted', sel ? 'b' : '')]]], r: [s(r.when, 'dim')], rm: W - (x0 + gw), click: { act: 'resume', i } });
    if (sel) l.bands = [[x0 - 2, x0 + gw + 2, 'pickBg']];
    out.push(l);
  });
  if (wl.out) for (const l of out) { l.fx = { dissolve: wl.out }; l.click = null; }
  return out;
}
