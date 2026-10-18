/* ---------- crew: every teammate and every room is a session of its own; you look at one at a time ---------- */

/* ----- sessions: the fields a view swaps; the rest belongs to the app ----- */
function blankSession(v) {
  return {
    log: [], input: '', chips: [], focus: 'input', dock: null, placeholder: null, queued: [], welcome: null,
    scroll: 0, maxScroll: 0, title: v.title || '', sel: null, edge: null, hist: [], hi: -1, pal: { sel: 0 }, grow: null,
    streamN: 0, viewH: 0, vmap: null, wantSel: false, escAt: -1e9, rules: [], mode: v.mode || 0,
    model: Object.assign({}, v.model), ctx: v.ctx ?? .04,
  };
}
const SESSION_KEYS = Object.keys(blankSession({}));
function stash() { const o = S.sessions[S.view] || (S.sessions[S.view] = {}); for (const k of SESSION_KEYS) o[k] = S[k]; }
function load(id) { const o = S.sessions[id]; for (const k of SESSION_KEYS) S[k] = o[k]; S.view = id; }
/* a session you are not looking at still runs: its turns and timers run inside it */
function withSession(id, f) {
  if (id === S.view) return f();
  const back = S.view;
  stash(); load(id);
  try { return f(); } finally { stash(); load(back); }
}
const sessionOf = id => id === S.view ? S : S.sessions[id];
const sessionIds = () => ['main', ...Object.keys(S.views)];
const viewOf = () => S.views[S.view] || null;
const inRoom = () => { const v = viewOf(); return !!v && v.kind === 'room'; };
const roomOpen = () => inRoom() && !viewOf().closed;
/* who does the work here: bingo at home, the teammate in its own session */
const worker = () => { const v = viewOf(); return v && v.kind === 'agent' ? v.name : 'bingo'; };
/* the accent here: ember at home, the teammate's colour in its own session */
const viewInk = () => { const v = viewOf(); return v && v.kind === 'agent' ? v.c : 'glow'; };
function openSession(id, view) {
  S.views[id] = Object.assign({ id }, view);
  S.sessions[id] = blankSession(view);
  return S.views[id];
}
/* walking in: what you were doing stays where you left it; an ask waiting here re-arms its guard */
function enterView(id) {
  if (id === S.view || (id !== 'main' && !S.views[id])) return;
  if (S.dock && !ASKING(S.dock)) closeDock();
  if (S.focus === 'ledger') foldLedger();
  stash(); load(id);
  S.grow = null;
  if (ASKING(S.dock)) S.dock.at = NOW();
  S.edge = { kind: 'mode', ink: id === 'main' ? 'glow' : S.views[id].c, at: NOW() };
}
/* walking in from a name: a teammate's session opens the first time you visit it */
function walkTo(id) {
  if (!S.views[id] && rosterOf(id)) openAgent(id);
  enterView(id);
}

