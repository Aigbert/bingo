import type { Dollar } from './dollar.ts';

export type Atom<T> = Readonly<{key: string; initial: T}>;
export type Derived<T> = Readonly<{compute: ($: Dollar) => T | Promise<T>}>;
export const atom = <T>(key: string, initial: T): Atom<T> => Object.freeze({key, initial});
export const derive = <T>(compute: ($: Dollar) => T | Promise<T>): Derived<T> => Object.freeze({compute});
const renders = new WeakMap<Dollar, Map<Derived<unknown>, Promise<unknown>>>();

export async function read<T>($: Dollar, value: Atom<T> | Derived<T>): Promise<T> {
  if ('key' in value) {
    const held = await $.state.get(value.key);
    return held === null ? value.initial : held.value as T;
  }
  let memo = renders.get($);
  if (!memo) renders.set($, memo = new Map());
  if (!memo.has(value)) memo.set(value, Promise.resolve().then(() => value.compute($)));
  return memo.get(value) as Promise<T>;
}

export async function update<T>($: Dollar, value: Atom<T>, change: (held: T) => T | Promise<T>): Promise<T> {
  for (let retry = 0; ; retry++) {
    const held = await $.state.get(value.key);
    const next = await change(held === null ? value.initial : held.value as T);
    try {
      const saved = await $.state.set(value.key, next, held?.version);
      renders.delete($);
      return saved.value as T;
    } catch (error) {
      if (retry >= 3 || !conflict(error)) throw error;
    }
  }
}
function conflict(error: unknown): boolean {
  return typeof error === 'object' && error !== null &&
    (('code' in error && error.code === -32009) ||
     ('message' in error && typeof error.message === 'string' && error.message.includes('plugin state version conflict')));
}
export const memberOf = <T>(values: readonly T[]) => (value: unknown): value is T => values.includes(value as T);
