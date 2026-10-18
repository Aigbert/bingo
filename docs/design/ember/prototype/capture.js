// Export the Ember prototype's scenes as reference captures: characters (<name>.txt) and ink (<name>.ink).
// usage: node capture.js <bingo-ember.html> <out-dir>   (build.sh makes the page)
// needs playwright-core (PLAYWRIGHT_CORE=<path to its package> if it is not resolvable) and a
// Chromium (CHROME=<executable>, else the macOS Google Chrome path).
const path = require('path'), fs = require('fs');
const { chromium } = require(process.env.PLAYWRIGHT_CORE || 'playwright-core');
const [html, out] = process.argv.slice(2);
if (!html || !out) { console.error('usage: node capture.js <bingo-ember.html> <out-dir>'); process.exit(2); }

const DRIVES = {
  welcome: [2700, 'up', 300, 'shot:recent', 'esc', ['修好它'], 'enter', 1500, 'shot:sent'],
  live: [800, ['测试名字用英文'], 'enter', ['再更新 CHANGELOG'], 'tab', 400, 'shot:steered', 'esc', 600, 'shot:stopped'],
  done: [300, 'ctrl+o', 300, 'shot:detail', 'down', 'enter', 300, 'shot:well', 'esc', 200, 'shot:folded'],
  ledger: [300, 'up', 'enter', 300, 'shot:up'],
  stop: [2900, 'enter', 800, 'shot:resent'],
  steer: [300, 4000, 'shot:landed'],
  shell: [800, ['!ls'], 'enter', 800, 'shot:ls'],
  image: [300, 'enter', 2500, 'shot:sent'],
  'ask-edit': [800, 'down', 300, 'shot:down', 'enter', 1500, 'shot:allowed'],
  'ask-bash': [800, 'down', 'down', 'down', 300, 'shot:change', 'enter', 500, 'shot:editing'],
  caution: [900, 'enter', 400, 'shot:no'],
  question: [900, 'down', 300, 'shot:preview', 'enter', 400, 'shot:second', 'space', 'down', 'space', 300, 'shot:ticked', 'enter', 1500, 'shot:answered'],
  mode: [2400, 2000, 'shot:later'],
  palette: [300, ['st'], 300, 'shot:st', 'down', 'enter', 500, 'shot:ran'],
  skills: [300, 'down', 'down', 'down', 300, 'shot:review', 'enter', 1200, 'shot:sent'],
  mention: [300, 'down', 'tab', 300, 'shot:inserted'],
  model: [300, 'down', 'right', 300, 'shot:moved', 'enter', 400, 'shot:chosen'],
  status: [300, 'down', 'down', 'down', 300, 'shot:moved', 'enter', 400, 'shot:entered'],
  plugins: [300, 'down', 'space', 300, 'shot:toggled', 'up', 'enter', 400, 'shot:github'],
  sessions: [300, ['ci'], 300, 'shot:filtered', 'enter', 500, 'shot:switched'],
  rewind: [300, 'down', 'enter', 600, 'shot:rewound'],
  keys: [300, 'esc', 200, 'shot:closed'],
  tasks: [300, 'esc', 'ctrl+t', 300, 'shot:again'],
  schedule: [300, 'space', 300, 'shot:paused', 'down', 'enter', 400, 'shot:ran'],
  memory: [300, 'down', 'down', 'down', 'backspace', 300, 'shot:forgot'],
  find: [300, 'up', 300, 'shot:up', 'esc', 300, 'shot:back'],
  crew: [4200, 'ctrl+g', 600, 'shot:reviewer', 'enter', 3600, 'shot:allowed', 'esc', 400, 'shot:home'],
  agent: [800, 'ctrl+g', 600, 'shot:reviewer', 'enter', 3600, 'shot:allowed', 'esc', 400, 'shot:home'],
  room: [600, ['@rev'], 300, 'shot:mention', 'tab', ['看一下 reset() 的注释'], 'enter', 1500, 'shot:waiting', 3000, 'shot:answered', 'esc', 400, 'shot:home'],
  background: [3600, 9000, 'shot:notified'],
  wake: [7600, 3000, 'shot:later'],
  retry: [700, 4000, 'shot:recovered'],
  signedout: [1100, ['/login'], 'enter', 2500, 'shot:login'],
  context: [300, ['/compact'], 'enter', 1500, 'shot:compacted'],
  exit: [600, 'enter', 800, 'shot:back'],
};
/* the same scenes under the terminal's other conditions */
const VARIANTS = [
  ['ascii-done', 'done', '100x32', 300, { ascii: true }], ['ascii-live', 'live', '100x32', 900, { ascii: true }],
  ['ascii-ask-edit', 'ask-edit', '100x32', 900, { ascii: true }], ['ascii-palette', 'palette', '100x32', 300, { ascii: true }],
  ['reduced-live', 'live', '100x32', 900, { reduced: true }], ['reduced-mode', 'mode', '100x32', 2400, { reduced: true }],
  ['small-done', 'done', '80x24', 300, {}], ['small-ask-edit', 'ask-edit', '80x24', 900, {}],
  ['small-palette', 'palette', '80x24', 300, {}], ['small-crew', 'crew', '80x24', 4200, {}],
];