/* ----- the crew: each agent keeps one colour everywhere it appears ----- */
const ROSTER = [
  { name: 'scout', c: 'crew1', role: 'finds things in the codebase', model: { id: 'haiku-5.5', eff: 'low' } },
  { name: 'reviewer', c: 'crew2', role: 'reads diffs and says what breaks', model: { id: 'opus-5.5', eff: 'high' } },
  { name: 'ada', c: 'crew0', role: 'writes the test first', model: { id: 'sonnet-5.5', eff: 'medium' } },
  { name: 'bo', c: 'crew3', role: 'runs releases and watches CI', model: { id: 'haiku-5.5', eff: 'low' } },
];
const rosterOf = n => ROSTER.find(r => r.name === n);
const crewInk = n => (rosterOf(n) || {}).c || (n === 'you' ? 'fg' : 'muted');
const ASKING = d => !!d && (d.kind === 'ask' || d.kind === 'question');
const liveIn = o => o.log.find(e => e.kind === 'turn' && isLive(e));
const typingIn = n => Object.values(S.views).find(v => v.typing && v.typing.name === n);
/* an agent's state is read off its session, never stored beside it */
function stateOf(id) {
  const o = sessionOf(id);
  if (o && ASKING(o.dock)) return 'asking';
  return (o && liveIn(o)) || typingIn(id) ? 'working' : 'idle';
}
function crewList() {
  return ROSTER.map(r => {
    const o = sessionOf(r.name), t = o && liveIn(o), ty = !t && typingIn(r.name);
    const doing = t ? t.prompt : ty ? `writing in ${ty.name}` : null;
    return Object.assign({ state: stateOf(r.name), doing, at: t ? t.t0 : ty ? ty.typing.at : 0 }, r);
  }).sort((a, b) => byNeed(a.state, b.state));
}
function openAgent(name) {
  const r = rosterOf(name);
  openSession(name, { kind: 'agent', name, c: r.c, role: r.role, model: r.model });
  withSession(name, () => notice([s(name, r.c, 'b'), s(g('sep'), 'faint'), s(r.role, 'muted')]));
}
/* bingo hands a teammate a task: it runs in the teammate's own session */
function hire(name, doing, script) {
  if (!S.views[name]) openAgent(name);
  return withSession(name, () => startTurn(doing, script, { from: 'bingo' }));
}
/* a task bingo handed out comes home as the teammate's words */
function reportBack(t) {
  const v = viewOf();
  if (!v || v.kind !== 'agent' || t.from !== 'bingo' || t.state !== 'done' || !t.reply) return;
  const text = t.reply.blocks.map(b => b.p).filter(Boolean).join(' ');
  withSession('main', () => report(v.name, text));
}
function report(name, text) {
  for (const t of S.log.filter(e => e.kind === 'turn'))
    for (const x of steps(t)) if (x.agent && x.agent.name === name && x.res === 'started') { x.res = 'reported'; x.lit = NOW(); }
  S.log.push({ kind: 'post', who: name, text, at: NOW() });
}

/* ----- what the teammates do with what bingo handed them ----- */
function scoutScript() {
  return [
    B.think(0, 500, '先 grep 调用点，再逐个读上下文。'),
    B.step(600, 'grep', 'drain\\('), B.ok(900, '4 hits in 3 files'),
    B.step(1000, 'read', 'crates/bingo-core/src/turn/loop.rs'), B.ok(1200, '312 lines'),
    B.step(1300, 'read', 'crates/bingo-core/src/session/mod.rs'), B.ok(1450, '188 lines'),
    B.step(1550, 'read', 'crates/bingo-rpc/src/steer.rs'), B.ok(1700, '96 lines'),
    B.reply(1900, [{ p: SCOUT_SAYS }]),
  ];
}
function reviewerScript() {
  return [
    B.think(0, 600, '先读改动，再跑一遍队列的测试。'),
    B.step(700, 'read', QF), B.ok(900, '214 lines'),
    B.step(1000, 'read', TF), B.ok(1150, '2 tests'),
    B.step(1300, 'bash', 'cargo test -p bingo-core queue'), B.ask(1400, 'bash', { why: 'checks the fix before I sign off' }),
    B.tail(1800, OUT_PASS[0]), B.tail(2400, OUT_PASS[2]),
    B.ok(3000, '12 passed', { out: OUT_PASS }),
    B.reply(3200, [{ p: REVIEWER_SAYS }]),
  ];
}
const AGENT_REPLY = {
  scout: '好，我顺便看一下 `rpc/steer.rs` 里有没有绕过 `drain()` 的地方。',
  reviewer: '收到，这一条我也写进 review 里。',
  ada: '好，我先把它写成一个会失败的测试。',
  bo: '记下了，发版前我再确认一遍。',
};
const agentReplyScript = v => [B.think(0, 500, '想想怎么接。'), B.reply(600, [{ p: AGENT_REPLY[v.name] || '好，记下了。' }])];

