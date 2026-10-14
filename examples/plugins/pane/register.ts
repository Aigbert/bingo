import { atom, derive, read, update } from 'bingo';
import type { On } from 'bingo';
const calls = atom<{id: string; tool: string}[]>('calls', []);
const rows = derive(async $ => (await read($, calls)).map(call => `${call.tool} · ${call.id}`));

export function register(on: On) {
  on('session.start', async $ => {
    await $.command.register({name: 'ledger', aliases: [], hint: 'Open the tool ledger', args: {kind: 'none'}, instant: true, family: 'ui'}, async api => {
      await api.ui.open({id: 'ledger', title: 'Tool ledger', focus: true, closeOnEscape: true, rows: 6});
      return {kind: 'applied'};
    });
  });
  on('tool.call', async ($, event, next) => {
    await update($, calls, held => [...held, {id: event.tool_use_id, tool: event.tool}]);
    return next(event);
  });
  on('ui.render', {component: 'Pane', requestId: 'ledger'}, async ($, event) => {
    const {Box, Text} = await $.ui.resolve(event);
    const content = await read($, rows);
    return {kind: Box, children: (content.length ? content : ['No calls yet']).map(text => ({kind: Text, children: [text]}))};
  });
}
