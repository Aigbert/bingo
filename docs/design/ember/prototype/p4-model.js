/* ---------- model: session state, turns, beats ---------- */
const RATE_R = 150, RATE_N = 70;           /* characters a second: answer, narration */

function fresh(o = {}) {
  return Object.assign({
    W: 100, H: 32, ascii: false, reduced: false, timers: [], clicks: [], toast: null, keyAt: -1e9, jobs: [], wake: null,
    scrollAt: -1e9, ctrlcAt: -1e9, resend: null, exited: null, signedOut: false, images: 0,
    plugins: PLUGINS.map(p => Object.assign({}, p)), tasks: TASKS.map(x => Object.assign({}, x)), scheds: SCHEDS.map(x => Object.assign({}, x)),
    memory: MEMORY.map(x => Object.assign({}, x)), view: 'main', views: {}, sessions: {},
  }, blankSession({ model: { id: 'opus-5.5', eff: 'high' }, ctx: .21 }), o);
}
const base = () => S ? { W: S.W, H: S.H, ascii: S.ascii, reduced: S.reduced } : {};

let TURN_ID = 0;
function mkTurn(prompt, o = {}) {
  return Object.assign({
    kind: 'turn', id: ++TURN_ID, prompt, chips: [], t0: NOW(), state: 'live', open: false, row: -1, well: -1,
    items: [], reply: null, error: null, script: [], si: 0,
    paused: false, pauseAt: 0, pausedMs: 0, endAt: 0, answer: null,
  }, o);
}
const steps = t => t.items.filter(it => it.k === 'step');
const isLive = t => t.state === 'live' || t.state === 'asking';
const turnTime = (t, now) => (t.paused ? t.pauseAt : now) - t.t0 - t.pausedMs;
/* the clock on the band: time spent waiting for you is not bingo's time */
const elapsedOf = (t, now) => (isLive(t) ? turnTime(t, now) : t.endAt - t.t0 - t.pausedMs);
const liveTurn = () => S.log.find(e => e.kind === 'turn' && isLive(e));
const lastTurn = () => S.log.filter(e => e.kind === 'turn').pop();

/* the reveal clock: only words count, decoration does not */
const blockChars = blocks => blocks.reduce((n, b) =>
  n + [b.h, b.p, b.quote, b.code, ...(b.ul || []), ...(b.ol || []), ...((b.table || []).flat())].filter(Boolean).join('').length, 0);
const revealMs = (n, rate) => n / rate * 1000;

