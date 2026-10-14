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
    if (input?.event === 'close') {
      const call = async (method: string, params: unknown) => (await port.request<{result:any}>('service/call', {key:'bingo.plugin.ui',method,params})).result;
      const panes = await call('panes', {session});
      if (!panes.some((pane: any) => pane.kind === 'pane' && pane.id === input.handler && pane.closeOnEscape)) return {stale:true};
      await dispatch('ui.press', input, () => call('close', {session,id:input.handler}));
      return {stale:false};
    }
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

import type { HookSite, Tone, Level, InteractionKind, AnswerSpec, Answer, Slot, SurfaceName, PanePlace, PaneSize } from './wire.ts';
export type OpenPane = {id: string; title: string; focus?: boolean; closeOnEscape?: boolean; holdToasts?: boolean; rows?: number; place?: PanePlace[]; size?: PaneSize; props?: unknown};
export function ui(rpc: Rpc, site: HookSite) {
  const call = <T>(method: string, params: unknown): Promise<T> =>
    rpc.request<{result:T}>('service/call', {key:'bingo.plugin.ui', method, params}).then(reply => reply.result);
  const toast = (method: string, text: string, options: {duration?:number} = {}) =>
    call<null>(method, {session:site.session,text,...options});
  return {
    status: (text: string | null, options: {key?:string; tone?:Tone} = {}) =>
      rpc.request<null>('host/place', {session:site.session,slot:options.key ?? 'status',
        view:text === null ? null : {kind:'status',text,tone:options.tone ?? 'neutral',until:null}}),
    toast: (text: string, options?: {duration?:number}) => toast('toast', text, options),
    notify: (text: string, options?: {duration?:number}) => toast('notify', text, options),
    open: (options: OpenPane) => call<Record<string,never>>('open', {...options,session:site.session}),
    close: (id: string) => call<null>('close', {session:site.session,id}),
    panes: () => call<Slot[]>('panes', {session:site.session}),
    invalidate: (requests: string | string[]) => rpc.request<null>('host/ui/invalidate',
      {session:site.session,requests:typeof requests === 'string' ? [requests] : requests}),
    resolve: async (event: {surface:SurfaceName}) => {
      const table = await call<{kind:string;props:string[]}[]>('resolve', {surface:event.surface});
      return Object.fromEntries(table.map(element => [element.kind, element.kind])) as Record<string,string>;
    },
    notice: (text: string, options: {level?:Level} = {}) => call<null>('notice', {text,...options}),
    ask: (kind: InteractionKind, answers: AnswerSpec[]) => call<Answer>('ask', {session:site.session,kind,answers}),
  };
}
