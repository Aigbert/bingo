import {register} from './parity-entry.ts';
import {test, expect, mock} from 'bingo/testing';
const readFile = {steps:[{toolCall:{name:'Read',input:{file_path:'plugin.json'}}}]};
const done = {steps:[{text:'Finished.'}]};

test('the band counts each turn independently and hides on press', async $ => {
  await mock.session({cwd:$.plugin.root,model:{responses:[readFile,readFile,done,readFile,done]}});
  await mock.session.submit('Read twice');
  const band = await $.ui.mount('AbovePrompt',{});
  expect(await band.drawn()).toContain('2 tool calls');
  await mock.session.submit('Read once');
  expect(await band.drawn()).toContain('1 tool calls');
  await band.press('Hide');
  expect(await band.drawn()).not.toContain('tool calls');
});

test('only a turn beyond two minutes publishes a toast', async $ => {
  const hooks = new Map<string, any>();
  register(((name: string, ...args: any[]) => {
    hooks.set(name, args[args.length - 1]);
    return {catch() {}};
  }) as any);
  const pending: Promise<unknown>[] = [];
  const api = new Proxy($, {get:(target, noun) => noun === 'ui' ? {
    ...target.ui, toast:(...args:any[]) => {
      const call = (target.ui.toast as any)(...args); pending.push(call); return call;
    }
  } : target[noun]});
  await mock.clock.set('2026-01-01T00:00:00Z');
  await hooks.get('prompt.submit')(api, {}, async () => null);
  await mock.clock.advance(120000);
  await hooks.get('turn.complete')(api, {}, async () => null);
  const toasts = async () => Object.values((await mock.session.snapshot() as any).signals?.[$.plugin.name] ?? {})
    .flatMap((value: any) => value.payload.slots ?? []).filter((slot:any) => slot.kind === 'toast');
  expect((await toasts()).length).toBe(0);
  await mock.clock.advance(1000);
  await hooks.get('turn.complete')(api, {}, async () => null);
  await Promise.all(pending);
  expect((await toasts()).length).toBe(1);
  expect((await toasts())[0].text).toContain('121');
});
