/* ---------- actions: what an answer, a command or a panel does to the session ---------- */
const printable = k => [...k].length === 1;
function toast(m, ms = 2400) { S.toast = { segs: typeof m === 'string' ? [s(m, 'muted')] : m, at: NOW(), ms }; }
/* a toast of parts: a string is quiet text, a segment keeps its own ink */
function say(...parts) {
  const ms = typeof parts[parts.length - 1] === 'number' ? parts.pop() : 2400;
  toast(parts.flatMap((p, i) => [...(i ? [s(g('sep'), 'faint')] : []), typeof p === 'string' ? s(p, 'muted') : p]), ms);
}
const okToast = (text, ms) => toast([s(`${g('ok')} `, 'good'), s(text, 'muted')], ms);
function notice(segs) { S.log.push({ kind: 'notice', segs, at: NOW() }); S.scroll = 0; }
function closeDock() { S.dock = null; S.focus = 'input'; }
/* one line typed somewhere other than the composer */
function editText(o, key, k) {
  if (k === 'backspace') o[key] = [...o[key]].slice(0, -1).join('');
  else if (k === 'space') o[key] += ' ';
  else if (printable(k)) o[key] += k;
  else return false;
  return true;
}
/* stopping a turn hands back what you queued behind it */
function halt(t, why = 'stopped') {
  stopTurn(t, why);
  if (!S.queued.length || S.input) return;
  S.input = S.queued.map(q => q.text).join('\n');
  S.chips = S.queued.flatMap(q => q.chips || []);
  S.queued = [];
  say('stopped', 'what you queued is back in the input');
}
function submitText(text) {
  if (liveTurn()) { S.queued.push({ text, chips: [], at: NOW() }); return say('queued', text); }
  send(text);
}

/* ----- an ask: once, for the session, no with a reason, or change it first ----- */
function askKey(d, k) {
  if (NOW() - d.at < GUARD_MS) return;
  if (d.typing != null) return askTyping(d, k);
  const n = d.spec.options.length;
  if (k === 'up' || k === 'down') d.cur = clamp(d.cur + (k === 'up' ? -1 : 1), 0, n - 1);
  else if (k === 'enter') answerAsk(d, d.cur);
  else if (k === 'esc') leaveAsk(d, 'said no');
  else if (printable(k)) { d.cur = d.spec.options.findIndex(o => o.a === 'deny'); d.typing = k; }
}
/* esc on an ask: at home it ends the turn; in a teammate's session it only takes you home */
function leaveAsk(d, why) {
  if (viewOf()) return enterView('main');
  halt(d.t, why);
}
function askTyping(d, k) {
  if (k === 'esc') { d.typing = null; return; }
  if (k !== 'enter') { editText(d, 'typing', k); return; }
  const text = d.typing.trim();
  if (d.spec.options[d.cur].a === 'change') changeAsk(d, text); else deny(d, text);
}
function answerAsk(d, i) {
  const o = d.spec.options[i];
  if (o.free) { d.cur = i; d.typing = o.a === 'change' ? d.step.arg : ''; return; }
  if (o.a === 'session') { S.rules.push(d.spec.rule); okToast(`allowed ${o.scope} for this session`); }
  allow(d);
}
function allow(d) { d.step.st = 'run'; resume(d.t); }
function changeAsk(d, text) {
  if (!text) { d.typing = null; return; }
  if (text !== d.step.arg) Object.assign(d.step, { arg: text, changed: true });
  allow(d);
}
function deny(d, why) {
  const t = d.t, x = d.step, now = NOW();
  Object.assign(x, { st: 'skip', res: 'said no', end: now });
  resume(t);
  if (why) t.items.push({ k: 'you', at: now, text: why, note: 'said no' });
  branch(t, deniedBeats(d.ask, why, x));
}
/* bingo takes a no gracefully: what it did not do, and what it can do instead */
function deniedBeats(kind, why, x) {
  const heard = why ? `明白，「${why.length > 30 ? why.slice(0, 30) + '…' : why}」。` : '';
  const text = {
    edit: `${heard}好，\`${baseName(x.arg)}\` 没动。要我换个改法，还是先把计划写给你看？`,
    bash: `${heard}好，不跑了。改动已经写好，你方便的时候跑一下 \`${x.arg}\` 就行。`,
    caution: `${heard}好，不强推。远端的 \`4e1a9c2\` 和 \`b81f0d3\` 都还在。要不要先 \`git pull --rebase\` 把它们接进来，再正常推？`,
  }[kind] || `${heard}好，这一步跳过。`;
  return [B.reply(500, [{ p: text }])];
}

