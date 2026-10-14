import {test, expect, mock} from 'bingo/testing';
const readFile = {steps:[{toolCall:{name:'Read',input:{file_path:'plugin.json'}}}]};
const done = {steps:[{text:'Finished.'}]};

test('the band counts each turn independently and hides on press', async $ => {
  await mock.session({cwd:$.plugin.root,model:{responses:[readFile,readFile,done,readFile,done]}});
  await mock.session.submit('Read twice');
  const band = await $.ui.mount('AbovePrompt',{});
  expect(await band.drawn()).toContain('2 tool calls');
  await mock.session.submit('Read once');
  expect(await band.drawn()).toContain('1 tool calls');
  await band.press('Hide');
  expect(await band.drawn()).not.toContain('tool calls');
});
