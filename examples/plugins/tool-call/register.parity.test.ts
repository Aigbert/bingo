import {test, expect, mock} from 'bingo/testing';

const invoke = (name: string, input: unknown) => ({steps:[{toolCall:{name,input}}]});
const done = {steps:[{text:'Finished.'}]};
const edits = (file_path: string) => invoke('Edit',{file_path,old_string:'old',new_string:'new'});

test('environment edits are denied with the plugin name', async $ => {
  for (const path of ['.env','config/.env.local']) {
    await mock.session({permissions:{allow:['Write','Edit','Read']},model:{responses:[invoke('Write',{file_path:path,content:'old'}),edits(path),invoke('Read',{file_path:path}),done]}});
    await mock.session.submit('Check the environment guard');
    const snapshot = await mock.session.snapshot() as any;
    const call = snapshot.items.find((item: any) => item.body.kind === 'toolCall' && item.body.name === 'Edit');
    const result = JSON.stringify(call.body.output);
    expect(call.status).toBe('failed');
    expect(result).toContain('protected');
    expect(result).toContain($.plugin.name);
    const read = snapshot.items.find((item: any) => item.body.kind === 'toolCall' && item.body.name === 'Read');
    expect(JSON.stringify(read.body.output)).toContain('old');
  }
});

test('envrc is editable', async () => {
  await mock.session({permissions:{allow:['Write','Edit']},model:{responses:[
    invoke('Write',{file_path:'.envrc',content:'old'}),edits('.envrc'),done,
  ]}});
  await mock.session.submit('Write and edit the shell configuration');
  const snapshot = await mock.session.snapshot() as any;
  const edit = snapshot.items.find((item: any) => item.body.kind === 'toolCall' && item.body.name === 'Edit');
  expect(edit.status).toBe('completed');
  expect(edit.body.output.isError ?? false).toBe(false);
});

test('shell input is trimmed and failure status clears after success', async () => {
  await mock.session({permissions:{allow:['Bash']},model:{responses:[
    invoke('Bash',{command:'  exit 7  '}),done,
    invoke('Bash',{command:'  echo parity  '}),done,
  ]}});
  await mock.session.submit('Run the failing shell');
  expect(JSON.stringify((await mock.session.snapshot() as any).signals)).toContain('failed:');
  await mock.session.submit('Run the successful shell');
  const snapshot = await mock.session.snapshot() as any;
  expect(JSON.stringify(snapshot.signals)).not.toContain('failed:');
  const calls = snapshot.items.filter((item: any) => item.body.kind === 'toolCall');
  expect(JSON.stringify(calls[1].body.output)).toContain('$ echo parity');
});