/* ----- a question: tabs when there are several, a reason for each choice, your own words last ----- */
const qOpts = d => [...d.qs[d.tab].options, { label: 'Something else', free: true }];
function questionKey(d, k) {
  if (NOW() - d.at < GUARD_MS) return;
  if (d.typing != null) return questionTyping(d, k);
  const Q = d.qs[d.tab], cur = d.cur[d.tab];
  if (k === 'left' || k === 'right') d.tab = clamp(d.tab + (k === 'left' ? -1 : 1), 0, d.qs.length - 1);
  else if (k === 'up' || k === 'down') d.cur[d.tab] = clamp(cur + (k === 'up' ? -1 : 1), 0, qOpts(d).length - 1);
  else if (k === 'space' && Q.multi && cur < Q.options.length) toggleTick(d, cur);
  else if (k === 'enter') chooseOption(d, cur);
  else if (k === 'esc') leaveAsk(d);
  else if (printable(k)) { d.cur[d.tab] = qOpts(d).length - 1; d.typing = k; }
}
function toggleTick(d, i) {
  const T = d.ticks[d.tab], j = T.indexOf(i);
  if (j < 0) T.push(i); else T.splice(j, 1);
}
function chooseOption(d, i) {
  const Q = d.qs[d.tab];
  if (i >= Q.options.length) { d.typing = ''; return; }
  if (!Q.multi) d.answers[d.tab] = Q.options[i].label;
  else {
    if (!d.ticks[d.tab].length) d.ticks[d.tab].push(i);
    d.answers[d.tab] = d.ticks[d.tab].slice().sort((a, b) => a - b).map(j => Q.options[j].label);
  }
  nextQuestion(d);
}
function questionTyping(d, k) {
  if (k === 'esc') { d.typing = null; return; }
  if (k !== 'enter') { editText(d, 'typing', k); return; }
  const text = d.typing.trim();
  d.typing = null;
  if (!text) return;
  d.answers[d.tab] = d.qs[d.tab].multi ? [text] : text;
  nextQuestion(d);
}
function nextQuestion(d) {
  const i = d.qs.findIndex((_, j) => d.answers[j] == null);
  if (i < 0) return finishQuestion(d);
  d.tab = i; d.at = NOW();
}
function finishQuestion(d) {
  const t = d.t, ans = d.answers, text = a => Array.isArray(a) ? a.join(', ') : a;
  Object.assign(d.step, { st: 'ok', end: NOW(), res: ans.length > 1 ? plural(ans.length, 'answer') : text(ans[0]), qa: d.qs.map((q, i) => [q.question, text(ans[i])]) });
  resume(t);
  if (d.then) branch(t, d.then(ans));
}

/* ----- find: the dock becomes the search field; esc puts the view back where it was ----- */
function openFind() {
  if (!S.log.length) return say('nothing to find yet');
  S.dock = { kind: 'find', q: '', hits: [], cur: null, want: false, from: S.scroll };
  S.focus = 'dock';
}
function findKey(d, k) {
  if (k === 'esc') { S.scroll = d.from; closeDock(); return; }
  if (k === 'enter' || k === 'ctrl+f') { S.scrollAt = NOW(); closeDock(); return; }
  if (k === 'up' || k === 'down') {
    if (d.hits.length) { d.cur = clamp(d.cur + (k === 'up' ? -1 : 1), 0, d.hits.length - 1); d.want = true; }
    return;
  }
  if (editText(d, 'q', k)) { d.cur = null; d.want = true; }
}

