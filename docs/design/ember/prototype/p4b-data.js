/* ---------- the session's world: modes, people, files, models, and the stories the scenes tell ---------- */
const MODES = [
  { n: 'default', c: 'muted', d: 'trusted read-only tools run; everything else asks' },
  { n: 'accept edits', c: 'auto', d: 'edits inside the working directories run without a prompt' },
  { n: 'plan', c: 'plan', d: 'nothing that is not read-only runs at all' },
  { n: 'bypass', c: 'bad', d: 'everything runs except what only a person may decide' },
];
const modeIs = n => MODES[S.mode].n === n;
const modeInk = m => m.c === 'muted' ? 'fg' : m.c;

/* the wordmark: five letters on an eight-row grid, drawn two rows to a cell */
const MARK_SRC = {
  b: ['#.....', '#.....', '######', '#....#', '#....#', '######', '......', '......'],
  i: ['#', '.', '#', '#', '#', '#', '.', '.'],
  n: ['......', '......', '######', '#....#', '#....#', '#....#', '......', '......'],
  g: ['......', '......', '######', '#....#', '#....#', '######', '.....#', '######'],
  o: ['......', '......', '######', '#....#', '#....#', '######', '......', '......'],
};
const MARK = MARK_SRC.b.map((_, y) => [...'bingo'].map(c => MARK_SRC[c][y]).join('.').split('').map(c => c === '#'));
const MARK_ASCII = markCells().map(r => r.replace(/█/g, '#').replace(/▀/g, '"').replace(/▄/g, '_'));

const HERE = { dir: '~/bingo-improve', branch: 'dev', changed: 3 };
/* the way back names this session, so it is this one that returns, not the newest in the directory */
const SID = 'ses_01JB7Y2K9QH4X3V8M6N5P2R1TC', RESUME = `bingo --resume ${SID}`;
const RECENTS = [
  { t: 'steer 吞掉排队消息', when: '12m' },
  { t: '给 total 加标签', when: '2h' },
  { t: 'bingo-site 文档页', when: 'yesterday' },
];
const SESSIONS = [
  { t: 'bingo-site 文档页', dir: '~/bingo-site', when: '3m', st: 'working' },
  { t: '飞书卡片按钮回调', dir: '~/bingo-improve', when: '40m', st: 'asking' },
  { t: 'steer 吞掉排队消息', dir: '~/bingo-improve', when: '12m' },
  { t: '给 total 加标签', dir: '~/total', when: '2h' },
  { t: 'release v0.6.3', dir: '~/bingo-improve', when: 'Sep 15' },
  { t: 'axum 路由迁移', dir: '~/api', when: 'Sep 12' },
  { t: 'blender 场景灯光', dir: '~/blender_demo', when: 'Sep 9' },
  { t: 'windows CI 超时', dir: '~/bingo-improve', when: 'Sep 3' },
  { t: 'cargo deny 报 rustls', dir: '~/bingo-improve', when: 'Aug 30' },
];

/* the slash palette: commands first, then skills; the key shows when one exists */
const COMMANDS = [
  { n: 'model', d: 'switch model and effort', k: 'alt+m' },
  { n: 'think', d: 'how hard the model thinks', k: 'alt+t' },
  { n: 'compact', d: 'summarize the conversation to free context' },
  { n: 'status', d: 'model, mode, context, cost, plugins' },
  { n: 'resume', d: 'open an earlier session', k: 'ctrl+r' },
  { n: 'rewind', d: 'go back to before a prompt', k: 'esc esc' },
  { n: 'permission', d: 'mode and rules for what runs without asking', k: '⇧tab' },
  { n: 'agents', d: 'the agents working with you' },
  { n: 'tasks', d: 'what the team is working on', k: 'ctrl+t' },
  { n: 'room', d: 'talk with agents in a room' },
  { n: 'team', d: 'roles and who holds them' },
  { n: 'seat', d: 'bring an agent into this room', room: true },
  { n: 'unseat', d: 'ask an agent to leave this room', room: true },
  { n: 'close', d: 'close this room and say why', room: true },
  { n: 'schedule', d: 'prompts that run every, daily at, or once at' },
  { n: 'wake', d: 'come back to this session later' },
  { n: 'plugins', d: 'switch plugins, review trust' },
  { n: 'mcp', d: 'MCP servers and their tools' },
  { n: 'memory', d: 'read and forget memories and instructions' },
  { n: 'experience', d: 'what bingo learned in this repo' },
  { n: 'rename', d: 'rename this session' },
  { n: 'provider', d: 'add or switch model providers' },
  { n: 'login', d: 'sign in to a provider' },
  { n: 'logout', d: 'sign out of a provider' },
  { n: 'clear', d: 'start a fresh session' },
  { n: 'help', d: 'every key and command', k: '?' },
  { n: 'review', d: 'review the diff on this branch', kind: 'skill', fam: 'project' },
  { n: 'release', d: 'gates, tag, notes, publish', kind: 'skill', fam: 'project' },
  { n: 'imagegen', d: 'generate an image through your endpoint', kind: 'skill', fam: 'yours' },
  { n: 'tdd', d: 'red, green, refactor on one behaviour', kind: 'skill', fam: 'built in' },
  { n: 'explain', d: 'explain code at the depth you ask for', kind: 'skill', fam: 'built in' },
  { n: 'guide', d: 'how bingo works, a page at a time', kind: 'skill', fam: 'guide' },
];

