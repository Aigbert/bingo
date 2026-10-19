// Export only prototype state; capture rows remain immutable references.
const fs = require('node:fs'), vm = require('node:vm'), path = require('node:path');
const root = path.resolve(__dirname, '../../..'), dir = path.join(root, 'docs/design/ember/prototype');
const context = vm.createContext({performance:{now:()=>0}});
const names = ['p3-core','p4-model','p4b-data','p4c-act','p4d-crew','p5-stream','p6-dock','p7-input','p8-page'];
vm.runInContext(names.map(name => {
 let source = fs.readFileSync(path.join(dir, name+'.js'), 'utf8');
 if (name === 'p8-page') source = source.replace(/\ninit\(\);\s*$/, '\n').replace('return { name, t: clock - T0, W: S.W, H: S.H, rows:', 'return { state: JSON.parse(JSON.stringify(S, (key, value) => key === "script" ? undefined : value)), name, t: clock - T0, W: S.W, H: S.H, rows:');
 return source;
}).join('\n'), context);
vm.runInContext('PAL = PALS.light;', context);
const index = JSON.parse(fs.readFileSync(path.join(root, 'docs/design/ember/scenes/index.json')));
// Keep only what run/acting/edges.rs and story.rs read; the rest is prototype bookkeeping.
const pick = (value, keys) => Object.fromEntries(keys.filter(k => value[k] != null).map(k => [k, value[k]]));
const ITEM = {step: ['k','at','end','st','add','del','res','tail','tool','arg']}, PLAIN = ['k','at','end','text'];
const item = data => pick(data, ITEM[data.k] || PLAIN);
const turn = t => Object.assign(pick(t, ['kind','t0','prompt','state','endAt']), {items: t.items.map(item)},
 t.reply ? {reply: pick(t.reply, ['at','blocks','revealed'])} : {}, t.retry ? {retry: pick(t.retry, ['at','until','n'])} : {},
 t.error ? {error: pick(t.error, ['msg','code'])} : {},
 t.chips && t.chips.length ? {chips: t.chips} : {});
const curate = (scene, state) => Object.assign({title: state.title, log: state.log.filter(t => t.kind === 'turn').map(turn)},
 scene === 'background' ? {jobs: state.jobs.map(() => ({}))} : {});
const fixtures = {};
for (const name of ['welcome','welcome-recent','welcome-sent','exit-back','retry','retry-recovered','context','context-compacted','background','background-notified','narrow','signedout','signedout-login','wake','wake-later']) {
 const entry = index.find(x=>x.file===name);
 const seq = [...(entry.after || [entry.t]), 'shot'];
 const shot = vm.runInContext(`capture(${JSON.stringify(entry.scene)},${JSON.stringify(entry.size)},${JSON.stringify(seq)},{}).at(-1)`,context);
 fixtures[name] = curate(entry.scene, shot.state);
}
fs.writeFileSync(path.join(root, 'crates/bingo-surface-tui/src/test_support/ember/edges.json'), JSON.stringify(fixtures,null,2)+'\n');
