// Export the ask scenes' turn data and the asks they open; tests fold this data as events.
// Merges into acting-turns.json and leaves the other scenes' keys as they are.
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const root = path.resolve(__dirname, '../../..');
const dir = path.join(root, 'docs/design/ember/prototype');
const CLOCK_ORIGIN = 10000000;
const context = vm.createContext({performance:{now:()=>0}});
const sources = ['p3-core','p4-model','p4b-data','p4c-act','p4d-crew','p5-stream','p6-dock','p7-input','p8-page'].map(name => {
  let source = fs.readFileSync(path.join(dir, name+'.js'), 'utf8');
  if (name === 'p8-page') {
    source = source.replace(/\ninit\(\);\s*$/, '\n');
    source = source.replace('return { name, t: clock - T0, W: S.W, H: S.H, rows:', 'return { state: JSON.parse(JSON.stringify(S, (key, value) => key === "script" || key === "t" && value && value.items ? undefined : value)), name, t: clock - T0, W: S.W, H: S.H, rows:');
  }
  return source;
});
vm.runInContext(sources.join('\n'), context);
// Each ask the dock opens: when, which item asked, and what the beat said beside the step.
vm.runInContext(`PAL = PALS.light; var ASKS = [];
const openAsk = askBeat;
askBeat = function (t, T, kind, o) {
  openAsk(t, T, kind, o);
  if (!S.dock || S.dock.t !== t) return;
  const x = S.dock.step;
  ASKS.push({ at: T, kind, item: t.items.indexOf(x), why: o.why || null, caution: o.caution || null, lost: o.lost || null, qs: o.qs || null });
};`, context);
const names = ['ask-edit','ask-edit-down','ask-edit-allowed','ask-bash','ask-bash-change','ask-bash-editing','caution','caution-no','question','question-preview','question-second','question-ticked','question-answered','small-ask-edit','ascii-ask-edit'];
const index = JSON.parse(fs.readFileSync(path.join(root,'docs/design/ember/scenes/index.json')));
const target = path.join(root,'crates/bingo-surface-tui/src/test_support/ember/acting-turns.json');
const fixtures = JSON.parse(fs.readFileSync(target, 'utf8'));
for (const name of names) {
  const entry = index.find(x=>x.file===name);
  const sequence = [...(entry.after || [entry.t]), 'shot'];
  vm.runInContext('ASKS = [];', context);
  const shots = vm.runInContext(`capture(${JSON.stringify(entry.scene)}, ${JSON.stringify(entry.size)}, ${JSON.stringify(sequence)}, ${JSON.stringify(entry.flags || {})})`, context);
  const shot = shots.at(-1);
  if (shot.t !== entry.t) throw new Error(`${name}: clock differs`);
  const asks = JSON.parse(JSON.stringify(vm.runInContext('ASKS', context)));
  const {log} = shot.state;
  const rate = vm.runInContext('RATE_R', context);
  for (const turn of log) {
    if (turn.state !== 'done' && turn.reply) turn.reply.revealed = Math.floor(Math.max(0, CLOCK_ORIGIN + shot.t - turn.reply.at) / 1000 * rate);
  }
  const live = log.slice(entry.scene === 'caution' ? 2 : 1);
  live[0].asks = asks;
  fixtures[name] = live;
}
fs.writeFileSync(target, JSON.stringify(fixtures, function(key, value) { return key === 'script' ? undefined : value; }, 2)+'\n');
console.log(`Exported ${names.length} ask scenes beside ${Object.keys(fixtures).length - names.length} others`);
