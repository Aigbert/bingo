import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Peer } from './rpc.ts';
import { runtime } from './plugin.ts';

test('registered callbacks receive a fresh dollar and frozen plugin identity', async () => {
  let plugin: Peer;
  const host = new Peer(message => { void plugin.receive(message); });
  plugin = new Peer(message => { void host.receive(message); });
  const running = runtime(plugin, on => {
    on('session.start', async $ => {
      await $.tool.register({name:'hello',description:'fixture',inputSchema:{type:'object'}},
        async (api, input) => ({plugin:api.plugin, input, frozen:Object.isFrozen(api.plugin)}));
    });
  });
  let callback = '';
  host.handlers.set('service/call', params => {
    assert.equal(params.key, 'bingo.plugin.tool');
    assert.equal(params.method, 'register');
    callback = params.params.callback;
    return {result:'plugin__fixture__hello'};
  });
  try {
    await host.request('initialize', {protocol:2,pluginName:'fixture',pluginRoot:'/fixture',config:{},
      capabilities:['intercept/*','hook/observe','callback/run','callback/start','service/call']});
    const site = {session:'s',cwd:'/work'};
    await host.request('hook/observe', {event:'session.start',payload:{session:'s'},site});
    assert.equal(callback, 'tool/hello');
    callback = '';
    await host.request('callback/start', site);
    assert.equal(callback, 'tool/hello');
    assert.deepEqual(await host.request('callback/run',{chain:'callback-1',callback,input:{v:1},site}),
      {plugin:{name:'fixture',root:'/fixture'},input:{v:1},frozen:true});
  } finally {running.close(); plugin.close(); host.close();}
});

test('a command descriptor can register without a callback', async () => {
  const {dollar} = await import('./dollar.ts');
  const {Callbacks} = await import('./callbacks.ts');
  const callbacks = new Callbacks();
  const calls: any[] = [];
  const rpc = {request: async (_: string, params: any) => {calls.push(params);return {result:'inspect'};}} as any;
  const api = dollar(rpc, {session:'s'} as any, {name:'fixture',root:'.'}, callbacks);
  await api.command.register({name:'inspect',description:'Inspect activity'});
  assert.deepEqual(calls[0].params.spec, {name:'inspect',aliases:[],hint:'Inspect activity',args:{kind:'none'},instant:false,family:'plugin'});
  assert.deepEqual(await callbacks.run(calls[0].params.callback, api, ''), {kind:'applied'});
  assert.throws(() => (api.tool.register as any)({name:'missing'}), /callback/);
});