const FILES = [
  { p: 'crates/bingo-core/src/session/queue.rs', st: 'modified' },
  { p: 'crates/bingo-core/tests/queue_steer.rs', st: 'new' },
  { p: 'docs/adr/0097-ember-tui.md', st: 'new' },
  { p: 'crates/bingo-core/src/session/mod.rs' },
  { p: 'crates/bingo-core/src/turn/loop.rs' },
  { p: 'crates/bingo-sdk/src/event/mod.rs' },
  { p: 'crates/bingo-surface-tui/src/dock/composer.rs' },
  { p: 'crates/bingo-surface-tui/src/stream/steps.rs' },
  { p: 'ARCHITECTURE.md' },
  { p: 'Cargo.toml' },
];

const MODELS = [
  { id: 'opus-5.5', prov: 'anthropic', ctx: '1M', img: true, effs: ['low', 'medium', 'high', 'xhigh', 'max'] },
  { id: 'sonnet-5.5', prov: 'anthropic', ctx: '1M', img: true, effs: ['off', 'low', 'medium', 'high', 'max'] },
  { id: 'haiku-5.5', prov: 'anthropic', ctx: '200k', img: true, effs: ['off', 'low', 'medium', 'high'] },
  { id: 'gpt-6-astra', prov: 'openai', ctx: '400k', img: true, effs: ['minimal', 'low', 'medium', 'high', 'xhigh'] },
  { id: 'gpt-6-mini', prov: 'openai', ctx: '400k', img: false, effs: ['minimal', 'low', 'medium', 'high'] },
  { id: 'qwen4-coder', prov: 'local', ctx: '128k', img: false, effs: ['off'] },
];
const EFFORT = { off: 'answers without thinking', minimal: 'barely thinks', low: 'a short think on hard steps', medium: 'thinks on most steps',
  high: 'thinks before most answers', xhigh: 'thinks long on hard problems', max: 'thinks as long as it needs' };
const modelOf = (id = S.model.id) => MODELS.find(m => m.id === id) || MODELS[0];
const nearEffort = (m, e) => m.effs.includes(e) ? e : m.effs[Math.min(m.effs.length - 1, Math.max(0, m.effs.indexOf('high')))] || m.effs[0];

const PLUGINS = [
  { n: 'tool-fs', d: 'read, edit and write files', on: true, kind: 'built in' },
  { n: 'tool-bash', d: 'run shell commands', on: true, kind: 'built in' },
  { n: 'tool-web', d: 'fetch pages and search the web', on: true, kind: 'built in' },
  { n: 'agents', d: 'hire agents and give them work', on: true, kind: 'built in' },
  { n: 'rooms', d: 'rooms where agents and you talk', on: true, kind: 'built in' },
  { n: 'schedule', d: 'prompts that run on a clock', on: true, kind: 'built in' },
  { n: 'experience', d: 'learns from your corrections', on: false, kind: 'built in' },
  { n: 'feishu', d: 'talk to this session from Feishu', on: true, kind: 'channel' },
  { n: 'github', d: 'issues, pull requests and checks', on: true, kind: 'mcp', err: 'token expired', fix: '⏎ sign in' },
  { n: 'linear', d: 'issues and cycles', on: false, kind: 'mcp' },
];

const TASKS = [
  { id: 5, t: '审一遍 queue 的改动', st: 'pending', who: 'reviewer' },
  { id: 4, t: '补 steer 之后排队的回归测试', st: 'in_progress', who: 'bingo' },
  { id: 3, t: '找出所有调用 drain() 的地方', st: 'in_progress', who: 'scout' },
  { id: 2, t: '修 reset() 清空队列', st: 'completed', who: 'bingo' },
  { id: 1, t: '复现消息被吞', st: 'completed', who: 'you' },
];
const SCHEDS = [
  { when: 'every 30m', what: '看一眼 CI，挂了就告诉我', next: 'in 12m', on: true },
  { when: 'daily 09:00', what: '总结昨天合进 dev 的提交', next: 'tomorrow 09:00', on: true },
  { when: 'once Fri 18:00', what: '提醒我发 v0.6.4', next: 'Fri 18:00', on: false },
];
/* what the model reads every turn: instructions you wrote, memories it kept */
const MEMORY = [
  { kind: 'instructions', t: 'AGENTS.md', d: 'this project · 64 lines' },
  { kind: 'instructions', t: '~/.bingo/AGENTS.md', d: 'every project · 12 lines' },
  { kind: 'memory', t: '在 dev 上开发，main 是发布线', d: 'Sep 2' },
  { kind: 'memory', t: '测试别钉墙上的时钟，CI 更慢', d: 'Sep 11' },
  { kind: 'memory', t: '提交信息用英文，主题不超过 60 个字符', d: 'Aug 30' },
];