/* ----- who needs you: an ask waits in its own session; ctrl+g walks you there ----- */
const needsYou = id => { const o = sessionOf(id); return !!o && ASKING(o.dock); };
const needers = () => sessionIds().filter(id => id !== S.view && needsYou(id));
function nextNeed() {
  const ids = sessionIds(), at = ids.indexOf(S.view);
  const id = [...ids.slice(at + 1), ...ids.slice(0, at)].find(x => needsYou(x));
  if (id) return enterView(id);
  if (S.view !== 'main') return enterView('main');
  say('nobody is waiting on you');
}

/* ----- rooms: you and the teammates you seat; whoever you @ owes you an answer ----- */
const seated = v => v.members.filter(m => m !== 'you');
const roomHead = v => [s(v.name, v.c, 'b'), s(g('sep'), 'faint'), s(v.purpose, 'muted')];
function designRoom() {
  const id = '#design', at = NOW() - 9 * 60e3;
  const v = openSession(id, { kind: 'room', name: id, c: 'plan', purpose: 'decide how reset() treats queued lines',
    members: ['you', 'reviewer', 'scout', 'ada'], typing: null, closed: null, title: id });
  withSession(id, () => S.log.push(
    { kind: 'notice', segs: () => roomHead(v), at },
    { kind: 'post', who: 'you', text: '`reset()` 要不要清空排队的消息？@ada @scout 说说看', calls: ['ada', 'scout'], at: at + 1e3 },
    { kind: 'post', who: 'ada', text: '我倾向于 `reset()` 完全不碰队列：排队的消息是你说过的话，属于下一轮。', at: at + 40e3 },
    { kind: 'post', who: 'scout', text: '`drain()` 有 4 处调用，都在一轮开始时，在 `reset()` 之后。', at: at + 70e3 },
    { kind: 'post', who: 'scout', text: '所以 `reset()` 不清队列，`drain()` 也能拿到它们。', at: at + 75e3 },
  ));
  return v;
}
const ROOM_SAYS = {
  reviewer: '注释写的是「只重置 steer 槽」，和代码一致。建议补一句为什么排队的消息属于下一轮，不然下一个人还会把 `clear()` 加回去。',
  scout: '`reset()` 只有 `turn/loop.rs` 一处调用，在一轮开始时。',
  ada: '我先写个测试钉住它：steer 之后排队，`reset()` 之后队列还在。',
  bo: '我这边没意见；合进去之前我再跑一遍三个平台的 CI。',
};
/* a post: whoever you @ starts writing, then answers, one after another; @all calls on everyone seated */
function post(text) {
  const v = viewOf();
  const named = [...text.matchAll(/@([\w-]+)/g)].map(m => m[1]), here = seated(v);
  const calls = named.includes('all') ? here : [...new Set(named.filter(n => here.includes(n)))];
  const away = named.find(n => rosterOf(n) && !here.includes(n));
  S.hist.push(text);
  S.log.push({ kind: 'post', who: 'you', text, calls, at: NOW() });
  calls.forEach((n, i) => {
    later(700 + i * 2200, () => { if (canSpeak(v, n)) v.typing = { name: n, at: NOW() }; });
    later(2600 + i * 2200, () => answer(v, n));
  });
  if (away) say(`${away} is not in this room`, s(`/seat ${away}`, 'glow'), 'brings them in');
  else if (!calls.length) say('posted', '@ someone to ask for an answer');
}
/* only someone still seated in an open room speaks; leaving or closing drops what they owed */
const canSpeak = (v, name) => !v.closed && v.members.includes(name);
function answer(v, name) {
  if (v.typing && v.typing.name === name) v.typing = null;
  if (canSpeak(v, name)) S.log.push({ kind: 'post', who: name, text: ROOM_SAYS[name] || '好，我看看。', at: NOW() });
}
function seatAgent(args) {
  const v = viewOf(), name = args.trim(), free = ROSTER.map(r => r.name).filter(n => !v.members.includes(n));
  if (!name) { S.input = '/seat '; return say('seat who?', free.join(', ') || 'everyone is here'); }
  if (!rosterOf(name)) return say(`no agent called ${name}`);
  if (v.members.includes(name)) return say(`${name} is already here`);
  v.members.push(name);
  notice([s(name, crewInk(name), 'b'), s(' joined', 'muted')]);
}
function unseatAgent(args) {
  const v = viewOf(), name = args.trim();
  if (!name) { S.input = '/unseat '; return say('unseat who?', seated(v).join(', ')); }
  if (!seated(v).includes(name)) return say(`${name} is not in this room`);
  v.members = v.members.filter(m => m !== name);
  if (v.typing && v.typing.name === name) v.typing = null;
  notice([s(name, crewInk(name), 'b'), s(' left', 'muted')]);
}
function closeRoom(args) {
  const v = viewOf(), why = args.trim();
  if (!why) { S.input = '/close '; return say('say why the room closes'); }
  v.closed = { why, at: NOW() };
  v.typing = null;
  notice([s('closed', 'muted'), s(g('sep'), 'faint'), s(why, 'fg')]);
}
/* what @ offers: in a room, the people seated there; elsewhere, the crew and the files */
function mentionPool() {
  const v = viewOf(), crew = crewList();
  if (v && v.kind === 'room') return [...seated(v).map(n => ({ t: n, kind: 'agent', a: crew.find(a => a.name === n) })), { t: 'all', kind: 'all' }];
  return [...crew.map(a => ({ t: a.name, kind: 'agent', a })), ...FILES.map(f => ({ t: f.p, kind: 'file', f }))];
}