/* ----- details: the last turn's thoughts and steps, one key in and one key out ----- */
const rowItem = (t, it) => it.k === 'step' || (it.k === 'think' && t.open);
function toggleLedger() {
  if (S.focus === 'ledger') return foldLedger();
  const t = S.log.filter(e => e.kind === 'turn' && !isLive(e) && !e.shell && e.items.some(it => it.k === 'step' || it.k === 'think')).pop();
  if (!t) return say('no steps to show yet');
  t.open = true; t.well = -1;
  S.sel = { t, i: t.items.findIndex(it => rowItem(t, it)) };
  S.focus = 'ledger'; S.wantSel = true;
}
function foldLedger() {
  const t = S.sel && S.sel.t;
  if (t) { t.open = false; t.well = -1; }
  S.sel = null; S.focus = 'input';
}
function ledgerKey(k) {
  const { t, i } = S.sel, rows = t.items.map((it, j) => rowItem(t, it) ? j : -1).filter(j => j >= 0), at = rows.indexOf(i);
  if (k === 'up' || k === 'down') S.sel.i = rows[clamp(at + (k === 'up' ? -1 : 1), 0, rows.length - 1)];
  else if (k === 'enter' || k === 'space') t.well = t.well === i ? -1 : i;
  else if (k === 'esc' || k === 'ctrl+o') { foldLedger(); return true; }
  else return false;
  S.wantSel = true;
  return true;
}

/* ----- mode, model, effort: one key each, the dock edge and a toast confirm ----- */
function setMode(i) {
  const m = MODES[S.mode = i];
  S.edge = { kind: 'mode', at: NOW(), ink: modeInk(m) };
  say(s(m.n, modeInk(m), 'b'), m.d, 2800);
}
const cycleMode = () => setMode((S.mode + 1) % MODES.length);
function nextModel() {
  const i = MODELS.findIndex(m => m.id === S.model.id), m = MODELS[(i + 1) % MODELS.length];
  S.model = { id: m.id, eff: nearEffort(m, S.model.eff) };
  say(s(m.id, 'fg', 'b'), s(S.model.eff, 'muted'), `${m.prov}, ${m.ctx} context`);
}
function nextEffort() {
  const m = modelOf(), e = m.effs[(m.effs.indexOf(S.model.eff) + 1) % m.effs.length];
  S.model.eff = e;
  say(s(`effort ${e}`, 'fg', 'b'), EFFORT[e]);
}
const stepEffort = (m, want, dir) => m.effs[clamp(m.effs.indexOf(nearEffort(m, want)) + dir, 0, m.effs.length - 1)];

/* ----- things you add to a message, things you send away ----- */
function pasteImage() {
  if (!modelOf().img) return say(s(`${S.model.id} can't read images`, 'amber'), `${keyText('alt+m')} switches model`);
  S.chips.push(`image ${++S.images}`);
}
function backgroundBash() {
  const t = liveTurn(), x = t && steps(t).find(y => y.st === 'run' && y.tool === 'bash');
  if (!x) return say('nothing running to send to the background');
  Object.assign(x, { st: 'ok', res: 'in background', end: NOW() });
  S.jobs.push({ cmd: x.arg, at: NOW() });
  branch(t, [B.reply(300, [{ p: `\`${x.arg}\` 在后台跑着，跑完我告诉你。这段时间你可以接着说别的。` }])]);
  later(9000, () => jobDone(x.arg));
}
function jobDone(cmd) {
  S.jobs = S.jobs.filter(j => j.cmd !== cmd);
  notice([s(`${g('ok')} `, 'good'), s(cmd, 'code'), s(' finished in the background', 'muted'), s(g('sep'), 'faint'), s('412 passed', 'muted')]);
  okToast('background job finished · 412 passed');
}
function resumeSession(title) {
  enterView('main'); S.views = {}; S.sessions = {};
  S.log = []; S.queued = []; S.sel = null; S.scroll = 0;
  closeDock();
  if (S.welcome && !S.welcome.out) S.welcome.out = NOW();
  S.log.push(title === RECENTS[0].t ? T1() : simulate(title, pastScript(title)));
  S.title = title;
  okToast(`resumed · ${title}`);
}
function compact() {
  if (liveTurn()) return say('compact waits for this turn to end');
  const from = Math.round(S.ctx * 100);
  S.ctx = .18;
  notice([s('compacted', 'muted'), s(g('sep'), 'faint'), s(`context ${from}% ${g('arrow')} 18%`, 'dim')]);
  S.edge = { kind: 'good', at: NOW() };
}
function rename(args) {
  if (!args) { S.input = '/rename '; return; }
  S.title = args;
  okToast(`renamed · ${args}`);
}
const restart = (o = {}) => { S = fresh(Object.assign(base(), { welcome: { at: NOW(), sel: 0 } }, o)); };
function exitApp() { enterView('main'); closeDock(); S.exited = { at: NOW() }; }
/* a wake-up you set comes due: bingo starts a turn of its own */
function wakeNow() {
  S.wake = null;
  if (liveTurn()) return later(4000, wakeNow);
  startTurn('再看一眼 CI，过了就告诉我', ciScript(), { wake: true });
}
function ciScript() {
  const out = ['✓ fmt', '✓ clippy', '✓ test (ubuntu)', '✓ test (macos)', '✓ test (windows)'];
  return [B.step(0, 'bash', 'gh run watch --exit-status'), ...out.map((l, i) => B.tail(200 + i * 160, l)),
    B.ok(1100, 'passed', { out }), B.reply(1300, [{ p: 'CI 全绿了，Windows 也过了。可以合了。' }])];
}