/* ----- beats the scripts share ----- */
B.end = (at, state = 'done') => ({ at, f: (t, T) => endTurn(t, T, state) });
const counts = diff => ({ add: diff.filter(d => d.s === '+').length, del: diff.filter(d => d.s === '-').length });
const shift = (beats, ms) => beats.map(b => Object.assign({}, b, { at: b.at + ms }));
function editBeats(at, tool, file, diff, o = {}) {
  const out = [B.step(at, tool, file, { diff, id: o.id })];
  if (o.ask) out.push(B.ask(at + 120, 'edit', { id: o.id }));
  out.push(B.ok(at + (o.ms || 300), '', Object.assign(counts(diff), o.id ? { id: o.id } : {})));
  return out;
}
/* what the rest of a turn does once you have answered: the script after now is replaced */
function branch(t, beats) {
  const T0 = turnTime(t, NOW());
  t.script = t.script.slice(0, t.si).concat(beats.map(b => Object.assign({}, b, { at: T0 + b.at })));
}
/* steering lands when the next step starts */
function landSteers(t, T) {
  const st = S.queued.filter(q => q.steer);
  if (!st.length) return;
  S.queued = S.queued.filter(q => !q.steer);
  for (const q of st) t.items.push({ k: 'you', at: T, text: q.text, note: 'steered' });
}

/* ----- the story: a steer that swallows a queued line, found, fixed, tested ----- */
const QF = 'crates/bingo-core/src/session/queue.rs';
const TF = 'crates/bingo-core/tests/queue_steer.rs';
const T1P = '看一下 session/queue.rs 里的 steer 逻辑，为什么排队的消息有时候会被吞掉';
const T2P = '修好它，加一个回归测试';
const DIFF_RESET = [
  { s: ' ', n: 86, t: '    /// Called when a turn starts.' },
  { s: ' ', n: 87, t: '    pub fn reset(&mut self) {' },
  { s: '-', n: 88, t: '        self.queue.clear();' },
  { s: '+', n: 88, t: '        // queued lines belong to the next turn; only the steer slot resets' },
  { s: ' ', n: 89, t: '        self.steer = None;' },
  { s: ' ', n: 90, t: '    }' },
];
/* what the lookups returned, as their owner mod draws it in a well */
const QF_READ = { start: 1, of: 214, lines: [
  'use std::collections::VecDeque;', '', 'use bingo_sdk::Message;', '',
  '/// What you type while a turn runs: one steer slot, then the queue.', '#[derive(Debug, Default)]',
  'pub struct Queue {', '    steer: Option<Message>,', '    queue: VecDeque<Message>,', '}', '', 'impl Queue {'] };
const LOOP_READ = { start: 1, of: 486, lines: [
  'use bingo_sdk::{Event, Message};', 'use tokio::sync::mpsc;', '', 'use crate::session::Queue;', '',
  '/// One turn: ask the model, run its steps, until it answers without one.', 'pub struct TurnLoop {',
  '    queue: Queue,', '    events: mpsc::Sender<Event>,', '}', '', 'impl TurnLoop {'] };
const HITS = [
  `${QF}:41: pub fn drain(&mut self) -> Option<Message> {`, `${QF}:87: pub fn reset(&mut self) {`,
  `${QF}:118: fn reset_clears_steer() {`, `${QF}:126: fn drain_takes_steer_first() {`,
  'crates/bingo-core/src/turn/loop.rs:58: self.queue.reset();',
  'crates/bingo-core/src/turn/loop.rs:131: while let Some(m) = self.queue.drain() {',
  'crates/bingo-core/src/turn/loop.rs:204: self.queue.reset();',
  'crates/bingo-core/src/session/mod.rs:77: self.queue.drain().into_iter().collect()',
  'crates/bingo-core/src/session/mod.rs:92: self.turn.reset();'];