const pad = n => String(n).padStart(2, '0');
function write(name, meta, snap) {
  const head = `# ${name} · scene ${meta.scene} · ${snap.t} ms · ${snap.W}x${snap.H}${meta.flags.ascii ? ' · ascii' : ''}${meta.flags.reduced ? ' · reduced motion' : ''} · light`;
  fs.writeFileSync(path.join(out, name + '.txt'), head + '\n' + snap.rows.join('\n') + '\n');
  const ink = [head, '# one line per layer of a row with ink on it: row, layer, then x0-x1 token runs (columns 0-based, inclusive)'];
  snap.ink.forEach((r, y) => {
    const runs = xs => xs.map(([a, b, k]) => (a === b ? `${a}` : `${a}-${b}`) + ' ' + k).join(' · ');
    if (r.bg.length) ink.push(`${pad(y)} bg ${runs(r.bg)}`);
    if (r.fg.length) ink.push(`${pad(y)} fg ${runs(r.fg)}`);
  });
  fs.writeFileSync(path.join(out, name + '.ink'), ink.join('\n') + '\n');
}

(async () => {
  fs.mkdirSync(out, { recursive: true });
  const browser = await chromium.launch({ executablePath: process.env.CHROME || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', headless: true });
  const page = await (await browser.newContext({ viewport: { width: 1440, height: 1000 }, colorScheme: 'light' })).newPage();
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.goto('file://' + path.resolve(html), { waitUntil: 'load' });
  await page.waitForFunction(() => window.__bingo && window.__bingo.capture);
  const scenes = await page.evaluate(() => window.__bingo.scenes());
  const index = [], mism = [];
  const cap = (id, size, seq, flags) => page.evaluate(([id, size, seq, flags]) => window.__bingo.capture(id, size, seq, flags), [id, size, seq, flags]);
  for (const sc of scenes) {
    const size = sc.id === 'narrow' ? '80x24' : '100x32';
    const [still] = await cap(sc.id, size, [sc.t, 'shot'], {});
    write(sc.id, { scene: sc.id, flags: {} }, still);
    index.push({ file: sc.id, scene: sc.id, grp: sc.grp, name: sc.name, card: sc.card, t: still.t, size: `${still.W}x${still.H}` });
    /* the gallery's still and the capture must agree, or frame() has a side effect the plans would inherit */
    const snapRows = await page.evaluate(([id, t, size]) => window.__bingo.snapshot(id, t, size).rows.map(R => R.c.map(c => c.ch).join('')), [sc.id, sc.t, size]);
    const capRows = await page.evaluate(([id, t, size]) => window.__bingo.capture(id, size, [t, 'shot'], {})[0].rows, [sc.id, sc.t, size]);
    const plain = r => r.replace(/\s+$/, '');
    if (snapRows.map(plain).join('\n').replace(/ /g, '') !== capRows.map(plain).join('\n').replace(/ /g, '')) mism.push(sc.id);
    const drive = DRIVES[sc.id];
    if (!drive) continue;
    const shots = await cap(sc.id, size, drive, {});
    for (const sh of shots) {
      const name = `${sc.id}-${sh.name}`;
      write(name, { scene: sc.id, flags: {} }, sh);
      index.push({ file: name, scene: sc.id, grp: sc.grp, name: sc.name, after: drive.slice(0, drive.indexOf('shot:' + sh.name)), t: sh.t, size: `${sh.W}x${sh.H}` });
    }
  }
  for (const [name, id, size, t, flags] of VARIANTS) {
    const [sh] = await cap(id, size, [t, 'shot'], flags);
    write(name, { scene: id, flags }, sh);
    index.push({ file: name, scene: id, t: sh.t, size: `${sh.W}x${sh.H}`, flags });
  }
  fs.writeFileSync(path.join(out, 'index.json'), JSON.stringify(index, null, 1) + '\n');
  console.log(JSON.stringify({ captures: index.length, errors, snapshotMismatch: mism }));
  await browser.close();
})();