const COMMAND_ACTS = {
  model: () => openSheet('model'), think: () => openSheet('model'), compact, status: () => openSheet('status'),
  resume: () => openSheet('sessions'), rewind: () => openSheet('rewind'), permission: () => openSheet('mode'),
  agents: () => openSheet('crew'), team: () => openSheet('crew'), room: () => openSheet('crew'),
  seat: seatAgent, unseat: unseatAgent, close: closeRoom,
  tasks: () => openSheet('tasks'), schedule: () => openSheet('schedule'), wake: () => openSheet('wake'),
  plugins: () => openSheet('plugins'), mcp: () => openSheet('plugins'), memory: () => openSheet('memory'), experience: () => openSheet('memory'),
  rename, provider: () => openSheet('login'), login: () => openSheet('login'),
  logout: () => { S.signedOut = true; say('signed out of anthropic', s('/login', 'glow'), 'signs you in again'); },
  clear: () => restart({ model: S.model, mode: S.mode }), help: () => openSheet('keys'),
};
function runCommand(name, args = '') {
  S.input = ''; S.pal.sel = 0;
  const f = COMMAND_ACTS[name];
  if (f) f(args); else say(`/${name} is not in this prototype`);
}

/* ----- sheets: every panel has the same anatomy — a head, a list, a foot ----- */
function openSheet(name, o = {}) {
  const sh = SHEETS[name];
  S.dock = Object.assign({ kind: 'sheet', sheet: name, sel: 0, q: '', at: NOW() }, sh.init ? sh.init() : {}, o);
  S.focus = 'dock';
  if (sh.open) sh.open(S.dock);
}
const RANK = { asking: 0, working: 1 };
const byNeed = (a, b) => (RANK[a] ?? 2) - (RANK[b] ?? 2);
/* a list row's lamp: amber when it needs you, breathing while it works, quiet otherwise */
function stateMark(st, now, ink = 'glow') {
  if (st === 'asking') return s(g('lamp'), mix('amberGlow', 'you', .6 * breathe(now, 1100)));
  if (st === 'working') return s(g('lamp'), mix(ink, 'you', .55 * breathe(now, 1800)));
  return s(g('lamp'), 'faint');
}
const needWord = (st, idle) => st === 'asking' ? s('needs you', 'amber') : st === 'working' ? s('working', 'muted') : s(idle, 'dim');
const pickInk = (sel, ink = 'muted') => [sel ? 'fg' : ink, sel ? 'b' : ''];
function filterLine(d, now, hint) {
  const caret = s(g('caret'), caretInk(now));
  return L(TX, [s('filter', 'dim'), s('  '), ...(d.q ? [s(d.q, 'fg'), caret] : [caret, s(hint, 'dim')])]);
}
function meter(k, w) {
  const n = Math.round(clamp(k, 0, 1) * w), [on, off] = S.ascii ? ['=', '-'] : ['━', '─'];
  return [s(on.repeat(n), k >= .8 ? 'amber' : 'glow'), s(off.repeat(w - n), 'faint')];
}
function effortScale(m, want) {
  const e = nearEffort(m, want), p = [s('effort', 'dim'), s('   ')];
  m.effs.forEach((x, i) => p.push(...(i ? [s('  ')] : []), s(x, x === e ? 'glow' : 'dim', x === e ? 'b' : '')));
  return [L(TX, p), L(TX, [s(EFFORT[e], 'muted')])];
}
const costOf = () => .08 + S.log.filter(e => e.kind === 'turn').length * .11;
function statusRows() {
  const m = modelOf(), mode = MODES[S.mode], on = S.plugins.filter(p => p.on), bad = S.plugins.find(p => p.err);
  const busy = crewList().filter(a => a.state !== 'idle');
  return [
    ['model', [s(m.id, 'fg'), s(` ${S.model.eff}`, 'muted'), s(g('sep'), 'faint'), s(m.prov, 'dim')], () => openSheet('model')],
    ['mode', [s(mode.n, modeInk(mode)), s(g('sep'), 'faint'), s(mode.d, 'dim')], () => openSheet('mode')],
    ['context', [...meter(S.ctx, 20), s(`  ${Math.round(S.ctx * 100)}% of ${m.ctx}`, S.ctx >= .8 ? 'amber' : 'muted')], () => { closeDock(); compact(); }],
    ['cost', [s(`$${costOf().toFixed(2)}`, 'fg'), s(' this session', 'dim')], null],
    ['directory', [s(HERE.dir, 'fg'), s(' on ', 'dim'), s(HERE.branch, 'muted'), s(g('sep'), 'faint'), s(`${HERE.changed} changed`, 'muted')], null],
    ['plugins', [s(`${on.length} on`, 'fg'), ...(bad ? [s(g('sep'), 'faint'), s(`${bad.n} ${bad.err}`, 'amber')] : [])], () => openSheet('plugins')],
    ['agents', busy.length ? [s(`${busy.length} working`, 'fg')] : [s('none working', 'dim')], () => openSheet('crew')],
    ['rules', S.rules.length ? [s(S.rules.join(', '), 'code')] : [s('none yet', 'dim')], () => openSheet('mode')],
  ];
}
const pluginList = () => [...S.plugins].sort((a, b) => !!b.err - !!a.err);
function switchPlugin(p) {
  p.on = !p.on;
  say(s(`${p.n} ${p.on ? 'on' : 'off'}`, p.on ? 'good' : 'muted'), 'from the next turn');
}
function sessionList(d) {
  const q = d.q.toLowerCase();
  return SESSIONS.map(x => ({ x, f: fuzzy(q, `${x.t} ${x.dir}`) })).filter(y => y.f)
    .sort((a, b) => byNeed(a.x.st, b.x.st) || b.f.score - a.f.score).map(y => y.x);
}
const rewindList = () => S.log.filter(e => e.kind === 'turn' && !e.shell).reverse();
function filesSince(t) {
  const files = new Set();
  for (const e of S.log.slice(S.log.indexOf(t))) if (e.kind === 'turn') for (const x of steps(e)) if (edits(x) && x.st === 'ok') files.add(x.arg);
  return [...files];
}
function rewindTo(t) {
  if (!t) return;
  const n = filesSince(t).length, live = liveTurn();
  if (live) stopTurn(live);
  S.log = S.log.slice(0, S.log.indexOf(t));
  S.queued = []; S.scroll = 0; S.sel = null;
  closeDock();
  S.input = t.prompt; S.chips = t.chips.slice();
  if (!S.log.length) S.welcome = { at: NOW(), sel: 0 };
  okToast(n ? `rewound · ${plural(n, 'file')} restored` : 'rewound');
}
function loginRows(d, now) {
  const gh = d.prov === 'github', url = gh ? 'github.com/login/device' : 'console.anthropic.com/device';
  const wait = d.ok ? [s(`${g('ok')} `, 'good'), s(`signed in to ${d.prov}`, 'fg')]
    : [s(spinG(now, d.at), 'glow'), s('  waiting for you to approve in the browser', 'muted')];
  return [L(TX, [s('open ', 'dim'), s(url, 'code', { u: true }), s(' and enter this code', 'dim')]), BLANK(),
    L(TX + 2, [s(gh ? 'K7QF-2MXP' : 'WDJB-MJHT', 'fg', 'b')]), BLANK(), L(TX, wait)];
}
function signedIn(d) {
  if (S.dock !== d) return;
  d.ok = NOW();
  const p = S.plugins.find(x => x.n === d.prov);
  if (p) { delete p.err; delete p.fix; }
  if (d.prov === 'anthropic') S.signedOut = false;
  later(900, () => afterSignIn(d));
}
function afterSignIn(d) {
  if (S.dock === d) closeDock();
  if (d.prov !== 'anthropic' || !S.resend) return okToast(`signed in to ${d.prov}`);
  const text = S.resend;
  S.resend = null;
  okToast('signed in · sending your message again');
  send(text);
}
const KEYMAP = [
  ['write', [['⏎', 'send'], ['⇧⏎', 'new line'], ['/', 'commands and skills'], ['@', 'files and agents'], ['!', 'run a shell command'], ['ctrl+v', 'paste an image']]],
  ['while it works', [['⏎', 'steer at the next step'], ['tab', 'queue for after'], ['↑', 'edit what you queued'], ['esc', 'stop the turn'], ['ctrl+b', 'send the command to the background']]],
  ['look back', [['ctrl+o', 'details of the last turn'], ['ctrl+f', 'find in the conversation'], ['pgup pgdn', 'scroll'], ['end', 'back to the latest'], ['esc esc', 'rewind']]],
  ['switch', [['⇧tab', 'permission mode'], ['alt+m', 'model'], ['alt+t', 'effort'], ['ctrl+r', 'sessions'], ['ctrl+t', 'tasks'], ['ctrl+g', 'whoever needs you'], ['ctrl+c', 'stop, twice to quit']]],
];
const keyGroup = ([name, ks], w) => [[s(name, 'dim')], ...ks.map(([k, a]) => [...padTo([s(keyText(k), 'fg')], 11), ...fitSegs([s(a, 'muted')], w - 11)])];
function keysRows(W) {
  const two = W >= 90, colW = Math.floor((W - TX - RM) / 2), out = [];
  const groups = KEYMAP.map(gr => keyGroup(gr, two ? colW - 2 : W - TX - RM));
  const step = two ? 2 : 1;
  for (let i = 0; i < groups.length; i += step) {
    if (i) out.push(BLANK());
    const a = groups[i], b = two ? groups[i + 1] || [] : [];
    for (let j = 0; j < Math.max(a.length, b.length); j++) out.push(ln({ p: [...(a[j] ? [[TX, a[j]]] : []), ...(b[j] ? [[TX + colW, b[j]]] : [])] }));
  }
  return out;
}
const TASK_ORDER = { in_progress: 0, pending: 1, completed: 2 };
const taskList = () => [...S.tasks].sort((a, b) => TASK_ORDER[a.st] - TASK_ORDER[b.st] || b.id - a.id);
const whoInk = w => w === 'bingo' ? 'glow' : crewInk(w);
function taskMark(x, now) {
  if (x.st === 'completed') return s(g('ok'), 'good');
  if (x.st === 'in_progress') return s(spinG(now, x.id * 170), whoInk(x.who));
  return s(g('still'), 'dim');
}
const WAKES = [['10s', 1e4], ['1m', 6e4], ['5m', 3e5], ['10m', 6e5], ['30m', 18e5], ['1h', 36e5]];
function forget(d) {
  const m = S.memory[d.sel];
  if (!m) return;
  if (m.kind !== 'memory') return say('instructions live in files', `edit ${m.t}`);
  S.memory.splice(d.sel, 1);
  d.sel = clamp(d.sel, 0, S.memory.length - 1);
  okToast(`forgot · ${m.t}`);
}