/* beats: {at: turn-time ms, f(turn, absolute time)} */
const B = {
  think: (at, ms, text) => ({ at, f: (t, T) => { t.items.push({ k: 'think', at: T, ms, text }); } }),
  narr: (at, text) => ({ at, f: (t, T) => { t.items.push({ k: 'narr', at: T, text: typeof text === 'function' ? text(t) : text }); } }),
  step: (at, tool, arg, o = {}) => ({ at, f: (t, T) => { landSteers(t, T); t.items.push(Object.assign({ k: 'step', at: T, tool, arg, st: 'run', tail: [] }, o)); } }),
  tail: (at, line, id) => ({ at, f: t => { const x = openStep(t, id); if (x) x.tail.push(line); } }),
  ok: (at, res, o = {}) => ({ at, f: (t, T) => close(t, T, 'ok', res, o) }),
  fail: (at, res, o = {}) => ({ at, f: (t, T) => close(t, T, 'fail', res, o) }),
  ask: (at, kind, o = {}) => ({ at, ask: true, f: (t, T) => askBeat(t, T, kind, o) }),
  reply: (at, blocks) => ({ at, f: (t, T) => { t.reply = { at: T, blocks: typeof blocks === 'function' ? blocks(t) : blocks }; } }),
  call: (at, fn) => ({ at, f: fn }),
};
function openStep(t, id) {
  const open = steps(t).filter(x => x.st === 'run' || x.st === 'ask');
  return id ? open.find(x => x.id === id) : open.pop();
}
function close(t, T, st, res, o) {
  const x = openStep(t, o.id); if (!x) return;
  Object.assign(x, { st, end: T, res }, o);
}
/* a mode or a rule may answer before the dock opens; then the step simply keeps running */
function askBeat(t, T, kind, o) {
  const x = openStep(t, o.id); if (!x) return;
  if (kind === 'question') { pauseFor(t, x, T); openQuestion(t, x, T, o); return; }
  const spec = ASK[kind](x, o), by = answeredBy(kind, spec);
  if (by) { x.auto = by; return; }
  pauseFor(t, x, T);
  S.dock = Object.assign({ kind: 'ask', ask: kind, t, step: x, spec, danger: !!spec.danger, cur: spec.start || 0, at: T, typing: null }, o);
  S.focus = 'dock';
}
function pauseFor(t, x, T) {
  x.st = 'ask';
  t.paused = true; t.pauseAt = T; t.state = 'asking';
}
function runScript(t, now, quiet) {
  if (!isLive(t)) return;
  while (!t.paused && t.si < t.script.length && t.script[t.si].at <= turnTime(t, now)) {
    const b = t.script[t.si++];
    b.f(t, t.t0 + t.pausedMs + b.at);
  }
  if (t.paused || !isLive(t)) return;
  if (t.reply && t.si >= t.script.length) {
    const end = t.reply.at + revealMs(blockChars(t.reply.blocks), RATE_R);
    if (end <= now) endTurn(t, end, 'done', quiet);
  }
}
function endTurn(t, at, state = 'done', quiet) {
  if (!isLive(t)) return;
  t.state = state; t.endAt = at; t.si = t.script.length;
  if (!quiet) onTurnEnd(t);
}
function onTurnEnd(t) {
  S.ctx = Math.min(.96, S.ctx + .035);
  if (S.dock && S.dock.t === t) { S.dock = null; S.focus = 'input'; }
  S.edge = { kind: t.state === 'done' ? 'good' : t.state === 'failed' ? 'bad' : 'none', at: t.endAt };
  if (S.queued.length && t.state === 'done') later(700, () => { const q = S.queued.shift(); if (q) send(q.text, q.chips); });
  reportBack(t);
}
/* a timer runs in the session that set it */
function later(ms, f) { S.timers.push({ at: NOW() + ms, f, sid: S.view }); }
function resume(t) {
  if (!t.paused) return;
  t.pausedMs += NOW() - t.pauseAt; t.paused = false; t.state = 'live';
  if (S.dock && S.dock.t === t) S.dock = null;
  S.focus = 'input'; S.edge = null;
}
function stopTurn(t, why = 'stopped') {
  const now = NOW();
  if (t.paused) { t.pausedMs += now - t.pauseAt; t.paused = false; }
  for (const it of t.items) if (it.k === 'narr' && now < it.at + revealMs(it.text.length, RATE_N)) it.text = it.text.slice(0, Math.floor((now - it.at) / 1000 * RATE_N));
  /* what was running is cut where it stands; what waited on you never ran */
  for (const x of steps(t)) {
    if (x.st === 'run') Object.assign(x, { st: 'cut', res: 'stopped', end: now });
    else if (x.st === 'ask') Object.assign(x, { st: 'skip', res: why, end: now });
  }
  if (t.reply) t.reply.cut = Math.floor((now - t.reply.at) / 1000 * RATE_R);
  t.state = 'stopped'; t.endAt = now; t.si = t.script.length;
  if (S.dock && S.dock.t === t) S.dock = null;
  S.focus = 'input'; S.edge = { kind: 'none', at: now };
  S.ctx = Math.min(.96, S.ctx + .015);
}
/* a finished turn from history: every beat already happened long ago */
function simulate(prompt, script, o = {}) {
  const t = mkTurn(prompt, Object.assign({ script, t0: NOW() - 1e7 }, o));
  const keepAsk = S.dock;
  t.script = script.filter(b => !b.ask);
  runScript(t, NOW(), true);
  if (isLive(t)) endTurn(t, t.t0 + script[script.length - 1].at + (t.reply ? revealMs(blockChars(t.reply.blocks), RATE_R) : 0), 'done', true);
  S.dock = keepAsk;
  return t;
}
function startTurn(prompt, script, o = {}) {
  const t = mkTurn(prompt, Object.assign({ script }, o));
  S.log.push(t); S.placeholder = null; S.scroll = 0;
  if (S.welcome && !S.welcome.out) S.welcome.out = NOW();
  if (!S.title) S.title = o.title || shortTitle(prompt);
  S.edge = null;
  return t;
}
function shortTitle(p) { return fitSegs([s(p.replace(/\s+/g, ' '), 'dim')], 28).map(x => x.t).join(''); }
function send(text, chips = []) {
  S.hist.push(text); S.hi = -1;
  const sc = scriptFor(text);
  return startTurn(text, sc.script || sc, Object.assign({ chips }, sc.o || {}));
}

