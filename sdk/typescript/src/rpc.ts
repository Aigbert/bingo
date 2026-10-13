import { createInterface } from 'node:readline';
import { stdin, stdout, stderr } from 'node:process';

export type Handler = (params: any) => unknown | Promise<unknown>;
export type Rpc = Pick<Peer, 'request' | 'notify'>;

export class Peer {
  handlers = new Map<string, Handler>();
  pending = new Map<number, { resolve: (value: any) => void; reject: (error: Error) => void }>();
  private serial = 0;
  private send: (message: unknown) => void;
  private closed = false;

  constructor(send: (message: unknown) => void) { this.send = send; }

  request<T = unknown>(method: string, params: unknown): Promise<T> {
    if (this.closed) return Promise.reject(new Error('RPC connection closed'));
    const id = ++this.serial;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      try { this.send({ jsonrpc: '2.0', id, method, params }); }
      catch (error) { this.pending.delete(id); reject(error); }
    });
  }

  notify(method: string, params: unknown): void {
    if (this.closed) throw new Error('RPC connection closed');
    this.send({ jsonrpc: '2.0', method, params });
  }

  async receive(message: any): Promise<void> {
    if (typeof message?.method !== 'string') {
      const pending = this.pending.get(message?.id);
      if (!pending) return;
      this.pending.delete(message.id);
      if (message.error) pending.reject(Object.assign(new Error(message.error.message), message.error));
      else pending.resolve(message.result);
      return;
    }
    const handler = this.handlers.get(message.method);
    if (!Object.hasOwn(message, 'id')) {
      try { await handler?.(message.params); }
      catch (error) { stderr.write(`${String(error)}\n`); }
      return;
    }
    let response: object;
    try {
      if (!handler) throw Object.assign(new Error(`unknown method ${message.method}`), { code: -32601 });
      response = { result: await handler(message.params) ?? null };
    } catch (error: any) {
      response = { error: { code: error.code ?? -32603, message: String(error.message ?? error) } };
    }
    this.send({ jsonrpc: '2.0', id: message.id, ...response });
  }

  close(): void {
    this.closed = true;
    for (const pending of this.pending.values()) pending.reject(new Error('RPC connection closed'));
    this.pending.clear();
  }
}

export function stdio(peer: Peer, close: () => void): void {
  const lines = createInterface({ input: stdin, crlfDelay: Infinity });
  lines.on('line', line => {
    try { void peer.receive(JSON.parse(line)); }
    catch { stdout.write(`${JSON.stringify({ jsonrpc: '2.0', id: null, error: { code: -32700, message: 'invalid JSON' } })}\n`); }
  });
  lines.on('close', () => { close(); peer.close(); });
}