const SHEETS = {
  model: {
    init: () => ({ sel: Math.max(0, MODELS.findIndex(m => m.id === S.model.id)), eff: S.model.eff }),
    head: () => ({ l: [s('Model', 'fg', 'b')], r: [s(`${S.model.id} ${S.model.eff}`, 'dim')] }),
    rows: d => MODELS.map((m, i) => ({
      segs: [...padTo([s(m.id, ...pickInk(i === d.sel))], 15), ...padTo([s(m.prov, 'dim')], 12), ...padTo([s(m.ctx, 'dim')], 7), s(m.img ? 'reads images' : 'text only', 'dim')],
      r: m.id === S.model.id ? [s('in use', 'good')] : null,
    })),
    foot: d => effortScale(MODELS[d.sel], d.eff),
    key: (d, k) => { if (k !== 'left' && k !== 'right') return false; d.eff = stepEffort(MODELS[d.sel], d.eff, k === 'left' ? -1 : 1); return true; },
    enter: d => { const m = MODELS[d.sel]; S.model = { id: m.id, eff: nearEffort(m, d.eff) }; closeDock(); okToast(`${m.id} ${S.model.eff}`); },
    keys: () => [['↑↓', 'model'], ['←→', 'effort'], ['⏎', 'use'], ['esc', 'close']],
  },
  status: {
    head: () => ({ l: [s('Status', 'fg', 'b')], r: [s(S.title || 'this session', 'dim')] }),
    rows: () => statusRows().map(([k, v]) => ({ segs: [...padTo([s(k, 'dim')], 12), ...v] })),
    enter: d => { const go = statusRows()[d.sel][2]; if (go) go(); },
    keys: d => [['↑↓', 'line'], ...(statusRows()[d.sel][2] ? [['⏎', 'open']] : []), ['esc', 'close']],
  },
  plugins: {
    head: () => ({ l: [s('Plugins', 'fg', 'b')], r: [s(`${S.plugins.filter(p => p.on).length} of ${S.plugins.length} on`, 'dim')] }),
    rows: d => pluginList().map((p, i) => ({
      segs: [s(p.on ? g('ok') : g('dot'), p.on ? 'good' : 'dim'), s('  '), ...padTo([s(p.n, ...pickInk(i === d.sel, p.on ? 'muted' : 'dim'))], 13), s(p.d, 'dim')],
      r: p.err ? [s(p.err, 'amber')] : [s(p.kind, 'faint')],
    })),
    key: (d, k) => { if (k !== 'space') return false; switchPlugin(pluginList()[d.sel]); return true; },
    enter: d => { const p = pluginList()[d.sel]; if (p.err) openSheet('login', { prov: p.n }); else switchPlugin(p); },
    keys: d => [['↑↓', 'plugin'], ['space', 'switch'], ...(pluginList()[d.sel].err ? [['⏎', 'sign in']] : []), ['esc', 'close']],
  },
  sessions: {
    opener: 'ctrl+r',
    head: d => ({ l: [s('Sessions', 'fg', 'b')], r: [s(d.q ? `${sessionList(d).length} of ${SESSIONS.length}` : 'needs you first', 'dim')] }),
    top: (d, now) => [filterLine(d, now, 'type to filter by title or directory'), BLANK()],
    rows: (d, now) => sessionList(d).map((x, i) => ({
      segs: [stateMark(x.st, now), s('  '), ...padTo([s(x.t, ...pickInk(i === d.sel))], 26), s(x.dir, 'dim')],
      r: [needWord(x.st, x.when)],
    })),
    key: (d, k) => { if (!editText(d, 'q', k)) return false; d.sel = 0; return true; },
    enter: d => { const x = sessionList(d)[d.sel]; if (x) resumeSession(x.t); },
    keys: () => [['↑↓', 'session'], ['⏎', 'open'], ['esc', 'close']],
  },
  rewind: {
    head: () => ({ l: [s('Rewind', 'fg', 'b')], r: [s('to just before a prompt', 'dim')] }),
    top: () => [L(TX, [s('the conversation and the files both go back', 'dim')]), BLANK()],
    rows: (d, now, W) => rewindList().map((t, i) => {
      const n = filesSince(t).length;
      return { segs: fitSegs([...chipSegs(t.chips), ...inl(t.prompt.replace(/\s+/g, ' '), i === d.sel ? 'fg' : 'muted')], W - TX - RM - 16), r: [s(n ? `${plural(n, 'file')} back` : 'no files', 'dim')] };
    }),
    enter: d => rewindTo(rewindList()[d.sel]),
    keys: () => [['↑↓', 'prompt'], ['⏎', 'rewind'], ['esc', 'close']],
  },
  login: {
    pick: false,
    init: () => ({ prov: 'anthropic' }),
    open: d => later(3600, () => signedIn(d)),
    head: d => ({ l: [s(`Sign in to ${d.prov}`, 'fg', 'b')], r: [s('device code', 'dim')] }),
    rows: (d, now) => loginRows(d, now),
    keys: d => d.ok ? [] : [['esc', 'cancel']],
  },
  keys: {
    pick: false, opener: '?', max: 40,
    head: () => ({ l: [s('Keys', 'fg', 'b')], r: [s('/help lists every command', 'dim')] }),
    rows: (d, now, W) => keysRows(W),
    keys: () => [['esc', 'close']],
  },
  schedule: {
    head: () => ({ l: [s('Schedule', 'fg', 'b')], r: [s(`${S.scheds.filter(x => x.on).length} running`, 'dim')] }),
    rows: (d, now) => S.scheds.map((x, i) => ({
      segs: [s(g('lamp'), x.on ? mix('glow', 'you', .5 * breathe(now, 2400)) : 'faint'), s('  '), ...padTo([s(x.when, x.on ? 'muted' : 'dim')], 16), s(x.what, ...pickInk(i === d.sel, x.on ? 'muted' : 'dim'))],
      r: [s(x.on ? `next ${x.next}` : 'paused', 'dim')],
    })),
    foot: () => [L(TX, [s('/schedule every 30m ', 'code'), s('<prompt>', 'dim'), s(' adds one', 'dim')])],
    key: (d, k) => { if (k !== 'space') return false; const x = S.scheds[d.sel]; x.on = !x.on; say(x.on ? 'resumed' : 'paused', x.what); return true; },
    enter: d => { const x = S.scheds[d.sel]; closeDock(); enterView('main'); submitText(x.what); },
    keys: () => [['↑↓', 'schedule'], ['space', 'pause'], ['⏎', 'run now'], ['esc', 'close']],
  },
  tasks: {
    opener: 'ctrl+t',
    head: () => ({ l: [s('Tasks', 'fg', 'b')], r: [s(`${S.tasks.filter(x => x.st === 'completed').length} of ${S.tasks.length} done`, 'dim')] }),
    rows: (d, now) => taskList().map((x, i) => ({ segs: [taskMark(x, now), s('  '), s(x.t, x.st === 'completed' ? 'dim' : i === d.sel ? 'fg' : 'muted')], r: [s(x.who, whoInk(x.who))] })),
    keys: () => [['↑↓', 'task'], ['esc', 'close']],
  },
  mode: {
    init: () => ({ sel: S.mode }),
    head: () => ({ l: [s('Permission mode', 'fg', 'b')], r: [s(`${keyText('⇧tab')} cycles`, 'dim')] }),
    rows: d => MODES.map((m, i) => ({ segs: [...padTo([s(m.n, i === d.sel ? modeInk(m) : 'muted', i === d.sel ? 'b' : '')], 15), s(m.d, 'dim')], r: i === S.mode ? [s('current', 'muted')] : null })),
    foot: () => [L(TX, [s('allowed for this session', 'dim')]),
      ...(S.rules.length ? S.rules.map(r => L(TX + 2, [s(r, 'code')])) : [L(TX + 2, [s(`none yet; "Yes, allow ${g('ell')} for this session" adds one`, 'dim')])])],
    enter: d => { closeDock(); setMode(d.sel); },
    keys: () => [['↑↓', 'mode'], ['⏎', 'use'], ['esc', 'close']],
  },
  wake: {
    head: () => ({ l: [s('Wake up later', 'fg', 'b')], r: [s(S.wake ? `wakes in ${fmtDur(S.wake.at - NOW())}` : 'bingo comes back to this session', 'dim')] }),
    rows: d => [...WAKES.map(([n], i) => ({ segs: [s(`in ${n}`, ...pickInk(i === d.sel))] })), ...(S.wake ? [{ segs: [s('cancel the wake-up', 'bad')] }] : [])],
    enter: d => {
      closeDock();
      if (d.sel >= WAKES.length) { S.wake = null; return say('wake-up cancelled'); }
      S.wake = { at: NOW() + WAKES[d.sel][1] };
      okToast(`wakes in ${WAKES[d.sel][0]}`);
    },
    keys: () => [['↑↓', 'when'], ['⏎', 'set'], ['esc', 'close']],
  },
  memory: {
    head: () => ({ l: [s('Memory', 'fg', 'b')], r: [s(`${S.memory.filter(m => m.kind === 'memory').length} memories`, 'dim')] }),
    rows: d => S.memory.map((m, i) => ({ segs: [...padTo([s(m.kind, 'dim')], 14), s(m.t, ...pickInk(i === d.sel))], r: [s(m.d, 'dim')] })),
    key: (d, k) => { if (k !== 'backspace') return false; forget(d); return true; },
    keys: d => [['↑↓', 'item'], ...((S.memory[d.sel] || {}).kind === 'memory' ? [['⌫', 'forget']] : []), ['esc', 'close']],
  },
};
