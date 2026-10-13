import type { Dollar } from './dollar.ts';

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
    return await handler(api, input) ?? null;
  }
}
