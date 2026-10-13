import { test } from 'node:test';
import assert from 'node:assert/strict';
import { matches, patternMatches, Hooks } from './on.ts';

test('matchers are partial, nested, scalar, regex or alternatives', () => {
  for (const [matcher, event, expected] of [
    [{ tool: 'Read' }, { tool: 'Read', path: 'a' }, true],
    [{ tool: /^Ba/ }, { tool: 'Bash' }, true],
    [{ tool: ['Read', 'Bash'] }, { tool: 'Write' }, false],
    [{ input: { force: false } }, { input: { force: false, count: 2 } }, true],
    [{ input: null }, { input: {} }, false],
    [{ missing: true }, {}, false],
  ] as const) assert.equal(matches(matcher, event), expected);
  const matcher = /a/g;
  assert.equal(matches(matcher, 'a'), true);
  assert.equal(matches(matcher, 'a'), true);
});

test('patterns do not depend on the current event catalogue', () => {
  for (const [pattern, name, expected] of [
    ['*', 'future.verb', true], ['tool.*', 'tool.call', true],
    ['tool.*', 'tools.call', false], ['!tool.*', 'turn.complete', true],
    ['!tool.call', 'tool.call', false], ['future.verb', 'future.verb', true],
  ] as const) assert.equal(patternMatches(pattern, name), expected);
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
