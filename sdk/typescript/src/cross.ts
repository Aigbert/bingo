import { Hooks as Registry, patternMatches } from './on.ts';
type Before = { chain: string; input: any; site?: any; tier?: string };
type Pending = {
  controller: AbortController;
  resume: (value: any) => void;
  reject: (error: Error) => void;
  result: Promise<any>;
};

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

export class Hooks extends Registry {
  pending = new Map<string, Pending>();
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

  async observe(name: string, payload: unknown, dollar: unknown): Promise<void> {
    const entries = this.entries.filter(entry => patternMatches(entry.pattern, name));
    await this.dispatch(entries, 0, payload, dollar, undefined, new AbortController().signal, async () => null);
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
