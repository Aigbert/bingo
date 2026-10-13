import type { EventInputs, EventResults } from './events.ts';
import type { Dollar } from './dollar.ts';

// Transport-independent matching and hook composition; the catalogue may grow.
export type Matcher = unknown;
export type Next = ((event: any) => Promise<any>) & {
  readonly called: boolean;
  readonly signal: AbortSignal;
  readonly origin: unknown;
  readonly error?: { kind: string; message: string };
  readonly budget?: { ms: number; remainingMs: number };
};
export type Hook = ($: Dollar, event: any, next: Next) => any;
export type EventNext<N extends keyof EventInputs> = ((event: EventInputs[N]) => Promise<EventResults[N]>) & Omit<Next, keyof Function>;
export type EventHook<N extends keyof EventInputs> = ($: Dollar, event: EventInputs[N], next: EventNext<N>) => EventResults[N] | void | Promise<EventResults[N] | void>;
export type On = {
  <N extends keyof EventInputs>(pattern: N, hook: EventHook<N>): Registration;
  <N extends keyof EventInputs>(pattern: N, matcher: Partial<EventInputs[N]>, hook: EventHook<N>): Registration;
  (pattern: string, hook: Hook): Registration;
  (pattern: string, matcher: Matcher, hook: Hook): Registration;
};
export type Register = (on: On, options: unknown) => void | Promise<void>;

export type Registration = { catch(hook: Hook): void };
export type Entry = { pattern: string; matcher: Matcher; hook: Hook; recover?: Hook };
export function patternMatches(pattern: string, name: string): boolean {
  if (pattern.startsWith('!')) return !patternMatches(pattern.slice(1), name);
  if (pattern === '*') return true;
  if (pattern.endsWith('.*')) return name.startsWith(pattern.slice(0, -1));
  return pattern === name;
}

export function matches(matcher: Matcher, event: unknown): boolean {
  if (matcher instanceof RegExp) {
    return typeof event === 'string' && new RegExp(matcher.source, matcher.flags).test(event);
  }
  if (Array.isArray(matcher)) return matcher.some(value => matches(value, event));
  if (matcher !== null && typeof matcher === 'object') {
    return event !== null && typeof event === 'object'
      && Object.entries(matcher).every(([key, value]) => matches(value, (event as Record<string, unknown>)[key]));
  }
  return Object.is(matcher, event);
}

function frozen<T>(value: T): T {
  if (value !== null && typeof value === 'object') {
    Object.values(value).forEach(frozen);
    Object.freeze(value);
  }
  return value;
}

export class Hooks {
  entries: Entry[] = [];
  on: On = (pattern: string, matcher: Matcher | Hook, hook?: Hook) => {
    const entry: Entry = { pattern, matcher: hook ? matcher : undefined, hook: hook ?? matcher as Hook };
    const key = JSON.stringify([pattern, entry.matcher], (_, value) => value instanceof RegExp ? { regex: value.source, flags: value.flags } : value);
    if (this.entries.some(held => JSON.stringify([held.pattern, held.matcher], (_, value) => value instanceof RegExp ? { regex: value.source, flags: value.flags } : value) === key)) throw new Error('duplicate hook registration');
    this.entries.push(entry);
    return { catch: (recover: Hook) => {
      if (entry.recover) throw new Error('catch may be registered only once');
      entry.recover = recover;
    } };
  };

  names(catalogue: readonly string[]): string[] {
    return catalogue.filter(name => this.entries.some(entry => patternMatches(entry.pattern, name)));
  }

  async dispatch(entries: Entry[], index: number, input: any, dollar: unknown,
    origin: unknown, signal: AbortSignal, terminal: (input: any) => Promise<any>, dollarFor?: (entry: Entry) => unknown): Promise<any> {
    if (signal.aborted) throw new Error('intercept cancelled');
    const entry = entries[index];
    if (!entry) return terminal(input);
    if (entry.matcher !== undefined && !matches(entry.matcher, input)) return this.dispatch(entries, index + 1, input, dollar, origin, signal, terminal, dollarFor);
    let called = false;
    let continuation: Promise<any> | undefined;
    const next = Object.assign(async (changed: unknown) => {
      if (called) throw new Error('next may be called only once');
      if (signal.aborted) throw new Error('intercept cancelled');
      called = true;
      continuation = this.dispatch(entries, index + 1, changed, dollar, origin, signal, terminal, dollarFor);
      return continuation;
    }, { signal, origin }) as Next;
    Object.defineProperty(next, 'called', { get: () => called });
    let answer;
    try { answer = await entry.hook((dollarFor?.(entry) ?? dollar) as Dollar, frozen(input), next); }
    catch (error) {
      if (!entry.recover) throw error;
      Object.defineProperty(next, 'error', { value: { kind: 'throw', message: String(error) } });
      answer = await entry.recover((dollarFor?.(entry) ?? dollar) as Dollar, frozen(input), next);
    }
    // A fire-and-forget next still owns the downstream result and its lifetime.
    return answer === undefined && continuation ? await continuation : answer;
  }

}