const TEST_FILES = ['compact', 'journal', 'permissions', 'replay', 'session', 'turn_loop'].map(f => `crates/bingo-core/tests/${f}.rs`);
/* one test per case you chose; the first case carries the slip the test run catches */
const CASES = {
  'steer 之后排队': ['steer_then_queue_keeps_both', [
    'let mut q = Queue::default();', 'q.steer(Message::text("first"));', 'q.push(Message::user("second"));',
    'assert_eq!(q.drain().map(|m| m.text()), Some("first".into()));', 'q.reset();',
    'assert_eq!(q.drain().map(|m| m.text()), Some("second".into()));']],
  '连续两次 steer': ['a_second_steer_replaces_the_first', [
    'let mut q = Queue::default();', 'q.steer(Message::text("old"));', 'q.steer(Message::text("new"));',
    'assert_eq!(q.drain().map(|m| m.text()), Some("new".into()));']],
  '工具调用中途 steer': ['a_steer_waits_for_the_step', [
    'let mut q = Queue::default();', 'q.begin_step();', 'q.steer(Message::text("wait"));', 'assert!(q.drain().is_none());',
    'q.end_step();', 'assert_eq!(q.drain().map(|m| m.text()), Some("wait".into()));']],
};
const DEFAULT_CASES = ['steer 之后排队', '连续两次 steer'];
const UNIT_AT = 215;
function testLines(cases, unit) {
  const pad = unit ? '    ' : '';
  const tests = cases.flatMap((c, i) => [...(i ? [''] : []), `${pad}#[test]`, `${pad}fn ${CASES[c][0]}() {`, ...CASES[c][1].map(l => `${pad}    ${l}`), `${pad}}`]);
  return unit ? ['', '#[cfg(test)]', 'mod steer_tests {', '    use super::*;', '    use bingo_sdk::Message;', '', ...tests, '}']
    : ['use bingo_core::session::Queue;', 'use bingo_sdk::Message;', '', ...tests];
}
function outFail(file, n, src, unit) {
  const col = src.indexOf('user') + 1, pad = ' '.repeat(String(n).length);
  return ['   Compiling bingo-core v0.6.3 (crates/bingo-core)',
    'error[E0599]: no function or associated item named `user` found for enum `Message`',
    `${pad}--> ${file}:${n}:${col}`, `${pad} |`, `${n} | ${src}`,
    `${pad} | ${' '.repeat(col - 1)}^^^^ function or associated item not found in \`Message\``,
    `error: could not compile \`bingo-core\` (${unit ? 'lib test' : 'test "queue_steer"'}) due to 1 previous error`];
}
function outPass(names, unit) {
  const n = 10 + names.length;
  return ['   Compiling bingo-core v0.6.3 (crates/bingo-core)', '    Finished `test` profile in 6.42s', `running ${n} tests`,
    'test session::queue::tests::drain_takes_steer_first ... ok', 'test session::queue::tests::reset_clears_steer ... ok',
    ...names.map(x => `test ${unit ? 'session::queue::steer_tests::' : ''}${x} ... ok`), '…',
    `test result: ok. ${n} passed; 0 failed; 0 ignored; finished in 0.04s`];
}
const fixDiff = (src, i, n0) => [
  { s: ' ', n: n0 + i - 1, t: src[i - 1] }, { s: '-', n: n0 + i, t: src[i] },
  { s: '+', n: n0 + i, t: src[i].replace('Message::user', 'Message::text') }, { s: ' ', n: n0 + i + 1, t: src[i + 1] }];
const OUT_PASS = outPass(DEFAULT_CASES.map(c => CASES[c][0]), false);
const REPLY1 = [
  { p: '问题不在 `drain()`，在 `reset()`：每一轮开始时，它把 `queue` 整个清空了。' },
  { code: 'pub fn reset(&mut self) {\n    self.queue.clear();   // 排着的消息在这里丢了\n    self.steer = None;\n}', lang: 'rust' },
  { p: '你先插了一句（steer），又排了一句（queue）：steer 让这一轮提前收尾，下一轮开始时 `reset()` 把还没发出去的那句一起清掉了。所以只有「插话之后紧跟着排队」才会丢。' },
];
function reply2(unit, cases, slipped) {
  const where = unit ? '`session/queue.rs` 底部的 `steer_tests`' : '`tests/queue_steer.rs`';
  return [
    { p: '修好了。`reset()` 只重置 steer，排队的消息留给下一轮。' },
    { ul: ['`session/queue.rs`：`reset()` 不再清空队列', `${where}：${cases.length} 个回归测试，覆盖${/^[\x21-\x7e]/.test(cases[0]) ? ' ' : ''}${cases.join('、')}`,
      ...(slipped ? ['第一次跑测试没编过，是我写错了构造函数，已经改成 `Message::text`'] : [])] },
    { p: `\`cargo test -p bingo-core queue\`：${10 + cases.length} 个全部通过。` },
  ];
}

function t1Script() {
  return [
    B.think(0, 1400, 'steer 和 queue 是两条路：steer 在下一步落地，queue 等这一轮结束。吞消息多半出在两条路交汇的地方。'),
    B.narr(1450, '先看 queue.rs，再看谁在调用 drain() 和 reset()。'),
    B.step(1500, 'read', QF), B.ok(1750, '214 lines', { code: QF_READ }),
    B.step(1850, 'grep', 'drain|reset'), B.ok(2050, '9 hits in 3 files', { out: HITS }),
    B.step(2150, 'read', 'crates/bingo-core/src/turn/loop.rs'), B.ok(2450, '486 lines', { code: LOOP_READ }),
    B.reply(2600, REPLY1),
  ];
}
const T1 = () => simulate(T1P, t1Script());

