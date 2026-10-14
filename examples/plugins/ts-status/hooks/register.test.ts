import { test, expect, mock, reload } from 'bingo/testing';

test('counts calls, places status, survives reload, and denies removal', async $ => {
  const read = {steps:[{toolCall:{name:'Read',input:{file_path:'README.md'}}}]};
  const answer = {steps:[{text:'read complete'}]};
  await mock.session({cwd:$.plugin.root,model:{responses:[read,answer,read,answer,read,answer,
    {steps:[{toolCall:{name:'Bash',input:{command:'rm -rf ./must-not-run'}}}]},answer]}});
  const submit = (text: string) => mock.session.submit(text);
  for (const count of [1,2,3]) {
    if (count === 3) await reload();
    await submit(`Read the file (${count})`);
    expect((await mock.store.read('calls')).value).toBe(count);
    expect(JSON.stringify(await mock.session.snapshot())).toContain(`Calls: ${count}`);
  }
  const readHistory = await mock.session.snapshot();
  expect(JSON.stringify(readHistory)).toContain('TypeScript status');
  await submit('Try the denied command');
  const history = await mock.session.snapshot();
  expect(JSON.stringify(history)).toContain('ts-status blocks rm -rf');
});
