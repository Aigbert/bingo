import { atom, read, update } from 'bingo';
import type { On } from 'bingo';
const hidden = atom('hidden', false);

export function register(on: On) {
  on('ui.render', {component: 'AbovePrompt', requestId: 'AbovePrompt'}, async ($, event, next) => {
    if (event.props.hasSurvey) return next(event);
    const {Box, Text, Button} = await $.ui.resolve(event);
    if (await read($, hidden)) return {kind: Box, children: []};
    return {kind: Box, children: [
      {kind: Text, children: ['A small band above your prompt']},
      {kind: Button, children: ['Hide band (Alt+h)'], hotkey: 'h', onPress: async () => {
        await update($, hidden, () => true);
        await $.ui.log('Band hidden');
      }},
    ]};
  });
}
