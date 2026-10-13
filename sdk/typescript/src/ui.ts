import type { Rpc } from './rpc.ts';
export class Handlers {
  mint(plugin: string, session: string, request: string, generation: string, view: any, port: Rpc): any {
    return view;
  }
  async fire(session: string, input: any, port: Rpc, dispatch: (event: string, input: any, next: () => Promise<unknown>) => Promise<unknown>): Promise<any> {
    return { stale: true };
  }
}
