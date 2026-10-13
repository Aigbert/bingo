import { stdout } from 'node:process';
import { Hooks } from './on.ts';
import type { Register } from './on.ts';
import { dollar } from './dollar.ts';
import { Peer, stdio } from './rpc.ts';
import type { InitializeParams, InitializeResult, ToolSpec } from './wire.ts';

// Older protocol-2 hosts advertise only their finite set of points.
function points(capabilities: readonly string[]): string[] {
  return capabilities.filter(name => name.startsWith('intercept/') && name !== 'intercept/cancel'
    && !name.endsWith('/after')).map(name => name.slice('intercept/'.length));
}

export function runtime(peer: Peer, register: Register) {
  const hooks = new Hooks();
  let negotiated = new Set<string>();
  const rpc = {
    request: <T>(method: string, params: unknown): Promise<T> => {
      if (!negotiated.has(method)) return Promise.reject(new Error(`capability unavailable: ${method}`));
      return peer.request<T>(method, params);
    },
    notify: (method: string, params: unknown) => {
      if (!negotiated.has(method)) throw new Error(`capability unavailable: ${method}`);
      peer.notify(method, params);
    },
  };
  peer.handlers.set('initialize', async (params: InitializeParams): Promise<InitializeResult> => {
    if (params.protocol !== 2) throw new Error(`plugin protocol 2 required; host speaks ${params.protocol}`);
    await register(hooks.on, params.config);
    const observed = hooks.names(params.capabilities.filter(name => name.startsWith('observe/')).map(name => name.slice(8)));
    const hooked = hooks.names(points(params.capabilities));
    const catalogue = params.capabilities.includes('intercept/*');
    const capabilities = ['intercept/*', 'intercept/*/after', 'host/place', 'host/extend', 'host/deliver', 'host/state/get',
      'host/state/set', 'host/model/complete', 'service/call', 'tools/changed', 'intercept/cancel', 'hook/observe',
      ...observed.map(name => `observe/${name}`),
      ...hooked.flatMap(name => [`intercept/${name}`, `intercept/${name}/after`])]
      .filter(name => params.capabilities.includes(name));
    negotiated = new Set(capabilities);
    for (const name of hooked) {
      peer.handlers.set(`intercept/${name}`, request => hooks.before(name, request, dollar(rpc, request.site)));
      peer.handlers.set(`intercept/${name}/after`, request => hooks.after(request));
    }
    if (catalogue) peer.fallback = method => {
      if (!method.startsWith('intercept/') || method === 'intercept/cancel') return undefined;
      if (method.endsWith('/after')) return request => hooks.after(request);
      const event = method.slice('intercept/'.length);
      return request => hooks.before(event, request, dollar(rpc, request.site));
    };
    return { protocol: 2, name: params.pluginName, version: '0.1.0', capabilities,
      links: catalogue ? [] : [...hooked, ...observed].map(point => ({ point, after: true })),
      subscriptions: catalogue && hooks.entries.length ? [{ patterns: [...new Set(hooks.entries.map(entry => entry.pattern))], prefilter: null }] : [] } as InitializeResult;
  });
  peer.handlers.set('hook/observe', request => hooks.observe(request.event, request.payload, dollar(rpc, request.site)));
  peer.handlers.set('intercept/cancel', params => hooks.cancel(params.chain));
  return {
    close: () => hooks.close(),
    toolsChanged: (tools: ToolSpec[]) => rpc.notify('tools/changed', { tools }),
  };
}

export async function plugin(module: string | URL): Promise<void> {
  const author = await import(String(module));
  if (typeof author.register !== 'function') throw new Error('module must export register(on, options)');
  const peer = new Peer(message => stdout.write(`${JSON.stringify(message)}\n`));
  const running = runtime(peer, author.register);
  stdio(peer, running.close);
}