const QS = [
  { header: '位置', question: '回归测试放在哪？', options: [
    { label: '集成测试', description: '新建 tests/queue_steer.rs，只走公开 API',
      preview: 'tests/queue_steer.rs\n\n#[test]\nfn steer_then_queue_keeps_both() {\n    let mut q = Queue::default();\n    q.steer(Message::text("first"));\n    q.push(Message::text("second"));\n    …\n}' },
    { label: '单元测试', description: '写在 queue.rs 底部，能看到私有字段',
      preview: 'src/session/queue.rs\n\n#[cfg(test)]\nmod steer_tests {\n    use super::*;\n\n    #[test]\n    fn steer_then_queue_keeps_both() {\n        …\n    }\n}' },
  ] },
  { header: '覆盖', question: '要覆盖哪些情况？', multi: true, options: [
    { label: 'steer 之后排队', description: '这次丢消息的情况' },
    { label: '连续两次 steer', description: '后一条替换前一条' },
    { label: '工具调用中途 steer', description: '等这一步做完才落地' },
  ] },
];

function mainScript(o = {}) {
  const b = [
    B.think(0, 700, '原因上一轮找到了：reset() 每轮开始时清空了队列。只重置 steer 就行，再写一个测试复现「steer 之后排队的那条被吞」。'),
    B.narr(800, '先改 reset()，再补一个回归测试。'),
    B.step(900, 'read', QF), B.ok(1100, '214 lines', { code: QF_READ }),
    B.step(1200, 'glob', 'crates/bingo-core/tests/*.rs'), B.ok(1350, '6 files', { out: TEST_FILES }),
    ...editBeats(1500, 'edit', QF, DIFF_RESET, { ask: o.askEdit }),
  ];
  if (o.question) return b.concat(B.step(2000, 'ask', '回归测试放在哪？'), B.ask(2060, 'question', { qs: QS, then: ans => restBeats(o, ans) }));
  return b.concat(shift(restBeats(o), 2050));
}
/* everything after the question: where the test goes and what it covers follow your answer */
function restBeats(o = {}, ans) {
  const unit = !!ans && ans[0] === '单元测试';
  const picked = ans && Array.isArray(ans[1]) ? ans[1].filter(c => CASES[c]) : [];
  const cases = picked.length ? picked : DEFAULT_CASES;
  const file = unit ? QF : TF, n0 = unit ? UNIT_AT : 1, src = testLines(cases, unit);
  const slip = src.findIndex(l => l.includes('Message::user')), names = cases.map(c => CASES[c][0]);
  const b = editBeats(0, unit ? 'edit' : 'write', file, src.map((t, i) => ({ s: '+', n: n0 + i, t })), { ms: 350 });
  let T = 550;
  const run = () => {
    b.push(B.step(T, 'bash', 'cargo test -p bingo-core queue'));
    if (o.askBash) b.push(B.ask(T + 100, 'bash', { why: 'runs the queue tests against the fix' }));
  };
  run();
  if (slip >= 0) {
    const fail = outFail(file, n0 + slip, src[slip], unit);
    b.push(B.tail(T + 500, fail[0]), B.tail(T + 1000, fail[1]), B.tail(T + 1150, fail[2]), B.fail(T + 1500, 'exit 101', { out: fail }));
    T += 1700;
    if (o.failStop) return b.concat(B.reply(T, [{ p: '测试没编过：`Message` 没有 `user` 这个构造函数。要我把测试改成 `Message::text` 再跑一次吗？' }]));
    b.push(B.narr(T, '测试里的构造函数写错了，应该是 Message::text。'), ...editBeats(T + 300, 'edit', file, fixDiff(src, slip, n0)));
    T += 800;
    run();
  }
  const pass = outPass(names, unit);
  b.push(B.tail(T + 400, pass[0]), B.tail(T + 1000, pass[2]), B.tail(T + 1500, pass[5]), B.ok(T + 2100, `${10 + names.length} passed`, { out: pass }));
  return b.concat(B.reply(T + 2300, reply2(unit, cases, slip >= 0)));
}
const T2done = () => simulate(T2P, mainScript());

