// Line-breaking cases for p3-core.js wrapSegs: node wraptest.js prints each case's rows (the Rust port's expected values).
const fs = require('fs'), vm = require('vm');
const ctx = { window: {}, document: { documentElement: { dataset: {} } }, matchMedia: () => ({ matches: false, addEventListener() {} }), localStorage: { getItem() { return null; }, setItem() {} }, console };
vm.createContext(ctx);
vm.runInContext(fs.readFileSync(require('path').join(__dirname, 'p3-core.js'), 'utf8') + '\nthis.wrapSegs = wrapSegs; this.s = s; this.sw = sw;', ctx);
const { wrapSegs, s } = ctx;
const show = (segs, w) => wrapSegs(segs, w).map(l => l.map(x => x.t).join(''));
// the screenshot case: a code span then 。 must not leave 。 alone
const a = [s('把返回值改成拥有所有权的 ', 'fg'), s('Message', 'code'), s(' 就行，', 'fg'), s('pop_front()', 'code'), s(' 本来就把所有权交给你；或者在调用处先 ', 'fg'), s('clone()', 'code'), s('。', 'fg')];
for (const w of [60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70]) {
  const rows = show(a, w);
  const bad = rows.slice(1).some(r => /^[，。、；：！？）」』]/.test(r));
  const over = rows.some(r => ctx.sw(r) > w);
  console.log(w, bad ? 'BAD-START' : 'ok', over ? 'OVER' : '', JSON.stringify(rows));
}
// opening bracket must not end a line; Latin split by styling stays whole
console.log(JSON.stringify(show([s('所以只有「插话之后紧跟着排队」才会丢。', 'fg')], 13)));
console.log(JSON.stringify(show([s('abc ', 'fg'), s('queue', 'code'), s('s are', 'fg')], 8)));
// a unit wider than the line is still cut
console.log(JSON.stringify(show([s('averyveryverylongidentifier_without_breaks', 'code')], 10)));
console.log(JSON.stringify(show([s('line one\nline two', 'fg')], 40)));
