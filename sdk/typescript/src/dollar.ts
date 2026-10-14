import type { Callbacks, Callback } from './callbacks.ts';
import { ui } from './ui.ts';
import { dollarRoutes } from './dollar-types.ts';
import type { ProcessOutput, DollarInputs, DollarResults, DollarData } from './dollar-types.ts';
import type { Rpc } from './rpc.ts';
import type { HookSite, StateValue, ModelRequest, Completed, Input, ToolSpec, CommandSpec } from './wire.ts';

export function dollar(rpc: Rpc, site: HookSite, identity: DollarData['plugin'] = { name: '', root: '' }, callbacks?: Callbacks): Dollar {
  const store = (scope: string) => ({
    get: (key: string) => rpc.request<StateValue | null>('host/state/get', { scope, key }),
    set: (key: string, value: unknown, ifVersion?: number) =>
      rpc.request<StateValue>('host/state/set', { scope, key, value, ifVersion }),
  });
  const api = {
    state: store(site.session),
    store: store('global'),
    ui: ui(rpc, site),
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
  for (const method of ['after', 'every']) {
    const schedule = hostApi.clock[method];
    hostApi.clock[method] = (ms: number, handler: Callback) => {
      if (!callbacks) throw new Error('callback runtime is unavailable');
      return schedule({ms,callback:callbacks.put('clock',handler)});
    };
  }
  const sleep = hostApi.clock.sleep;
  hostApi.clock.sleep = (ms: number) => sleep({ms});
  const deleteKey = hostApi.store.delete;
  hostApi.store.delete = (key: string) => deleteKey({scope:'global',key});
  for (const noun of ['tool', 'command']) {
    const register = hostApi[noun].register;
    hostApi[noun].register = (spec: { name: string }, handler: Callback) => {
      if (!callbacks) throw new Error('callback runtime is unavailable');
      return register({ spec, callback: callbacks.put(noun, handler, spec.name) });
    };
  }
  const spawn = hostApi.process.spawn;
  hostApi.process.spawn = async (input: any) => {
    const id = await spawn(input);
    const session = input?.session ?? site.session;
    return Object.freeze({
      id,
      read: () => hostApi.process.read({session,id}),
      write: (body: string | null) => hostApi.process.write({session,id,body}),
      kill: () => hostApi.process.kill({session,id}),
    });
  };
  Object.defineProperty(hostApi, 'plugin', { value: Object.freeze({ ...identity }), enumerable: true });
  return new Proxy(hostApi, {
    get(target, noun: string) {
      if (Object.hasOwn(target, noun)) return target[noun as keyof typeof target];
      if (noun === 'then' || typeof noun !== 'string') return undefined;
      return new Proxy({}, { get: (_, method: string) => (params: unknown) =>
        rpc.request<{ result: unknown }>('service/call', { key: noun, method, params }).then(reply => reply.result) });
    },
  }) as Dollar;
}

type Operation = keyof DollarInputs;
type Noun = Operation extends `${infer N}.${string}` ? N : never;
type AtSite<I> = I extends object ? Omit<I, 'session' | 'turn'> & Partial<Pick<I, Extract<keyof I, 'session' | 'turn'>>> : I;
type Method<K extends Operation> = AtSite<DollarInputs[K]> extends null
  ? () => Promise<DollarResults[K]>
  : {} extends AtSite<DollarInputs[K]>
    ? (input?: AtSite<DollarInputs[K]>) => Promise<DollarResults[K]>
    : (input: AtSite<DollarInputs[K]>) => Promise<DollarResults[K]>;
type HostNouns = { [N in Noun]: { [K in Operation as K extends `${N}.${infer M}` ? M : never]: Method<K> } };
type Replace<T, With> = Omit<T, keyof With> & With;
type State = {
  get(key: string): Promise<StateValue | null>;
  set(key: string, value: unknown, ifVersion?: number): Promise<StateValue>;
};
export type ProcessHandle = {
  readonly id: string;
  read(): Promise<ProcessOutput>;
  write(body: string | null): Promise<null>;
  kill(): Promise<null>;
};
export type Dollar = Replace<HostNouns, {
  process: Replace<HostNouns['process'], {spawn(input: AtSite<DollarInputs['process.spawn']>): Promise<ProcessHandle>}>;
  readonly plugin: Readonly<DollarData['plugin']>;
  state: State;
  store: State & {keys(): Promise<string[]>; delete(key: string): Promise<boolean>};
  model: Replace<HostNouns['model'], {complete(request: ModelRequest): Promise<Completed>}>;
  tool: Replace<HostNouns['tool'], {register(spec: ToolSpec, handler: Callback): Promise<string>}>;
  command: Replace<HostNouns['command'], {register(spec: CommandSpec, handler: Callback): Promise<string>}>;
  clock: {
    now(): Promise<string>;
    sleep(ms: number): Promise<null>;
    after(ms: number, handler: Callback): Promise<string>;
    every(ms: number, handler: Callback): Promise<string>;
  };
  session: HostNouns['session'] & {
    deliver(input: Input, wake?: boolean): Promise<null>;
    extend(key: string, value: unknown): Promise<null>;
  };
  ui: ReturnType<typeof ui>;
}>;
// A service face augments this type without widening the host's built-in nouns.
export type WithServices<Services> = Dollar & Services;
