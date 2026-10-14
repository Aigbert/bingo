import test from 'node:test';
import assert from 'node:assert/strict';
import { Handlers } from './ui.ts';

test('render handlers travel as owned tokens and fire on the current invocation', async () => {
  const handlers = new Handlers();
  const oldPort = { request: async () => { throw new Error('expired render invocation'); } } as any;
  const requests: any[] = [];
  const current = { request: async (method: string, params: any) => { requests.push({method, params}); return null; } } as any;
  const callback = async (value: string) => oldPort.request('host/state/set', {value});
  const tree = handlers.mint('counter', 'session', 'pane', 'first', {kind:'Input', onInput: callback}, oldPort);
  assert.equal(tree.onInput.name, 'counter:ui');
  assert.equal(typeof tree.onInput.args.handler, 'string');
  assert.equal(tree.onInput.args.event, 'input');
  const events: string[] = [];
  const dispatch = async (event: string, input: any, next: () => Promise<unknown>) => { events.push(event); return next(); };
  assert.deepEqual(await handlers.fire('session', {...tree.onInput.args, value:'typed'}, current, dispatch), {stale:false});
  assert.deepEqual(requests, [{method:'host/state/set', params:{value:'typed'}}]);
  assert.deepEqual(events, ['ui.input']);
  handlers.mint('counter', 'session', 'pane', 'second', {kind:'Text',children:['new']}, oldPort);
  assert.deepEqual(await handlers.fire('session', tree.onInput.args, current, dispatch), {stale:true});
});

test('handler tokens cannot cross sessions or change their event', async () => {
  const handlers = new Handlers();
  const port = {request:async () => null} as any;
  let calls = 0;
  const tree = handlers.mint('counter','one','pane','generation',{kind:'Button',onPress:()=>{calls++;}},port);
  const dispatch = async (_: string, __: any, next: () => Promise<unknown>) => next();
  assert.deepEqual(await handlers.fire('two',tree.onPress.args,port,dispatch),{stale:true});
  assert.deepEqual(await handlers.fire('one',{...tree.onPress.args,event:'select'},port,dispatch),{stale:true});
  assert.equal(calls,0);
});

test('Escape raises close in the mod and removes only its current closable pane', async () => {
  const handlers = new Handlers();
  const calls: any[] = [];
  const port = {request:async (_:string, params:any) => {calls.push(params);return {result:params.method==='panes' ? [{kind:'pane',id:'counter',closeOnEscape:true}] : null};}} as any;
  const events: any[] = [];
  const dispatch = async (event:string,input:any,next:()=>Promise<unknown>) => {events.push([event,input]);return next();};
  const input = {handler:'counter',event:'close'};
  assert.deepEqual(await handlers.fire('session',input,port,dispatch),{stale:false});
  assert.deepEqual(events,[['ui.press',input]]);
  assert.equal(calls.at(-1).method,'close');
  assert.deepEqual(calls.at(-1).params,{session:'session',id:'counter'});
  assert.deepEqual(await handlers.fire('session',{handler:'unknown',event:'close'},port,dispatch),{stale:true});
});

test('the Node runtime declares its own hidden UI handler command', async () => {
  const { Peer } = await import('./rpc.ts');
  const { runtime } = await import('./plugin.ts');
  let process: InstanceType<typeof Peer>;
  const host = new Peer(message => { void process.receive(message); });
  process = new Peer(message => { void host.receive(message); });
  const running = runtime(process, () => {});
  try {
    const initialized: any = await host.request('initialize', {protocol:2,pluginName:'counter',pluginRoot:'.',config:{},capabilities:['callback/run','host/ui/invalidate']});
    assert.deepEqual(initialized.commands, [{name:'counter:ui',aliases:[],hint:'',args:{kind:'none'},instant:true,hidden:true,family:'ui'}]);
  } finally {running.close();process.close();host.close();}
});

test('undefined status removes the same slot as null', async () => {
  const { ui } = await import('./ui.ts');
  const requests: any[] = [];
  const api = ui({request: async (method: string, params: any) => {requests.push({method, params});return null;}} as any, {session:'s'} as any);
  await api.status('failed');
  await api.status(undefined);
  await api.status(undefined, {key:'worker'});
  assert.deepEqual(requests.map(({params}) => [params.slot, params.view]), [
    ['status', {kind:'status',text:'failed',tone:'neutral',until:null}],
    ['status', null], ['worker', null],
  ]);
});

test('synchronous element references settle after the surface table operation', async () => {
  const {ui, resolveElements} = await import('./ui.ts');
  let answer: (reply: any) => void = () => {};
  const rpc = {request: () => new Promise(resolve => {answer = resolve;})} as any;
  const table = ui(rpc, {session:'s'} as any).resolve({surface:'terminal'});
  const {Box, Text} = table;
  assert.ok(Box, 'synchronous table destructuring must yield an element');
  let settled = false;
  const rendered = resolveElements({kind:Box,children:[{kind:Text,children:['hello']}]})
    .then(value => {settled = true;return value;});
  await Promise.resolve();
  assert.equal(settled, false);
  answer({result:[{kind:'Box',props:[]},{kind:'Text',props:[]}]});
  assert.deepEqual(await rendered, {kind:'Box',children:[{kind:'Text',children:['hello']}]});
  assert.deepEqual(await table, {Box:'Box',Text:'Text'});
});

test('denied resolution cannot publish an unresolved element', async () => {
  const {ui, resolveElements} = await import('./ui.ts');
  const rpc = {request: async () => {throw new Error('resolve denied');}} as any;
  const {Box} = ui(rpc, {session:'s'} as any).resolve({surface:'terminal'});
  await assert.rejects(resolveElements({kind:Box}), /resolve denied/);
});

test('the Node hook waits for an unawaited status publication', async () => {
  const {Peer} = await import('./rpc.ts');
  const {runtime} = await import('./plugin.ts');
  let process: InstanceType<typeof Peer>;
  const host = new Peer(message => {void process.receive(message);});
  process = new Peer(message => {void host.receive(message);});
  const running = runtime(process, on => {on('tool.call', $ => {void $.ui.status('queued');return {parts:[]};});});
  let release!: () => void;
  let started!: () => void;
  const ready = new Promise<void>(resolve => {started = resolve;});
  host.handlers.set('host/place', () => new Promise(resolve => {release = () => resolve(null);started();}));
  try {
    await host.request('initialize', {protocol:2,pluginName:'fixture',pluginRoot:'.',config:{},capabilities:['intercept/*','host/place']});
    let returned = false;
    const answer = host.request('intercept/tool.call', {chain:'c',input:{tool:'Example',tool_use_id:'1'},site:{session:'s'},tier:'rewrite'}).then(value => {returned = true;return value;});
    await ready;
    assert.equal(returned,false);
    release();
    assert.deepEqual(await answer,{answer:{parts:[]}});
  } finally {running.close();process.close();host.close();}
});

test('root onKey mints an owned press handler and preserves unhandled', async () => {
  const handlers = new Handlers();
  const port = {request:async () => null} as any;
  const dispatch = async (_event: string, _input: any, next: () => Promise<unknown>) => next();
  const tree = handlers.mint('keys','session','pane','one',
    {kind:'Box',children:[],onKey:(event: any) => ({handled:event.key === 'x'})},port);
  assert.equal(tree.onKey.name, 'keys:ui');
  for (const key of ['x','y']) {
    assert.deepEqual(await handlers.fire('session',{...tree.onKey.args,value:{key}},port,dispatch),
      {stale:false,handled:key === 'x'});
  }
});