/* ----- what a tool's owner mod declares: a mark the font already has, its ASCII twin, a verb, how its target reads ----- */
/* the TUI names no tool: these rows stand in for what the fs, bash, web … mods hand it; read-only and edit are kernel traits */
const TOOLS = {
  read: { mod: 'fs', mark: '¶', ascii: '<', verb: 'read', arg: 'path', ro: true },
  glob: { mod: 'fs', mark: '*', verb: 'find', arg: 'path', ro: true },
  grep: { mod: 'fs', mark: '/', verb: 'search', ro: true },
  edit: { mod: 'fs', mark: '±', ascii: '~', verb: 'edit', arg: 'path', edit: true },
  write: { mod: 'fs', mark: '+', verb: 'write', arg: 'path', edit: true },
  bash: { mod: 'bash', mark: '$', verb: 'run', arg: 'cmd' },
  kill: { mod: 'bash', mark: '×', ascii: 'x', verb: 'stop', arg: 'cmd' },
  fetch: { mod: 'web', mark: '↓', ascii: 'v', verb: 'fetch', arg: 'path', ro: true },
  search: { mod: 'web', mark: '»', ascii: '>', verb: 'web', ro: true },
  look: { mod: 'pictures', mark: '¤', ascii: 'o', verb: 'look', arg: 'path', ro: true },
  agent: { mod: 'agents', mark: '@' },
  room: { mod: 'rooms', mark: '@', verb: 'seat' },
  todo: { mod: 'tasks', mark: '#', verb: 'plan' },
  ask: { mod: 'fs', mark: '?', verb: 'ask' },
  skill: { mod: 'skills', mark: '§', ascii: '&', verb: 'skill', ro: true },
  wake: { mod: 'schedule', mark: '∞', ascii: '^', verb: 'wake' },
  recall: { mod: 'experience', mark: '√', ascii: '!', verb: 'recall', ro: true },
  mcp: { mod: 'mcp', mark: '◊', ascii: '%' },
  query: { mod: 'postgres', mark: '∑', ascii: '=', verb: 'query', ro: true },
};
const toolOf = x => TOOLS[x.tool] || {};
const looks = x => !!toolOf(x).ro;
const edits = x => !!toolOf(x).edit;
const verbOf = x => x.agent ? x.agent.name : x.via || toolOf(x).verb || x.tool.toLowerCase();
/* a mark that is not one cell wide, or not ASCII when the terminal is, falls back to the bullet */
function markOf(x) {
  const d = toolOf(x), m = S.ascii ? d.ascii || d.mark : d.mark;
  return m && [...m].length === 1 && sw(m) === 1 && (!S.ascii || m.charCodeAt(0) < 128) ? m : g('bullet');
}
