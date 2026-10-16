import {test, expect, mock, reload} from 'bingo/testing';

test('ledger draws two calls, survives reload and resize, then closes on Escape', async $ => {
  await mock.session({cwd:$.plugin.root,model:{responses:[
    {steps:[{toolCall:{name:'Read',input:{file_path:'register.ts'}}}]},
    {steps:[{toolCall:{name:'Read',input:{file_path:'plugin.json'}}}]},
    {steps:[{text:'Two files read.'}]},
  ]}});
  await $.command.run({name:'ledger',args:''});
  const pane = await $.ui.mount({pane:'ledger'});
  expect(await pane.drawn()).toBe('No calls yet');
  await mock.session.submit('Read both files');
  const calls = (await $.state.get('calls'))!.value as {id:string;tool:string}[];
  expect(calls.length).toBe(2);
  expect(await pane.drawn()).toBe(calls.map(call => `${call.tool} · ${call.id}`).join('\n'));
  await reload();
  await pane.resize(100,40);
  expect(await pane.drawn()).toBe(calls.map(call => `${call.tool} · ${call.id}`).join('\n'));
  await pane.press('Escape');
  expect(await $.ui.panes()).toEqual([]);
});
