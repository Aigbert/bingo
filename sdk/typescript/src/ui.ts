import type { Rpc } from './rpc.ts';
type Port = Rpc;
type Handler = { session: string; site: string; event: string; callback: (value: unknown) => unknown; port: Port };
const events: Record<string, string> = {onPress:'press', onInput:'input', onSubmit:'input', onSelect:'select'};

export class Handlers {
  private readonly tokens = new Map<string, Handler>();
  private readonly generations = new Map<string, string>();
  private readonly pending = new Map<Port, Promise<unknown>>();
  private serial = 0;
  private readonly epoch = crypto.randomUUID();
  mint(plugin: string, session: string, request: string, generation: string, view: any, port: Port): any {
    const site = JSON.stringify([session, request]);
    if (this.generations.get(site) !== generation) {
      for (const [token, handler] of this.tokens) if (handler.site === site) this.tokens.delete(token);
      this.generations.set(site, generation);
    }
    const visit = (value: any): any => {
      if (Array.isArray(value)) return value.map(visit);
      if (!value || typeof value !== 'object') return value;
      return Object.fromEntries(Object.entries(value).map(([key, child]) => {
        if (events[key] && typeof child === 'function') {
          const token = `${this.epoch}-${generation}:${++this.serial}`;
          this.tokens.set(token, {session, site, event:events[key], callback:child as Handler['callback'], port});
          return [key, {name:`${plugin}:ui`, args:{handler:token,event:events[key]}}];
        }
        return [key, visit(child)];
      }));
    };
    return visit(view);
  }
  async fire(session: string, input: any, port: Port, dispatch: (event: string, input: any, next: () => Promise<unknown>) => Promise<unknown>): Promise<any> {
    const handler = this.tokens.get(input?.handler);
    if (!handler || handler.session !== session || handler.event !== input.event) return {stale:true};
    const previous = this.pending.get(handler.port) ?? Promise.resolve();
    const run = previous.catch(() => {}).then(async () => {
      if (this.tokens.get(input.handler) !== handler) return {stale:true};
      const old = handler.port.request;
      handler.port.request = port.request;
      try {
        await dispatch(`ui.${handler.event}`, input, async () => handler.callback(input.value));
        return {stale:false};
      } finally { handler.port.request = old; }
    });
    this.pending.set(handler.port, run);
    try { return await run; }
    finally { if (this.pending.get(handler.port) === run) this.pending.delete(handler.port); }
  }
}
