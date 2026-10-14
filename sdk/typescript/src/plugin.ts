import { stdout } from 'node:process';
import { Hooks } from './cross.ts';
import type { Register, Entry } from './on.ts';
import { patternMatches, renderMatcher } from './on.ts';
import { Callbacks } from './callbacks.ts';
import { dollar } from './dollar.ts';
import { Handlers } from './ui.ts';
import { dollarRoutes } from './dollar-types.ts';
import { Peer, stdio } from './rpc.ts';
import type { InitializeParams, InitializeResult, ToolSpec } from './wire.ts';

// Older protocol-2 hosts advertise only their finite set of points.
function points(capabilities: readonly string[]): string[] {
  return capabilities.filter(name => name.startsWith('intercept/') && name !== 'intercept/cancel'
    && !name.endsWith('/after')).map(name => name.slice('intercept/'.length));
}

export function runtime(peer: Peer, register: Register) {
  const hooks = new Hooks();
  const callbacks = new Callbacks();
  const handlers = new Handlers();
  const renderSites = new WeakMap<object, {request: any; port: any}>();
  const wrap = (hook: any) => async (api: any, input: any, next: any) => {
    const answer = await hook(api, input, next);
    const site = renderSites.get(api);
    return site && answer !== undefined ? handlers.mint(identity.name, site.request.site.session,
      site.request.input.requestId, site.request.chain, answer, site.port) : answer;
  };
  let negotiated = new Set<string>();
  let pluginName = "";
  let identity: import('./dollar-types.ts').PluginData = Object.freeze({ name: "", root: "" });
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
  const invocation = (request: any) => {
    const callers = peer.context()?.callers ?? [];
    return {
      skip: callers.filter(caller => caller.plugin === pluginName).map(caller => caller.hook),
      dollarFor: (entry: Entry) => {
        const hook = hooks.entries.indexOf(entry);
        const context = { parent: request.chain, hook, callers: [...callers, { plugin: pluginName, hook }] };
        const port = { ...rpc, request: <T>(method: string, params: unknown) =>
          peer.withContext(context, () => rpc.request<T>(method, params)) };
        const api = dollar(port, request.site, identity, callbacks);
        if (request.event === 'ui.render') renderSites.set(api, {request, port});
        return api;
      },
    };
  };
  const before = (event: string, request: any) => {
    const { dollarFor, skip } = invocation({...request, event});
    return hooks.before(event, request, undefined, dollarFor, skip);
  };
  peer.handlers.set('initialize', async (params: InitializeParams): Promise<InitializeResult> => {
    if (params.protocol !== 2) throw new Error(`plugin protocol 2 required; host speaks ${params.protocol}`);
    pluginName = params.pluginName;
    identity = Object.freeze({ name: params.pluginName, root: params.pluginRoot, ...(params.pluginVersion === undefined ? {} : {version:params.pluginVersion}) });
    await register(hooks.on, params.config);
    for (const entry of hooks.entries) {
      entry.hook = wrap(entry.hook);
      if (entry.recover) entry.recover = wrap(entry.recover);
    }
    const observed = hooks.names(params.capabilities.filter(name => name.startsWith('observe/')).map(name => name.slice(8)));
    const hooked = hooks.names(points(params.capabilities));
    const catalogue = params.capabilities.includes('intercept/*');
    const capabilities = [
      ...Object.entries(dollarRoutes).filter(([, route]) => 'kernel' in route).map(([name]) => `host/${name.replace('.', '/')}`),'callback/run', 'callback/start', 'intercept/*', 'intercept/*/after', 'host/place', 'host/extend', 'host/deliver', 'host/state/get',
      'host/state/set', 'host/ui/invalidate', 'host/model/complete', 'service/call', 'tools/changed', 'intercept/cancel', 'hook/observe',
      ...observed.map(name => `observe/${name}`),
      ...hooked.flatMap(name => [`intercept/${name}`, `intercept/${name}/after`])]
      .filter(name => params.capabilities.includes(name));
    negotiated = new Set(capabilities);
    for (const name of hooked) {
      peer.handlers.set(`intercept/${name}`, request => before(name, request));
      peer.handlers.set(`intercept/${name}/after`, request => hooks.after(request));
    }
    if (catalogue) peer.fallback = method => {
      if (!method.startsWith('intercept/') || method === 'intercept/cancel') return undefined;
      if (method.endsWith('/after')) return request => hooks.after(request);
      const event = method.slice('intercept/'.length);
      return request => before(event, request);
    };
    return { protocol: 2, name: params.pluginName, version: '0.1.0', capabilities,
      commands: negotiated.has('callback/run') ? [{name:`${params.pluginName}:ui`,aliases:[],hint:'',args:{kind:'none'},instant:true,hidden:true,family:'ui'}] : [],
      links: catalogue ? [] : [...hooked, ...observed].map(point => ({ point, after: true })),
      subscriptions: catalogue && hooks.entries.length ? [{ patterns: [...new Set(hooks.entries.map(entry => entry.pattern))], prefilter: null, render: hooks.entries.filter(entry => patternMatches(entry.pattern, "ui.render")).map(entry => renderMatcher(entry.matcher)) }] : [] } as InitializeResult;
  });
  peer.handlers.set('hook/observe', request => {
    const { dollarFor, skip } = invocation(request);
    return hooks.observe(request.event, request.payload, undefined, dollarFor, skip);
  });
  peer.handlers.set('callback/start', async site => {
    const { dollarFor, skip } = invocation({site});
    await hooks.observe('session.start', {session:site.session}, undefined, dollarFor, skip);
    return null;
  });
  peer.handlers.set('callback/run', request => {
    const context = {parent:request.chain, callers:peer.context()?.callers ?? []};
    const port = {...rpc, request:<T>(method:string, params:unknown) =>
      peer.withContext(context, () => rpc.request<T>(method, params))};
    const api = dollar(port, request.site, identity, callbacks);
    if (request.callback === '__ui__') {
      return handlers.fire(request.site.session, request.input, port, (event, input, next) =>
        hooks.dispatch(hooks.entries.filter(entry => patternMatches(entry.pattern, event)),
          0, input, api, request.site, new AbortController().signal, next));
    }
    return callbacks.run(request.callback, api, request.input);
  });
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
