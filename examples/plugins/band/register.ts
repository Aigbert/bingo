import { atom, read, update } from 'bingo';
import type { On, Wire } from 'bingo';
const hidden = atom('hidden', false);
const summary = atom<{calls:number; seconds:number} | null>('summary', null);

export function register(on: On) {
  let began = 0, calls = 0;
  on('prompt.submit', async ($, event, next) => {
    began = await $.clock.now();
    calls = 0;
    return next(event);
  });
  on('tool.call', ($, event, next) => { calls++; return next(event); });
  on('turn.complete', async ($, event, next) => {
    const seconds = Math.round((await $.clock.now() - began) / 1000);
    await update($, summary, () => ({calls, seconds}));
    if (seconds > 120) await $.ui.toast(`Long turn: ${seconds} seconds`);
    return next(event);
  });
  on('ui.render', {component: 'AbovePrompt', requestId: 'AbovePrompt'}, async ($, event, next) => {
    if ((event.props as Wire.AbovePromptProps).hasSurvey) return next(event);
    const {Box, Text, Button} = await $.ui.resolve(event);
    if (await read($, hidden)) return {kind: Box, children: []};
    const completed = await read($, summary);
    return {kind: Box, children: [
      {kind: Text, children: [completed ? `${completed.calls} tool calls · ${completed.seconds} seconds` : 'A small band above your prompt']},
      {kind: Button, children: ['Hide band (Alt+h)'], hotkey: 'h', onPress: async () => {
        await update($, hidden, () => true);
        await $.ui.log('Band hidden');
      }},
    ]};
  });
}
