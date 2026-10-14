export type { Register, On, Hook, Next } from './on.ts';
export type { Dollar, WithServices } from './dollar.ts';
export type * from './events.ts';
export { h, Fragment } from './elements';
export type { Element } from './elements';
import type { Dollar } from './dollar.ts';
export interface PluginState {}
export interface Atom<T> { readonly key: string; readonly initial: T }
export interface Derived<T> { readonly compute: ($: Dollar) => T | Promise<T> }
export declare function atom<T>(key: string, initial: T): Atom<T>;
export declare function derive<T>(fn: ($: Dollar) => T | Promise<T>): Derived<T>;
export declare function read<T>($: Dollar, atom: Atom<T> | Derived<T>): Promise<T>;
export declare function update<T>($: Dollar, atom: Atom<T>, fn: (value: T) => T): Promise<T>;
export declare function memberOf<T>(atom: Atom<Record<string,T>>, id: string): Atom<T>;
declare global {
  interface AbortSignal { readonly aborted: boolean }
  class AbortController { readonly signal: AbortSignal; abort(reason?: unknown): void }
  class TextEncoder { encode(text?: string): Uint8Array }
  class TextDecoder { constructor(encoding?: string); decode(bytes?: ArrayBuffer | ArrayBufferView): string }
  class URL { constructor(value: string, base?: string); readonly href: string; readonly origin: string; readonly hostname: string; readonly pathname: string; toString(): string }
  const crypto: {randomUUID(): string; subtle: {digest(algorithm: string, bytes: ArrayBuffer | ArrayBufferView): Promise<ArrayBuffer>}};
  const performance: {now(): number};
}
