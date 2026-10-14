import { test } from 'node:test';
import assert from 'node:assert/strict';
import { atom, derive, read, update, memberOf } from './state.ts';
import type { Dollar } from './dollar.ts';

function host(conflicts = 0) {
  let gets = 0, sets = 0;
  let held: {value: unknown; version: number} | null = {value: 2, version: 1};
  const $ = {state: {
    get: async () => { gets++; return held; },
    set: async (_: string, value: unknown, ifVersion?: number) => {
      sets++;
      if (conflicts-- > 0) { held = {value: 4, version: 2}; throw {code: -32009, message: 'plugin state version conflict'}; }
      assert.equal(ifVersion, held?.version);
      return held = {value, version: (held?.version ?? 0) + 1};
    },
  }} as unknown as Dollar;
  return {$, counts: () => [gets, sets]};
}
test('derive memoizes within a render context and update retries fresh versions', async () => {
  const count = atom('count', 0);
  const double = derive(async ($: Dollar) => 2 * await read($, count));
  const one = host(2);
  assert.deepEqual(await Promise.all([read(one.$, double), read(one.$, double)]), [4, 4]);
  assert.deepEqual(one.counts(), [1, 0]);
  assert.equal(await update(one.$, count, n => n + 1), 5);
  assert.deepEqual(one.counts(), [4, 3]);
  const two = host();
  assert.equal(await read(two.$, double), 4);
  assert.deepEqual(two.counts(), [1, 0]);
});
test('update stops after three conflict retries and propagates other errors', async () => {
  const count = atom('count', 0);
  const one = host(9);
  await assert.rejects(update(one.$, count, n => n + 1));
  assert.deepEqual(one.counts(), [4, 4]);
  const broken = host();
  broken.$.state.set = async () => { throw new Error('disk is full'); };
  await assert.rejects(update(broken.$, count, n => n + 1), /disk is full/);
  assert.deepEqual(broken.counts(), [1, 0]);
});
test('resting reads do not write and membership is exact', async () => {
  const one = host();
  one.$.state.get = async () => null;
  assert.equal(await read(one.$, atom('fresh', false)), false);
  assert.deepEqual(one.counts(), [0, 0]);
  assert.equal(memberOf(['alpha', 'beta'])('alpha'), true);
  assert.equal(memberOf(['alpha', 'beta'])('alphabet'), false);
});
