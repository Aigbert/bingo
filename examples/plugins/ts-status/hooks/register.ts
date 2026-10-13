import type { Register } from 'bingo';

export const register: Register = on => {
  on('tool.call', async ($, e, next) => {
    const count = await $.store.get('calls');
    const value = Number(count?.value ?? 0) + 1;
    await $.store.set('calls', value, count?.version);
    await $.ui.status(`Calls: ${value}`, { key: 'calls' });
    return next(e);
  });
  on('tool.check', { tool: 'Bash' }, ($, e, next) => {
    if (typeof e.command === 'string' && e.command.startsWith('rm -rf')) {
      const { tool, tool_use_id, ...input } = e;
      return { call: { name: tool, callId: tool_use_id, input },
        decision: { kind: 'deny', reason: { kind: 'hook', hook: 'Denied: ts-status blocks rm -rf' } } };
    }
    return next(e);
  });
};
