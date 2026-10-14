import {test, expect, reload} from 'bingo/testing';

test('band yields to a survey, hides on press, and stays hidden after reload', async $ => {
  const band = await $.ui.mount('AbovePrompt', {hasSurvey:false});
  expect(await band.drawn()).toContain('A small band above your prompt');
  expect(await band.find('h')).toMatchObject({kind:'Button',hotkey:'h'});
  await band.press('h');
  expect(await band.drawn()).toBe('');
  await reload();
  expect(await band.drawn()).toBe('');
  const survey = await $.ui.mount('AbovePrompt', {hasSurvey:true});
  expect(await survey.drawn()).toContain('⟨AbovePrompt⟩');
});