/* ----- the crew sheet: teammates by who needs you, then rooms; ⏎ walks in ----- */
const crewItems = () => [...crewList().map(a => ({ id: a.name, a })),
  ...Object.values(S.views).filter(v => v.kind === 'room').map(v => ({ id: v.id, v }))];
function crewRow(x, sel, now, W) {
  const a = x.a, v = x.v, ink = a ? a.c : v.c;
  const r = a ? [a.state === 'working' ? s(`working ${fmtDur(now - a.at)}`, 'muted') : needWord(a.state, 'idle')]
    : [s(v.closed ? 'closed' : `${seated(v).length} seated`, 'dim')];
  const what = a ? s(a.doing || a.role, a.doing ? 'muted' : 'dim') : s(v.purpose, 'dim');
  return { segs: [stateMark(a ? a.state : v.typing ? 'working' : 'idle', now, ink), s('  '), ...padTo([s(x.id, ink, sel ? 'b' : '')], 11),
    ...fitSegs([what], W - TX - RM - 14 - segsW(r) - 3)], r };
}
SHEETS.crew = {
  head: () => {
    const c = crewList(), n = c.filter(a => a.state === 'asking').length, w = c.filter(a => a.state === 'working').length;
    const busy = [...(n ? [`${n} needs you`] : []), ...(w ? [`${w} working`] : [])];
    return { l: [s('Crew', 'fg', 'b')], r: [s(busy.join(g('sep')) || 'all idle', 'dim')] };
  },
  rows: (d, now, W) => crewItems().map((x, i) => crewRow(x, i === d.sel, now, W)),
  key: (d, k) => {
    const x = crewItems()[d.sel];
    if (k !== 'tab' || !x || !x.a) return false;
    closeDock(); S.input = `@${x.id} `;
    return true;
  },
  enter: d => {
    const x = crewItems()[d.sel];
    if (!x) return;
    closeDock();
    walkTo(x.id);
  },
  keys: d => [['↑↓', 'who'], ['⏎', 'open'], ...((crewItems()[d.sel] || {}).a ? [['tab', 'mention']] : []), ['esc', 'close']],
};