/* plan mode: the same request, answered with a plan and nothing that writes */
function planScript() {
  return [
    B.think(0, 800, 'plan 模式只读。先看清楚要改哪里，给出计划。'),
    B.step(900, 'read', QF), B.ok(1100, '214 lines', { code: QF_READ }),
    B.step(1200, 'glob', 'crates/bingo-core/tests/*.rs'), B.ok(1350, '6 files', { out: TEST_FILES }),
    B.reply(1500, [{ p: '现在是 plan 模式，我只读不改。计划：' },
      { ol: ['`reset()` 里去掉 `self.queue.clear()`，只重置 steer', '新建 `tests/queue_steer.rs`：steer 之后排队、连续两次 steer', '`cargo test -p bingo-core queue` 验证'] },
      { p: '按 `shift+tab` 切到 default 或 accept edits，我就开始做。' }]),
  ];
}
function testScript() {
  return [B.step(0, 'bash', 'cargo test -p bingo-core queue'), B.ask(100, 'bash', { why: 'runs the queue tests' }),
    B.tail(500, OUT_PASS[0]), B.tail(1100, OUT_PASS[2]), B.ok(1700, '12 passed', { out: OUT_PASS }),
    B.reply(1900, [{ p: '`cargo test -p bingo-core queue`：12 个全部通过。' }])];
}
/* a caution ask: force-pushing drops commits only the remote has */
const LOST = ['4e1a9c2 wip: queue experiments', 'b81f0d3 wip: try a ring buffer'];
function forceScript() {
  return [
    B.think(0, 600, '远端 dev 上有两个本地没有的提交。强推会把它们从远端拿掉，这要问一下。'),
    B.step(700, 'bash', 'git log --oneline origin/dev -3'), B.tail(900, LOST[1]), B.tail(950, LOST[0]),
    B.ok(1100, '3 lines', { out: [LOST[1], LOST[0], '7c3e2a1 fix(core): keep queued lines across reset'] }),
    B.step(1300, 'bash', 'git push --force origin dev'),
    B.ask(1400, 'caution', { lost: LOST, caution: 'rewrites remote history' }),
    B.tail(1700, 'To github.com:bingo-inc/bingo.git'), B.tail(2000, ' + b81f0d3...7c3e2a1 dev -> dev (forced update)'),
    B.ok(2300, 'forced update'),
    B.reply(2500, [{ p: '推上去了。远端 `dev` 现在和你本地一致，`4e1a9c2` 和 `b81f0d3` 已经不在远端了，本地 reflog 里还找得回来。' }]),
  ];
}
/* two agents at once: each works in its own session and reports back here */
function crewScript() {
  return [
    B.think(0, 600, '两件事可以并行：scout 去找调用点，reviewer 看 diff。'),
    B.step(700, 'agent', 'find every caller of drain()', { agent: { name: 'scout', c: crewInk('scout') } }),
    B.call(720, () => hire('scout', 'find every caller of drain()', scoutScript())), B.ok(900, 'started'),
    B.step(1000, 'agent', 'review the queue.rs diff', { agent: { name: 'reviewer', c: crewInk('reviewer') } }),
    B.call(1020, () => hire('reviewer', 'review the queue.rs diff', reviewerScript())), B.ok(1200, 'started'),
    B.reply(1400, [{ p: '派出去了：**scout** 找所有调用 `drain()` 的地方，**reviewer** 看 `queue.rs` 的改动。它们做完会在这里说；要你点头的时候，页脚会告诉你，`ctrl+g` 过去。' }]),
  ];
}
const SCOUT_SAYS = '找到 4 处调用 `drain()`：`turn/loop.rs` 两处、`session/mod.rs` 一处、`rpc/steer.rs` 一处。都只在一轮开始时调用，和 `reset()` 的顺序一致，改动不影响它们。';
const REVIEWER_SAYS = '`reset()` 的改法没问题。建议在 `a_second_steer_replaces_the_first` 里再断言一次队列是空的，不然以后有人把 steer 改成追加，这个测试还是会过。';
function imageScript() {
  return [
    B.think(0, 900, '截图是 cargo 的报错：借用活得比被借的值还久。'),
    B.reply(1000, [{ p: '截图里是一个借用错误：`drain()` 返回的引用，活得比 `self.queue` 还久。' },
      { p: '把返回值改成拥有所有权的 `Message` 就行，`pop_front()` 本来就把所有权交给你；或者在调用处先 `clone()`。' }]),
  ];
}
function wakeScript() {
  const out = ['✓ fmt', '✓ clippy', '✓ test (ubuntu)', '✓ test (macos)', '* test (windows)   7/12'];
  return [
    B.step(0, 'bash', 'gh run watch --exit-status'), ...out.map((l, i) => B.tail(250 + i * 220, l)),
    B.ok(1500, 'still running', { out }),
    B.step(1600, 'wake', 'in 5m'), B.call(1650, () => { S.wake = { at: NOW() + 300000 }; }), B.ok(1700, 'set'),
    B.reply(1800, [{ p: 'Linux 和 macOS 都过了，Windows 还在跑（7/12）。我 5 分钟后醒来再看一眼，过了就告诉你。' }]),
  ];
}
function retryScript() {
  return [
    B.call(0, (t, T) => { t.retry = { at: T, until: T + 3000, n: 1, of: 10, why: 'rate limited' }; }),
    B.call(3000, (t, T) => { t.retry = { at: T, until: T + 4000, n: 2, of: 10, why: 'rate limited' }; }),
    B.call(7000, t => { t.retry = null; }),
    ...shift(mainScript({}), 7000),
  ];
}
function signedOutScript() {
  return [B.call(500, (t, T) => {
    t.error = { head: 'signed out of anthropic', msg: 'the token expired', code: 'AUTH_REQUIRED',
      hint: () => [s('/login', 'glow'), s(' signs you in again', 'muted'), s(g('sep'), 'faint'), s('your message is kept', 'muted')] };
    endTurn(t, T, 'failed');
    S.resend = t.prompt;
  })];
}
function bgScript() {
  const lines = ['   Compiling bingo-sdk v0.6.3', '   Compiling bingo-core v0.6.3', '   Compiling bingo-surface-tui v0.6.3', 'running 412 tests'];
  return [
    B.narr(0, '跑一遍全量测试，要一会儿。'),
    B.step(200, 'bash', 'cargo test --workspace', { id: 'w', prog(now) { return `${Math.min(412, Math.floor(Math.max(0, now - this.at - 2400) / 1000 * 31))}/412 tests`; } }),
    ...lines.map((l, i) => B.tail(600 + i * 700, l, 'w')),
    B.ok(15000, '412 passed', { id: 'w' }),
    B.reply(15200, [{ p: '全部 412 个测试都过了。' }]),
  ];
}
function shellScript(cmd) {
  const out = SHELL_OUT[cmd] || ['(this prototype only knows a few commands)'];
  return [B.step(0, 'bash', cmd), ...out.map((l, i) => B.tail(80 + i * 40, l)), B.ok(200 + out.length * 40, plural(out.length, 'line'), { out }), B.end(260 + out.length * 40)];
}
const SHELL_OUT = {
  'git status --short': [' M crates/bingo-core/src/session/queue.rs', '?? crates/bingo-core/tests/queue_steer.rs', '?? docs/adr/0097-ember-tui.md'],
  'git log --oneline -3': ['7c3e2a1 fix(core): keep queued lines across reset', 'ea337d14 revert(tui): restore M185 marks', 'bed9c196 fix(tui): bring the filled dot back'],
  ls: ['ARCHITECTURE.md  Cargo.lock  Cargo.toml  crates  docs  scripts'],
};
function skillScript(name) {
  return [
    B.step(0, 'skill', name), B.ok(200, 'loaded'),
    B.step(300, 'bash', 'git diff dev...HEAD --stat'), B.ok(600, '3 files'),
    B.step(700, 'read', QF), B.ok(900, '214 lines'),
    B.reply(1100, [{ p: '看了这次的改动，两点：' },
      { ol: ['`reset()` 的注释写清楚了为什么不清队列，好。', '`queue_steer.rs` 两个测试都只断言了第一条，建议把 `drain()` 之后的队列也比一遍。'] }]),
  ];
}
function echoScript(text) {
  return [B.think(0, 500, '原型里的回答是预先写好的。'),
    B.reply(600, [{ p: `收到「${text.length > 24 ? text.slice(0, 24) + '…' : text}」。这是原型，回答是预先写好的；试试「${T2P}」，看完整的一轮：读文件、改代码、跑测试、失败、修好。` }])];
}
function pastScript(title) {
  return [B.think(0, 600, '这是另一个会话。'), B.step(700, 'read', 'README.md'), B.ok(900, '88 lines'),
    B.reply(1000, [{ p: `这是「${title}」会话的最后一轮。往下接着说就行。` }])];
}

