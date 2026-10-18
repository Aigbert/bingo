/* ---------- input: one key, one meaning wherever the focus is ---------- */
const CTRL = { KeyO: 'ctrl+o', KeyR: 'ctrl+r', KeyT: 'ctrl+t', KeyF: 'ctrl+f', KeyG: 'ctrl+g', KeyB: 'ctrl+b', KeyC: 'ctrl+c', KeyD: 'ctrl+d', KeyV: 'ctrl+v' };
const KEYS = { Enter: 'enter', Tab: 'tab', ArrowUp: 'up', ArrowDown: 'down', ArrowLeft: 'left', ArrowRight: 'right', Escape: 'esc',
  Backspace: 'backspace', End: 'end', PageUp: 'pgup', PageDown: 'pgdn', ' ': 'space' };
function keyName(e) {
  if (e.ctrlKey && !e.altKey && !e.metaKey) return CTRL[e.code] || null;
  if (e.metaKey) return null;
  if (e.altKey) return e.code === 'KeyM' ? 'alt+m' : e.code === 'KeyT' ? 'alt+t' : null;
  const k = KEYS[e.key];
  if (k === 'enter' || k === 'tab') return e.shiftKey ? `shift+${k}` : k;
  return k || ([...e.key].length === 1 ? e.key : null);
}
const NAMED = new Set([...Object.values(KEYS), ...Object.values(CTRL), 'shift+enter', 'shift+tab', 'alt+m', 'alt+t']);

const GLOBAL = {
  'shift+tab': cycleMode, 'alt+m': nextModel, 'alt+t': nextEffort,
  'ctrl+o': toggleLedger, 'ctrl+r': () => openSheet('sessions'), 'ctrl+t': () => openSheet('tasks'), 'ctrl+f': openFind,
};
const DOCK_KEYS = { ask: askKey, question: questionKey, find: findKey, sheet: sheetKey };
function press(k) {
  S.keyAt = NOW();
  if (S.exited) return shellKey(k);
  if (k === 'ctrl+c') return ctrlC();
  if (k === 'ctrl+g') return nextNeed();
  if (S.focus === 'welcome') return welcomeKey(k);
  const d = S.dock;
  if (d && DOCK_KEYS[d.kind]) return DOCK_KEYS[d.kind](d, k);
  if (S.focus === 'ledger') {
    if (ledgerKey(k)) return;
    S.focus = 'input'; S.sel = null;
  }
  if (GLOBAL[k]) return GLOBAL[k]();
  composerKey(k);
}

/* ----- a sheet: its own keys first, then the list, then the keys that work everywhere ----- */
function sheetKey(d, k) {
  const sh = SHEETS[d.sheet];
  if (sh.key && sh.key(d, k)) return;
  if (k === 'esc' || k === sh.opener) return closeDock();
  if (k === 'up' || k === 'down') {
    d.sel = clamp(d.sel + (k === 'up' ? -1 : 1), 0, Math.max(0, sh.rows(d, NOW(), S.W).length - 1));
    return;
  }
  if (k === 'enter') { if (sh.enter) sh.enter(d); return; }
  if (/^ctrl\+[ortf]$/.test(k)) { closeDock(); GLOBAL[k](); return; }
  if (GLOBAL[k]) GLOBAL[k]();
}

