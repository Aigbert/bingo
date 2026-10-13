import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Peer } from './rpc.ts';
import { runtime } from './plugin.ts';
import { register } from '../../../crates/bingo-mods/tests/mods/reentrant.ts';

test('the QuickJS re-entry fixture skips only its calling hook under node', async () => {
  let plugin: Peer;
  const host = new Peer(message => { void plugin.receive(message); });
  plugin = new Peer(message => { void host.receive(message); });
  const running = runtime(plugin, register);
  let serial = 0;
  let calls = 0;

  // Model the host's two-phase echo face without a process or credentials.
  async function echo(input: unknown): Promise<unknown> {
    const chain = String(++serial);
    const before = await host.request<{ next?: unknown; answer?: unknown }>('intercept/fixture.echo', {
      chain, input, tier: 'rewrite', site: { session: 'fixture', cwd: '.' },
    });
    if (Object.hasOwn(before, 'answer')) return before.answer;
    const after = await host.request<{ result: unknown }>('intercept/fixture.echo/after', {
      chain, result: before.next,
    });
    return after.result;
  }

  host.handlers.set('service/call', async params => {
    assert.equal(params.key, 'fixture');
    assert.equal(params.method, 'echo');
    calls++;
    // Bound a broken exclusion rule deterministically instead of timing out.
    assert.equal(calls, 1, 'the calling hook re-entered through its own dollar call');
    return { result: await echo(params.params) };
  });

  try {
    await host.request('initialize', {
      protocol: 2, pluginName: 'fixture', pluginRoot: '.', config: {},
      capabilities: ['intercept/*', 'intercept/*/after', 'service/call', 'intercept/cancel'],
    });
    assert.deepEqual(await echo({ value: 1 }), { value: 1, other: 1 });
    assert.equal(calls, 1);
  } finally {
    running.close();
    plugin.close();
    host.close();
  }
});
