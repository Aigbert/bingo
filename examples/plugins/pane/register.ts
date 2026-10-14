import { atom, derive, read, update } from 'bingo';
import type { On } from 'bingo';
const calls = atom<{id: string; tool: string; isDone:boolean}[]>('calls', []);
const rows = derive(async $ => (await read($, calls)).map(call => `${call.tool} · ${call.id}`));

export function register(on: On) {
  on('session.start', async $ => {
    await $.command.register({name: 'ledger', aliases: [], hint: 'Open the tool ledger', args: {kind: 'none'}, instant: true, family: 'ui'}, async api => {
      await api.ui.close('tool-calls');
      await api.ui.open({id: 'ledger', title: 'Tool ledger', focus: true, closeOnEscape: true, rows: 6});
      return {kind: 'applied'};
    });
    await $.command.register({name:'tool-calls',description:'Open session activity'}, async api => {
      await api.ui.open({id:'tool-calls',title:'Tool activity'});
      return {kind:'applied',message:'Activity pane opened'};
    });
    if ((await $.ui.panes()).length === 0) await $.ui.open({id:'tool-calls',title:'Tool activity'});
  });
  on('tool.call', async ($, event, next) => {
    await update($, calls, held => [...held, {id: event.tool_use_id, tool: event.tool, isDone:false}].slice(-200));
    const result = await next(event);
    await update($, calls, held => held.map(call => call.id === event.tool_use_id ? {...call,isDone:true} : call));
    return result;
  });
  on('ui.render', {component: 'Pane', requestId: 'ledger'}, async ($, event) => {
    const {Box, Text} = await $.ui.resolve(event);
    const content = await read($, rows);
    return {kind: Box, children: (content.length ? content : ['No calls yet']).map(text => ({kind: Text, children: [text]}))};
  });
  on('ui.render', {component:'Pane',requestId:'tool-calls'}, async ($, event) => {
    const {Box, Text} = await $.ui.resolve(event);
    const visible = (await read($, calls)).slice(-Math.max(1,event.viewport.rows - 4));
    return {kind:Box,children:visible.map(call => ({kind:Text,dimColor:call.isDone,children:[`${call.tool} · ${call.isDone ? 'finished' : 'running'}`]}))};
  });

}
