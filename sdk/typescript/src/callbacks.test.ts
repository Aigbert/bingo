import {test} from 'node:test';
import assert from 'node:assert/strict';
import {Callbacks} from './callbacks.ts';
import {dollar} from './dollar.ts';

test('only a command opener returns a one-shot Pane receipt', async () => {
  const callbacks = new Callbacks();
  const rpc = {request: async () => ({result:{}})} as any;
  const api = dollar(rpc, {session:'s'} as any, {name:'fixture',root:'.'});
  const open = async ($: typeof api) => {
    await $.ui.open({id:'keys',title:'Keys'});
    return {kind:'applied',message:'Opened keys'};
  };
  const command = callbacks.put('command', open, 'open-keys');
  const tick = callbacks.put('clock', open);
  for (const id of [command, tick, command]) {
    assert.deepEqual(await callbacks.run(id, api, ''), id === command
      ? {kind:'surface',action:{name:'Pane',args:{plugin:'fixture',id:'keys'}},fallback:'Opened keys'}
      : {kind:'applied',message:'Opened keys'});
  }
  const passive = callbacks.put('command', async $ => {
    await $.ui.open({id:'art',title:'Art',focus:false});
    return {kind:'applied'};
  });
  assert.deepEqual(await callbacks.run(passive, api, ''), {kind:'applied'});
  const explicit = {kind:'surface',action:{name:'Composer',args:{text:'draft'}},fallback:''};
  const fill = callbacks.put('command', async $ => { await open($); return explicit; });
  assert.deepEqual(await callbacks.run(fill, api, ''), explicit);
  const empty = callbacks.put('command', () => ({kind:'applied'}));
  assert.deepEqual(await callbacks.run(empty, api, ''), {kind:'applied'});
});
