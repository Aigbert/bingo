// Export the prototype's turn data, not its rendered rows. Tests fold this data as events.
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const root = path.resolve(__dirname, '../../..');
const dir = path.join(root, 'docs/design/ember/prototype');
const context = vm.createContext({});
vm.runInContext(`let now = 0; let S; const NOW = () => now;
const markCells = () => []; const blankSession = o => Object.assign({log:[], queued:[], hist:[], mode:0}, o);
const reportBack = () => {}; const shortTitle = x => x;
`, context);
// shortTitle is declared in the model, so use its dependencies only when needed.
vm.runInContext(fs.readFileSync(path.join(dir, 'p4-model.js'), 'utf8').replace('function shortTitle(p)', 'function unusedShortTitle(p)'), context);
vm.runInContext(fs.readFileSync(path.join(dir, 'p4b-data.js'), 'utf8'), context);
for (const scene of ['live','done','stop','steer','steer-landed']) {
 const result = vm.runInContext(`(() => {
   now = 0; S = fresh(); S.log.push(T1());
   let t;
   if (${JSON.stringify(scene)} === 'done') { S.log.push(T2done()); return S.log; }
   const name = ${JSON.stringify(scene)};
   const ago = name === 'stop' ? 2000 : name.startsWith('steer') ? 1600 : 2600;
   t = startTurn(T2P, mainScript({}), {t0:-ago, title:'fixture'});
   runScript(t, now, true);
   if (name.startsWith('steer')) S.queued.push({text:'测试名字用英文', steer:true, at:0});
   now = name === 'stop' ? 1800 : name === 'steer' ? 200 : name === 'steer-landed' ? 4300 : 700;
   runScript(t, now, true);
   if (name === 'stop') stopTurn(t);
   return S.log;
 })()`, context);
 const target = path.join(root, `crates/bingo-surface-tui/src/test_support/ember/${scene}.json`);
 fs.writeFileSync(target, JSON.stringify(result, function(key, value) { return key === 'script' || (this.k === 'step' && ['code','diff','out','qa'].includes(key)) ? undefined : value; }, 2)+'\n');
}
fs.writeFileSync(path.join(root, 'crates/bingo-surface-tui/src/test_support/ember/tools.json'), JSON.stringify(vm.runInContext('TOOLS', context), null, 2)+'\n');

{
// Additional turn-only samples at the acting capture instants.
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const root = path.resolve(__dirname, '../../..');
const proto = path.join(root, 'docs/design/ember/prototype');
const context = vm.createContext({performance:{now:()=>0}});
const sources = [];
for (const name of ['p3-core','p4-model','p4b-data','p4c-act','p4d-crew','p5-stream','p6-dock','p7-input','p8-page']) {
  let source = fs.readFileSync(path.join(proto, name+'.js'),'utf8');
  if (name === 'p8-page') {
    source = source.replace(/\ninit\(\);\s*$/, '\n');
    source = source.replace('return { name, t: clock - T0, W: S.W, H: S.H, rows:', 'return { state: JSON.parse(JSON.stringify(S, (key, value) => key === "script" ? undefined : value)), name, t: clock - T0, W: S.W, H: S.H, rows:');
  }
  sources.push(source);
}
vm.runInContext(sources.join('\n'), context);
vm.runInContext('PAL = PALS.light;', context);
const names = ['live-steered','live-stopped','stop-resent','shell','shell-ls','image-sent'];
const index = JSON.parse(fs.readFileSync(path.join(root,'docs/design/ember/scenes/index.json')));
const fixtures = {};
for (const name of names) {
  const entry = index.find(x=>x.file===name);
  const sequence = [...(entry.after || [entry.t]), 'shot'];
  const shots = vm.runInContext(`capture(${JSON.stringify(entry.scene)}, ${JSON.stringify(entry.size)}, ${JSON.stringify(sequence)}, {})`, context);
  const shot = shots.at(-1);
  if (shot.t !== entry.t) throw new Error(`${name}: clock differs`);
  const {log} = shot.state;
  const rate = vm.runInContext('RATE_R', context);
  for (const turn of log) {
    if (turn.state === 'live' && turn.reply) turn.reply.revealed = Math.floor(Math.max(0, 10000000 + shot.t - turn.reply.at) / 1000 * rate);
  }
  fixtures[name] = log.slice(['stop-resent','image-sent'].includes(name) ? 2 : 1);
}
fs.writeFileSync(path.join(root,'crates/bingo-surface-tui/src/test_support/ember/acting-turns.json'), JSON.stringify(fixtures,null,2)+'\n');
console.log('Exported acting turns only; keys, queues, drafts and toasts stay in the run');

}