/* ----- the composer ----- */
function composerKey(k) {
  const pal = paletteOf(), men = !pal && mentionOf();
  if ((pal || men) && listKey(k, pal, men)) return;
  if (COMPOSER[k]) return COMPOSER[k]();
  if (printable(k)) typeChar(k);
}
function listKey(k, pal, men) {
  const items = (pal || men).items;
  if (!items.length) return false;
  if (k === 'up' || k === 'down') { S.pal.sel = clamp(S.pal.sel + (k === 'up' ? -1 : 1), 0, items.length - 1); return true; }
  const it = items[clamp(S.pal.sel, 0, items.length - 1)];
  if (pal && k === 'enter') { runPick(it.c); return true; }
  if (pal && k === 'tab') { S.input = `/${it.c.n} `; S.pal.sel = 0; return true; }
  if (men && (k === 'enter' || k === 'tab')) { S.input = S.input.replace(/@[\w./-]*$/, `@${it.x.t} `); S.pal.sel = 0; return true; }
  return false;
}
/* a skill is a message to bingo; a command acts on the session */
function runPick(c) {
  if (c.kind !== 'skill') return runCommand(c.n);
  S.input = `/${c.n}`;
  submitInput();
}
const COMPOSER = {
  enter: submitInput, tab: queueInput, 'shift+enter': () => { S.input += '\n'; },
  up: () => recall(-1), down: () => recall(1), backspace, esc: escChain, space: () => { S.input += ' '; S.hi = -1; },
  end: () => { S.scroll = 0; S.scrollAt = NOW(); }, pgup: () => pageBy(1), pgdn: () => pageBy(-1),
  'ctrl+d': () => { if (!S.input && !S.chips.length) exitApp(); }, 'ctrl+v': pasteImage, 'ctrl+b': backgroundBash,
};
function submitInput() {
  const text = S.input.trim(), chips = S.chips.slice(), cmd = text.match(/^\/(\S+)\s*([\s\S]*)$/);
  if (!text && !chips.length) return;
  if (cmd && COMMANDS.some(c => c.n === cmd[1] && c.kind !== 'skill' && (!c.room || roomOpen()))) { S.hist.push(text); return runCommand(cmd[1], cmd[2].trim()); }
  if (inRoom() && !roomOpen()) return say('this room is closed');
  S.input = ''; S.chips = []; S.pal.sel = 0; S.scroll = 0; S.toast = null;
  if (inRoom()) return post(text);
  if (liveTurn()) { S.hist.push(text); S.queued.push({ text, chips, steer: true, at: NOW() }); return; }
  if (!chips.some(c => /^image \d+$/.test(c))) return send(text, chips);
  S.hist.push(text);
  startTurn(text || '看看这张图', imageScript(), { chips });
}
function queueInput() {
  const text = S.input.trim();
  if (!liveTurn() || (!text && !S.chips.length)) return;
  S.hist.push(text);
  S.queued.push({ text, chips: S.chips.slice(), at: NOW() });
  S.input = ''; S.chips = [];
}
/* ↑ on an empty line takes back what you queued last, before it reaches history */
function recall(dir) {
  const empty = !S.input && !S.chips.length;
  if (dir < 0 && empty && S.queued.length) {
    const q = S.queued.pop();
    S.input = q.text; S.chips = q.chips || [];
    return;
  }
  if (dir < 0 && empty && S.welcome && !S.welcome.out) { S.focus = 'welcome'; S.welcome.sel = RECENTS.length - 1; return; }
  if (S.hi < 0 && (!empty || dir > 0 || !S.hist.length)) return;
  S.hi = clamp((S.hi < 0 ? S.hist.length : S.hi) + dir, 0, S.hist.length);
  S.input = S.hist[S.hi] || '';
  if (S.hi === S.hist.length) S.hi = -1;
}
function backspace() {
  if (S.input) { S.input = [...S.input].slice(0, -1).join(''); S.pal.sel = 0; return; }
  S.chips.pop();
}
/* esc undoes the nearest thing: the turn, then the draft, then the scroll; twice on nothing rewinds.
   Away from your own session it never stops anything: the last esc takes you home. */
function escChain() {
  const t = liveTurn(), now = NOW(), away = S.view !== 'main';
  if (t && !away) return halt(t);
  if (S.input || S.chips.length) {
    if (S.input.trim()) S.hist.push(S.input);
    S.input = ''; S.chips = []; S.hi = -1;
    return say('cleared', `${keyText('↑')} brings it back`, 1800);
  }
  if (S.scroll > 0) { S.scroll = 0; S.scrollAt = now; return; }
  if (away) return enterView('main');
  if (!S.log.some(e => e.kind === 'turn')) return;
  if (now - S.escAt < 600) { S.escAt = -1e9; return openSheet('rewind'); }
  S.escAt = now;
  toast('esc again to rewind', 600);
}
function pageBy(dir) {
  S.scroll = clamp(S.scroll + dir * Math.max(1, S.viewH - 2), 0, S.maxScroll);
  S.scrollAt = NOW();
}
function typeChar(k) {
  if (k === '?' && !S.input && !S.chips.length && !liveTurn()) return openSheet('keys');
  S.input += k; S.pal.sel = 0; S.hi = -1;
}
/* stopping never arms the exit; only a second ctrl+c on an empty line leaves */
function ctrlC() {
  const now = NOW(), t = liveTurn();
  if (t) return halt(t);
  if (S.dock) return closeDock();
  if (S.focus === 'ledger') return foldLedger();
  if (S.focus === 'welcome') { S.focus = 'input'; return; }
  if (S.input || S.chips.length) { if (S.input.trim()) S.hist.push(S.input); S.input = ''; S.chips = []; return; }
  if (now - S.ctrlcAt < 1500) return exitApp();
  S.ctrlcAt = now;
  toast('ctrl+c again to quit', 1500);
}
function welcomeKey(k) {
  const w = S.welcome;
  if (!w || w.out) { S.focus = 'input'; return press(k); }
  if (k === 'up') { w.sel = Math.max(0, w.sel - 1); return; }
  if (k === 'down') { if (w.sel >= RECENTS.length - 1) S.focus = 'input'; else w.sel++; return; }
  if (k === 'enter') return resumeSession(RECENTS[w.sel].t);
  S.focus = 'input';
  if (k === 'esc') return;
  if (GLOBAL[k]) return GLOBAL[k]();
  composerKey(k);
}
/* back at the shell: fish suggests --resume with this session's id; taking it brings the session back as it was */
function shellKey(k) {
  if (NOW() - S.exited.at < GUARD_MS) return;
  const turns = S.log.some(e => e.kind === 'turn');
  if (turns && (k === 'enter' || k === 'right' || k === 'tab')) { S.exited = null; okToast(`resumed · ${S.title}`); return; }
  restart({ model: S.model, mode: S.mode });
}

