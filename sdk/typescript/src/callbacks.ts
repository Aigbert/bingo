import type { Dollar } from './dollar.ts';
import type { OpenPane } from './ui.ts';

export type Callback = (api: Dollar, input: unknown) => unknown | Promise<unknown>;
export class Callbacks {
  private readonly handlers = new Map<string, Callback>();
  private serial = 0;
  put(kind: string, handler: Callback, name?: string): string {
    if (typeof handler !== 'function') throw new Error(`${kind} requires a callback`);
    const id = `${kind}/${name ?? ++this.serial}`;
    this.handlers.set(id, handler);
    return id;
  }
  async run(id: string, api: Dollar, input: unknown): Promise<unknown> {
    const handler = this.handlers.get(id);
    if (!handler) throw new Error(`callback ${id} is unavailable in this generation`);
    return await (id.startsWith('command/') ? command(handler, api, input) : handler(api, input)) ?? null;
  }
}

// A receipt belongs to one command call; a live publication carries no request.
async function command(handler: Callback, api: Dollar, input: unknown): Promise<unknown> {
  let opened: OpenPane | undefined;
  const ui = {...api.ui, open: async (options: OpenPane) => {
    const result = await api.ui.open(options);
    if (options.focus !== false) opened = options;
    return result;
  }};
  const scoped = new Proxy(api, {get: (target, key) => key === 'ui' ? ui : Reflect.get(target, key)});
  const result = await handler(scoped, input);
  if (!opened || (result != null && (typeof result !== 'object' || !('kind' in result) || result.kind !== 'applied'))) return result;
  const message = result != null && typeof result === 'object' && 'message' in result ? result.message : undefined;
  return {kind:'surface', action:{name:'Pane',args:{plugin:api.plugin.name,id:opened.id}},
    fallback:typeof message === 'string' ? message : `Opened ${opened.title}.`};
}
