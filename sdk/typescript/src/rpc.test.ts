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

test('catalogue subscriptions wrap a service event added after initialization', async () => {
  const messages: any[] = [];
  const peer = new Peer(message => messages.push(message));
  let observed = 0;
  const app = runtime(peer, on => {
    on('mcp.*', async ($, e, next) => ({ ...await next(e), wrapped: true }));
    on('session.start', () => { observed++; });
  });
  await peer.receive({ id: 1, method: 'initialize', params: {
    protocol: 2, pluginName: 'example', config: {},
    capabilities: ['intercept/*', 'intercept/*/after', 'hook/observe'],
  } });
  assert.deepEqual(messages[0].result.links, []);
  assert.deepEqual(messages[0].result.subscriptions,
    [{ patterns: ['mcp.*', 'session.start'], prefilter: null, render: [] }]);
  await peer.receive({ id: 2, method: 'intercept/mcp.servers', params: { chain: 'c', input: null, site: { session: 's' } } });
  assert.deepEqual(messages[1].result, { next: null });
  await peer.receive({ id: 3, method: 'intercept/mcp.servers/after', params: { chain: 'c', result: { servers: {} } } });
  assert.deepEqual(messages[2].result, { result: { servers: {}, wrapped: true } });
  await peer.receive({ method: 'hook/observe', params: { event: 'session.start', payload: { session: 's' }, site: { session: 's' } } });
  assert.equal(observed, 1);
  app.close();
});

test('invocation context follows concurrent reverse calls without entering payloads', async () => {
  const messages: any[] = [];
  const peer = new Peer(message => messages.push(message));
  const release: (() => void)[] = [];
  peer.handlers.set('hook', async params => {
    assert.deepEqual(params, { value: 1 });
    await new Promise<void>(resolve => release.push(resolve));
    peer.notify('reverse', { value: 2 });
  });
  const first = peer.receive({ method: 'hook', params: { value: 1, _bingo: { hook: 1 } } });
  const second = peer.receive({ method: 'hook', params: { value: 1, _bingo: { hook: 2 } } });
  release[1]();
  await second;
  release[0]();
  await first;
  assert.deepEqual(messages.map(message => message.params._bingo), [{ hook: 2 }, { hook: 1 }]);
  peer.notify('outside', { value: 3 });
  assert.deepEqual(messages.at(-1).params, { value: 3 });
  peer.handlers.set('plain', params => assert.equal(params, null));
  await peer.receive({ method: 'plain', params: null });
});
