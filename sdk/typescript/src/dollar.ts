import type { Callbacks, Callback } from './callbacks.ts';
import { dollarRoutes } from './dollar-types.ts';
import type { Rpc } from './rpc.ts';
import type { HookSite, StateValue, ModelRequest, Completed, Input, Tone } from './wire.ts';

export function dollar(rpc: Rpc, site: HookSite, identity: { name: string; root: string } = { name: '', root: '' }, callbacks?: Callbacks) {
  const store = (scope: string) => ({
    get: (key: string) => rpc.request<StateValue | null>('host/state/get', { scope, key }),
    set: (key: string, value: unknown, ifVersion?: number) =>
      rpc.request<StateValue>('host/state/set', { scope, key, value, ifVersion }),
  });
  const api = {
    state: store(site.session),
    store: store('global'),
    ui: {
      status: (text: string | null, options: { key?: string; tone?: Tone } = {}) =>
        rpc.request<null>('host/place', { session: site.session, slot: options.key ?? 'status',
          view: text === null ? null : { kind: 'status', text, tone: options.tone ?? 'neutral', until: null } }),
    },
    model: {
      complete: (request: ModelRequest) => rpc.request<Completed>('host/model/complete', {
        session: site.session, turn: site.turn, request,
      }),
    },
    session: {
      deliver: (input: Input, wake = true) => rpc.request<null>('host/deliver', { session: site.session, input, wake }),
      extend: (key: string, value: unknown) => rpc.request<null>('host/extend', { session: site.session, key, value }),
    },
  };
  const hostApi: Record<string, any> = api;
  for (const [name, route] of Object.entries(dollarRoutes)) {
    const [noun, method] = name.split('.');
    const object = hostApi[noun] ??= {};
    if (Object.hasOwn(object, method)) continue;
    object[method] = (input: any = null) => {
      const params = route.site.length ? { ...input } : input;
      for (const field of route.site) params[field] ??= site[field];
      if ('kernel' in route) return rpc.request(`host/${name.replace('.', '/')}`, params);
      return rpc.request<{ result: unknown }>('service/call', { key: route.face, method, params }).then(reply => reply.result);
    };
  }
  const deleteKey = hostApi.store.delete;
  hostApi.store.delete = (key: string) => deleteKey({scope:'global',key});
  for (const noun of ['tool', 'command']) {
    const register = hostApi[noun].register;
    hostApi[noun].register = (spec: { name: string }, handler: Callback) => {
      if (!callbacks) throw new Error('callback runtime is unavailable');
      return register({ spec, callback: callbacks.put(noun, handler, spec.name) });
    };
  }
  Object.defineProperty(hostApi, 'plugin', { value: Object.freeze({ ...identity }), enumerable: true });
  return new Proxy(hostApi, {
    get(target, noun: string) {
      if (Object.hasOwn(target, noun)) return target[noun as keyof typeof target];
      if (noun === 'then' || typeof noun !== 'string') return undefined;
      return new Proxy({}, { get: (_, method: string) => (params: unknown) =>
        rpc.request<{ result: unknown }>('service/call', { key: noun, method, params }).then(reply => reply.result) });
    },
  });
}

export type Dollar = ReturnType<typeof dollar>;
// A service face augments this type without widening the host's built-in nouns.
export type WithServices<Services> = Dollar & Services;
