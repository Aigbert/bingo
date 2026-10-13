import { test } from 'node:test';
import assert from 'node:assert/strict';
import { matches, patternMatches, Hooks } from './on.ts';
import cases from '../test/cases.json' with { type: 'json' };

test('matchers are partial, nested, scalar, regex or alternatives', () => {
  for (const [matcher, event, expected] of cases.matchers) assert.equal(matches(matcher, event), expected);
  assert.equal(matches({ tool: /^Ba/ }, { tool: 'Bash' }), true);
  const matcher = /a/g;
  assert.equal(matches(matcher, 'a'), true);
  assert.equal(matches(matcher, 'a'), true);
});

test('patterns do not depend on the current event catalogue', () => {
  for (const [pattern, name, expected] of cases.patterns) assert.equal(patternMatches(pattern as string, name as string), expected);
});

test('multiple hooks wrap once, preserving after order', async () => {
  const hooks = new Hooks();
  const order: string[] = [];
  hooks.on('future.*', async ($, e, next) => {
    order.push('outer');
    assert.equal(Object.isFrozen(e), true);
    const result = await next({ ...e, added: true });
    order.push('outer after');
    return result + 1;
  });
  hooks.on('*', async ($, e, next) => {
    order.push('inner');
    const result = await next(e);
    assert.equal(next.called, true);
    await assert.rejects(next(e), /once/);
    order.push('inner after');
    return result + 1;
  });
  const first = await hooks.before('future.verb', { chain: '1', input: {}, site: {}, tier: 'tighten' }, {});
  assert.deepEqual(first, { next: { added: true } });
  assert.deepEqual(await hooks.after({ chain: '1', result: 5 }), { result: 7 });
  assert.deepEqual(order, ['outer', 'inner', 'inner after', 'outer after']);
  assert.equal(hooks.pending.size, 0);
});

test('short circuit answers without next and cancellation rejects next', async () => {
  const hooks = new Hooks();
  hooks.on('tool.check', { tool: 'Bash' }, () => ({ deny: 'blocked' }));
  assert.deepEqual(await hooks.before('tool.check', { chain: '1', input: { tool: 'Bash' } }, {}), { answer: { deny: 'blocked' } });
  hooks.on('tool.call', async ($, e, next) => {
    await assert.rejects(next(e), /cancel/);
    assert.equal(next.signal.aborted, true);
  });
  await hooks.before('tool.call', { chain: '2', input: {} }, {});
  hooks.cancel('2');
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(hooks.pending.size, 0);
});

test('an omitted matcher accepts null and scalar event inputs', async () => {
  for (const input of [null, false, 0, 'complete']) {
    const hooks = new Hooks();
    hooks.on('future.verb', ($, e) => ({ observed: e }));
    assert.deepEqual(await hooks.before('future.verb', { chain: '1', input }, {}),
      { answer: { observed: input } });
  }
});

test('cancel before next rejects before and clears the chain', { timeout: 5000 }, async () => {
  const hooks = new Hooks();
  let entered!: () => void;
  const started = new Promise<void>(resolve => { entered = resolve; });
  let continueHook!: () => void;
  const held = new Promise<void>(resolve => { continueHook = resolve; });
  hooks.on('future.verb', async ($, e, next) => {
    entered();
    await held;
    assert.equal(next.signal.aborted, true);
    await assert.rejects(next(e), /cancel/);
  });
  const before = hooks.before('future.verb', { chain: '1', input: {} }, {});
  await started;
  const rejected = assert.rejects(before, /cancel/);
  hooks.cancel('1');
  continueHook();
  await rejected;
  assert.equal(hooks.pending.size, 0);
});

test('cancel while the after hook is waiting rejects after', { timeout: 5000 }, async () => {
  const hooks = new Hooks();
  let entered!: () => void;
  const started = new Promise<void>(resolve => { entered = resolve; });
  hooks.on('future.verb', async ($, e, next) => {
    await next(e);
    entered();
    await new Promise(() => {});
  });
  await hooks.before('future.verb', { chain: '1', input: {} }, {});
  const after = hooks.after({ chain: '1', result: 'done' });
  await started;
  const rejected = assert.rejects(after, /cancel/);
  hooks.cancel('1');
  await rejected;
  assert.equal(hooks.pending.size, 0);
});
