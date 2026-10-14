import {test, expect, mock} from 'bingo/testing';

test('session startup opens tool-calls and completed rows respect the viewport', async $ => {
  const readFile = {steps:[{toolCall:{name:'Read',input:{file_path:'plugin.json'}}}]};
  await mock.session({cwd:$.plugin.root,model:{responses:[readFile,readFile,readFile,{steps:[{text:'Finished.'}]}]}});
  await mock.session.waitForSlot($.plugin.name, 'tool-calls');
  expect((await $.ui.panes()).some(pane => pane.id === 'tool-calls')).toBe(true);
  await $.command.run({name:'tool-calls',args:{}});
  expect((await $.ui.panes()).find(pane => pane.id === 'tool-calls')?.asked).toBe(true);
  await mock.session.submit('Read three times');
  const pane = await $.ui.mount({pane:'tool-calls'});
  expect((await pane.drawn()).split('\n').length).toBe(3);
  const tree = await pane.tree() as any;
  expect(tree.children.map((child:any) => child.dimColor)).toEqual([true,true,true]);
  await pane.resize(80,6);
  expect((await pane.drawn()).split('\n').length).toBe(2);
});
