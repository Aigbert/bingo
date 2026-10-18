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
