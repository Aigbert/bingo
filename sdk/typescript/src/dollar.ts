import type { Rpc } from './rpc.ts';
import type { HookSite, StateValue, ModelRequest, Completed, Input, Tone } from './wire.ts';

export function dollar(rpc: Rpc, site: HookSite) {
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
  return new Proxy(api, {
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
