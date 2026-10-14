export type { Register, On, Hook, Next } from './on.ts';
export type { Dollar, WithServices } from './dollar.ts';
export type * from './events.ts';
export { h, Fragment } from './elements';
export type { Element } from './elements';
import type { Dollar } from './dollar.ts';
export interface PluginState {}
export {atom, derive, read, update, memberOf} from './state.ts';
export type {Atom, Derived} from './state.ts';
declare global {
  interface AbortSignal { readonly aborted: boolean }
  class AbortController { readonly signal: AbortSignal; abort(reason?: unknown): void }
  class TextEncoder { encode(text?: string): Uint8Array }
  class TextDecoder { constructor(encoding?: string); decode(bytes?: ArrayBuffer | ArrayBufferView): string }
  class URL { constructor(value: string, base?: string); readonly href: string; readonly origin: string; readonly hostname: string; readonly pathname: string; toString(): string }
  const crypto: {randomUUID(): string; subtle: {digest(algorithm: string, bytes: ArrayBuffer | ArrayBufferView): Promise<ArrayBuffer>}};
  const performance: {now(): number};
}
