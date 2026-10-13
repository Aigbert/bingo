import type { Dollar } from './dollar.ts';

// Transport-independent matching and hook composition; the catalogue may grow.
export type Matcher = unknown;
export type Next = ((event: any) => Promise<any>) & {
  readonly called: boolean;
  readonly signal: AbortSignal;
  readonly origin: unknown;
};
export type Hook = ($: Dollar, event: any, next: Next) => any;
export type On = {
  (pattern: string, hook: Hook): void;
  (pattern: string, matcher: Matcher, hook: Hook): void;
};
export type Register = (on: On, options: unknown) => void | Promise<void>;

type Entry = { pattern: string; matcher: Matcher; hook: Hook };
type Before = { chain: string; input: any; site?: any; tier?: string };
type Pending = {
  controller: AbortController;
  resume: (value: any) => void;
  reject: (error: Error) => void;
  result: Promise<any>;
};

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

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

export class Hooks {
  entries: Entry[] = [];
  pending = new Map<string, Pending>();
  on: On = (pattern: string, matcher: Matcher | Hook, hook?: Hook) => {
    this.entries.push({ pattern, matcher: hook ? matcher : undefined, hook: hook ?? matcher as Hook });
  };

  names(catalogue: readonly string[]): string[] {
    return catalogue.filter(name => this.entries.some(entry => patternMatches(entry.pattern, name)));
  }

  async before(name: string, request: Before, dollar: unknown): Promise<unknown> {
    if (this.pending.has(request.chain)) throw new Error('duplicate chain');
    const first = deferred<unknown>();
    const downstream = deferred<any>();
    const controller = new AbortController();
    const stopped = deferred<never>();
    // Cancellation can arrive before any hook has called next.
    void downstream.promise.catch(() => {});
    let crossed = false;
    const entries = this.entries.filter(entry => patternMatches(entry.pattern, name));
    const terminal = (input: unknown) => {
      crossed = true;
      first.resolve({ next: input });
      return downstream.promise;
    };
    const result = Promise.race([
      Promise.resolve().then(() => this.dispatch(entries, 0, request.input, dollar, request.site, controller.signal, terminal)),
      stopped.promise,
    ]);
    const reject = (error: Error) => {
      first.reject(error);
      downstream.reject(error);
      stopped.reject(error);
    };
    this.pending.set(request.chain, { controller, resume: downstream.resolve, reject, result });
    void result.then(answer => {
      if (!crossed) first.resolve({ answer: answer ?? null });
    }, error => { if (!crossed) first.reject(error); }).finally(() => {
      if (!crossed) this.pending.delete(request.chain);
    });
    return first.promise;
  }

  private async dispatch(entries: Entry[], index: number, input: any, dollar: unknown,
    origin: unknown, signal: AbortSignal, terminal: (input: any) => Promise<any>): Promise<any> {
    if (signal.aborted) throw new Error('intercept cancelled');
    const entry = entries[index];
    if (!entry) return terminal(input);
    if (entry.matcher !== undefined && !matches(entry.matcher, input)) return this.dispatch(entries, index + 1, input, dollar, origin, signal, terminal);
    let called = false;
    let continuation: Promise<any> | undefined;
    const next = Object.assign(async (changed: unknown) => {
      if (called) throw new Error('next may be called only once');
      if (signal.aborted) throw new Error('intercept cancelled');
      called = true;
      continuation = this.dispatch(entries, index + 1, changed, dollar, origin, signal, terminal);
      return continuation;
    }, { signal, origin }) as Next;
    Object.defineProperty(next, 'called', { get: () => called });
    const answer = await entry.hook(dollar as Dollar, frozen(input), next);
    // A fire-and-forget next still owns the downstream result and its lifetime.
    return answer === undefined && continuation ? await continuation : answer;
  }

  async after(request: { chain: string; result: unknown }): Promise<unknown> {
    const pending = this.pending.get(request.chain);
    if (!pending) throw new Error('unknown intercept chain');
    pending.resume(request.result);
    try { return { result: await pending.result }; }
    finally { this.pending.delete(request.chain); }
  }

  cancel(chain: string): void {
    const pending = this.pending.get(chain);
    if (!pending) return;
    this.pending.delete(chain);
    pending.controller.abort();
    pending.reject(new Error('intercept cancelled'));
  }

  close(): void {
    for (const chain of this.pending.keys()) this.cancel(chain);
  }
}
