import {test, expect, mock} from 'bingo/testing';

test('denies protected edits, rewrites shell calls, counts reads and wraps ToolUse', async $ => {
  await mock.session({cwd:$.plugin.root,permissions:{allow:['Bash(echo:*)','Read']},model:{responses:[
    {steps:[{toolCall:{name:'Edit',input:{file_path:'protected.txt',old_string:'old',new_string:'new'}}}]},
    {steps:[{toolCall:{name:'Bash',input:{command:'echo before'}}}]},
    {steps:[{toolCall:{name:'Read',input:{file_path:'register.ts'}}}]},
    {steps:[{text:'All done.'}]},
  ]}});
  await mock.session.submit('Run the example calls');
  expect((await $.state.get('calls'))!.value).toBe(2);
  const history = JSON.stringify(await mock.session.snapshot());
  expect(history).toContain('example-tool-call: protected file');
  expect(history).toContain('after');
  expect(history).toContain('Example calls: 2');
  const shell = await $.ui.mount('ToolUse', {tool:'Bash'});
  expect(await shell.drawn()).toBe('Shell checked\n⟨ToolUse⟩');
  const read = await $.ui.mount('ToolUse', {tool:'Read'});
  expect(await read.drawn()).toBe('⟨ToolUse⟩');
});
