// Export state from the prototype's own capture clock, never rendered cells.
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
const names = ['live-steered','live-stopped','stop','stop-resent','steer','steer-landed','shell','shell-ls','image','image-sent'];
const index = JSON.parse(fs.readFileSync(path.join(root,'docs/design/ember/scenes/index.json')));
const fixtures = {};
for (const name of names) {
  const entry = index.find(x=>x.file===name);
  const sequence = [...(entry.after || [entry.t]), 'shot'];
  const shots = vm.runInContext(`capture(${JSON.stringify(entry.scene)}, ${JSON.stringify(entry.size)}, ${JSON.stringify(sequence)}, {})`, context);
  const shot = shots.at(-1);
  if (shot.t !== entry.t) throw new Error(`${name}: clock differs`);
  const {log, queued, input, chips, mode, toast, keyAt} = shot.state;
  const rate = vm.runInContext('RATE_R', context);
  for (const turn of log) {
    if (turn.state === 'live' && turn.reply) turn.reply.revealed = Math.floor(Math.max(0, 10000000 + shot.t - turn.reply.at) / 1000 * rate);
  }
  fixtures[name] = {log, queued, input, chips, mode, toast, keyAt};
}
fs.writeFileSync(path.join(root,'crates/bingo-surface-tui/src/test_support/ember/acting.json'), JSON.stringify(fixtures,null,2)+'\n');
console.log('Exported 10 acting states at their capture clocks');