/* ----- the mouse: a row knows what clicking it means ----- */
const CLICKS = {
  /* a row of another turn takes the selection, and folds the details it leaves */
  row: c => {
    if (S.sel && S.sel.t !== c.t) { S.sel.t.open = false; S.sel.t.well = -1; }
    c.t.well = c.t.well === c.i ? -1 : c.i; S.sel = { t: c.t, i: c.i }; S.focus = 'ledger';
  },
  resume: c => resumeSession(RECENTS[c.i].t),
  end: () => { S.scroll = 0; S.scrollAt = NOW(); },
  pal: c => { S.pal.sel = c.i; composerKey('enter'); },
  men: c => { S.pal.sel = c.i; composerKey('tab'); },
  opt: c => clickOption(c.i),
  sheet: c => { if (S.dock && S.dock.kind === 'sheet') { S.dock.sel = c.i; sheetKey(S.dock, 'enter'); } },
  view: c => walkTo(c.id),
};
function doClick(c) { if (c && CLICKS[c.act]) CLICKS[c.act](c); }
function clickOption(i) {
  const d = S.dock;
  if (!d || NOW() - d.at < GUARD_MS || d.typing != null) return;
  if (d.kind === 'ask') return answerAsk(d, i);
  if (d.kind !== 'question') return;
  d.cur[d.tab] = i;
  if (d.qs[d.tab].multi && i < d.qs[d.tab].options.length) toggleTick(d, i); else chooseOption(d, i);
}

function flashKey(k) {
  if (SNAP) return;
  const b = $$('#keys button').find(x => x.dataset.k === k);
  if (!b) return;
  b.classList.add('hit');
  setTimeout(() => b.classList.remove('hit'), 300);
}
function wireInput() {
  const wrap = $('#termwrap'), typer = $('#typer'), touch = () => matchMedia('(hover: none)').matches;
  const act = k => { press(k); flashKey(k); paint(); };
  wrap.addEventListener('keydown', e => {
    if (e.isComposing || e.keyCode === 229) { typer.focus({ preventScroll: true }); return; }
    const k = keyName(e);
    if (!k) return;
    e.preventDefault(); act(k);
  });
  typer.addEventListener('keydown', e => {
    if (e.isComposing || e.keyCode === 229) return;
    const k = keyName(e);
    if (k && NAMED.has(k)) { e.preventDefault(); act(k); }
  });
  const flush = () => {
    const v = typer.value;
    if (!v) return;
    typer.value = '';
    for (const ch of v) press(ch === ' ' ? 'space' : ch);
    paint();
  };
  typer.addEventListener('input', e => { if (!e.isComposing) flush(); });
  typer.addEventListener('compositionend', () => setTimeout(flush, 0));
  $('#term').addEventListener('click', e => {
    const r = e.target.closest('.r');
    if (r) { doClick(S.clicks[Array.prototype.indexOf.call(r.parentNode.children, r)]); S.keyAt = NOW(); paint(); }
    (touch() ? typer : wrap).focus({ preventScroll: true });
  });
  let acc = 0;
  wrap.addEventListener('wheel', e => {
    acc += e.deltaMode === 1 ? e.deltaY * 30 : e.deltaY;
    const d = Math.trunc(acc / 30);
    if (!d) { e.preventDefault(); return; }
    acc -= d * 30;
    const v = clamp(S.scroll - d, 0, S.maxScroll);
    if (v === S.scroll) return;
    S.scroll = v; S.scrollAt = NOW();
    e.preventDefault(); paint();
  }, { passive: false });
  for (const b of $$('#keys button')) b.addEventListener('click', () => {
    act(b.dataset.k);
    if (!touch()) wrap.focus({ preventScroll: true });
  });
}