/* every mark in one turn: each step's mark, verb and outcome come from the mod that owns the tool */
const ICONS_P = '把 drain 的坑查清楚、修好，开个 issue 记下来';
function iconsScript() {
  const scout = { agent: { name: 'scout', c: crewInk('scout') } };
  return [
    B.step(100, 'read', QF), B.ok(200, '214 lines'),
    B.step(300, 'glob', 'crates/bingo-core/tests/*.rs'), B.ok(400, '6 files'),
    B.step(500, 'grep', 'drain\\('), B.ok(600, '4 hits in 3 files'),
    B.step(700, 'search', 'VecDeque drain keeps capacity'), B.ok(800, '8 results'),
    B.step(900, 'fetch', 'doc.rust-lang.org/std/collections/struct.VecDeque.html'), B.ok(1000, '31 KB'),
    B.step(1100, 'look', 'screenshots/lost-line.png'), B.ok(1200, '1280×720'),
    B.step(1300, 'skill', 'review'), B.ok(1400, 'loaded'),
    B.step(1500, 'agent', 'find every caller of drain()', scout), B.ok(1600, 'reported'),
    ...editBeats(1700, 'edit', QF, DIFF_RESET),
    ...editBeats(2100, 'write', TF, testLines(DEFAULT_CASES, false).map((t, i) => ({ s: '+', n: i + 1, t }))),
    B.step(2500, 'bash', 'cargo test -p bingo-core queue'), B.ok(2700, '12 passed', { out: OUT_PASS }),
    B.step(2800, 'todo', 'note the fix in CHANGELOG'), B.ok(2900, 'added'),
    B.step(3000, 'ask', 'issue 写英文吗？'), B.ok(3100, 'English', { qa: [['issue 写英文吗？', 'English']] }),
    B.step(3200, 'mcp', 'create issue · steer drops queued lines', { via: 'linear' }), B.ok(3300, 'LIN-212'),
    B.step(3400, 'query', 'select count(*) from turns where lost'), B.ok(3500, '1 row'),
    B.step(3600, 'deploy', 'staging'), B.ok(3700, 'live'),
    B.step(3800, 'wake', 'in 30m · check CI on dev'), B.ok(3900, 'set'),
    B.reply(4000, [{ p: '查清楚了：`reset()` 每轮开头把队列清空了。改好了，12 个测试都过；issue 是 `LIN-212`，30 分钟后我看一眼 CI。' }]),
  ];
}

