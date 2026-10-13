import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Peer } from './rpc.ts';
import { runtime } from './plugin.ts';

test('RPC pairs ids, rejects unknown requests and closes all pending calls', async () => {
  const messages: any[] = [];
  const peer = new Peer(message => messages.push(message));
  const first = peer.request('one', {});
  const second = peer.request('two', {});
  await peer.receive({ jsonrpc: '2.0', id: 2, result: 'two' });
  await peer.receive({ jsonrpc: '2.0', id: 1, result: 'one' });
  assert.equal(await first, 'one');
  assert.equal(await second, 'two');
  await peer.receive({ jsonrpc: '2.0', id: 'host-1', method: 'unknown', params: {} });
  assert.equal(messages.at(-1).error.code, -32601);
  assert.equal(messages.at(-1).id, 'host-1');
  const pending = peer.request('never', {});
  peer.close();
  await assert.rejects(pending, /closed/);
  assert.equal(peer.pending.size, 0);
});

test('handshake advertises matching points and intersects host capabilities', async () => {
  const messages: any[] = [];
  const peer = new Peer(message => messages.push(message));
  const app = runtime(peer, on => on('!tool.*', ($, e, next) => next(e)));
  await peer.receive({ id: 1, method: 'initialize', params: {
    protocol: 2, pluginName: 'example', config: {},
    capabilities: ['host/place', 'intercept/tool.call', 'intercept/future.verb', 'intercept/future.verb/after'],
  } });
  const result = messages[0].result;
  assert.deepEqual(result.links, [{ point: 'future.verb', after: true }]);
  assert.deepEqual(result.capabilities, ['host/place', 'intercept/future.verb', 'intercept/future.verb/after']);
  assert.throws(() => app.toolsChanged([]), /capability/);
  app.close();
});

test('protocol one is refused before register runs', async () => {
  const messages: any[] = [];
  const peer = new Peer(message => messages.push(message));
  runtime(peer, () => assert.fail('register must not run'));
  await peer.receive({ id: 1, method: 'initialize', params: { protocol: 1 } });
  assert.match(messages[0].error.message, /protocol 2.*1/);
});
