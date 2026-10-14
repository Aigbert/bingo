import { test, expect } from 'bingo/testing';
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
      expect(ifVersion).toBe(held?.version);
      return held = {value, version: (held?.version ?? 0) + 1};
    },
  }} as unknown as Dollar;
  return {$, counts: () => [gets, sets]};
}
test('derive memoizes within a render context and update retries fresh versions', async () => {
  const count = atom('count', 0);
  const double = derive(async ($: Dollar) => 2 * await read($, count));
  const one = host(2);
  expect(await Promise.all([read(one.$, double), read(one.$, double)])).toEqual([4, 4]);
  expect(one.counts()).toEqual([1, 0]);
  expect(await update(one.$, count, n => n + 1)).toBe(5);
  expect(one.counts()).toEqual([4, 3]);
  expect(await read(one.$, double)).toBe(10);
  const two = host();
  expect(await read(two.$, double)).toBe(4);
  expect(two.counts()).toEqual([1, 0]);
});
test('update stops after three conflict retries and propagates other errors', async () => {
  const count = atom('count', 0);
  const one = host(9);
  await expect(update(one.$, count, n => n + 1)).rejects.toThrow();
  expect(one.counts()).toEqual([4, 4]);
  const broken = host();
  broken.$.state.set = async () => { throw new Error('disk is full'); };
  await expect(update(broken.$, count, n => n + 1)).rejects.toThrow('disk is full');
  expect(broken.counts()).toEqual([1, 0]);
});
test('resting reads do not write and membership is exact', async () => {
  const one = host();
  one.$.state.get = async () => null;
  expect(await read(one.$, atom('fresh', false))).toBe(false);
  expect(one.counts()).toEqual([0, 0]);
  expect(memberOf(['alpha', 'beta'])('alpha')).toBe(true);
  expect(memberOf(['alpha', 'beta'])('alphabet')).toBe(false);
});