/* what bingo does with what you typed */
function scriptFor(text) {
  const t = text.trim(), sk = t.match(/^\/(\S+)/);
  if (S.signedOut) return signedOutScript();
  if (t.startsWith('!')) return { script: shellScript(t.slice(1).trim()), o: { shell: true } };
  const v = viewOf();
  if (v && v.kind === 'agent') return agentReplyScript(v);
  if (sk) return skillScript(sk[1]);
  if (modeIs('plan') && /修|改|fix/i.test(t)) return planScript();
  if (/为什么|吞/.test(t)) return t1Script();
  if (/修好|回归|fix/i.test(t)) return mainScript({ askEdit: true, askBash: true });
  if (/推|push/i.test(t)) return forceScript();
  if (/测试|test/i.test(t)) return testScript();
  if (/scout|reviewer|agent|并行/i.test(t)) return crewScript();
  if (/截图|图片|image/i.test(t)) return imageScript();
  if (/CI|ci/.test(t)) return wakeScript();
  return echoScript(t);
}

/* ----- asks: what the step wants, a preview, the answers widest first ----- */
const pathTail = (p, n = 2) => p.split('/').slice(-n).join('/');
const ASK = {
  edit: x => {
    const c = counts(x.diff), dir = x.arg.split('/').slice(0, -1).join('/') + '/';
    return {
      title: `Edit ${pathTail(x.arg)}?`, right: [s(`+${c.add}`, 'good'), s(` ${g('minus')}${c.del}`, 'bad')],
      body: diffBody(x.diff), rule: `edit ${dir}`,
      options: [
        { label: 'Yes', a: 'once' },
        { label: 'Yes, allow edits in', scope: pathTail(dir, 3), tail: ' for this session', a: 'session' },
        { label: 'No, and tell it why', a: 'deny', free: true, hint: `type to tell ${worker()} why` },
      ],
    };
  },
  bash: (x, o) => ({
    title: 'Run this command?', right: [s(`in ${HERE.dir}`, 'dim')],
    body: cmdBody(x.arg, o.why), rule: cmdName(x.arg),
    options: [
      { label: 'Yes', a: 'once' },
      { label: 'Yes, allow', scope: cmdName(x.arg), tail: ' for this session', a: 'session' },
      { label: 'No, and tell it why', a: 'deny', free: true, hint: `type to tell ${worker()} why` },
      { label: 'Change it first', a: 'change', free: true, hint: '⏎ to edit the command' },
    ],
  }),
  caution: (x, o) => ({
    title: 'Run this command?', danger: true, right: [s(o.caution, 'bad')], start: 1,
    body: W => [
      ...wrapSegs([s('$ ', 'dim'), s(x.arg, 'code')], W - TX - RM - 2).map(r => L(TX + 2, r)),
      L(TX + 2, [s(`drops ${plural(o.lost.length, 'commit')} that only origin/dev has`, 'bad')]),
      ...o.lost.map(c => L(TX + 4, fitSegs([s(c.slice(0, 7), 'muted'), s(c.slice(7), 'dim')], W - TX - RM - 4))),
    ],
    options: [
      { label: 'Yes, force-push', a: 'once', tone: 'bad' },
      { label: 'No, and tell it why', a: 'deny', free: true, hint: `type to tell ${worker()} why` },
      { label: 'Change it first', a: 'change', free: true, hint: '⏎ to edit the command' },
    ],
  }),
};
/* a mode or a rule you set answers before the dock ever opens */
function answeredBy(kind, spec) {
  if (spec.danger) return null;
  if (spec.rule && S.rules.includes(spec.rule)) return 'session rule';
  if (modeIs('bypass')) return 'bypass';
  return kind === 'edit' && modeIs('accept edits') ? 'accept edits' : null;
}
function openQuestion(t, x, T, o) {
  S.dock = { kind: 'question', t, step: x, qs: o.qs, then: o.then, tab: 0, cur: o.qs.map(() => 0), ticks: o.qs.map(() => []), answers: [], typing: null, at: T };
  S.focus = 'dock';
}
