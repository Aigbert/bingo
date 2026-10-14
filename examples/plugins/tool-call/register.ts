import { atom, read, update, memberOf } from 'bingo';
import type { On } from 'bingo';
const calls = atom('calls', 0);
const counted = memberOf(['Bash', 'Read', 'Edit']);

export function register(on: On) {
  on('tool.check', {tool: 'Edit'}, async ($, event, next) => {
    const checked = await next(event);
    const path = String(event.file_path ?? '').replaceAll('\\', '/');
    if (path === 'protected.txt' || path.endsWith('/protected.txt')) {
      return {...checked, decision: {kind: 'deny', reason: {kind: 'hook', hook: 'example-tool-call: protected file'}}};
    }
    return checked;
  });
  on('tool.call', async ($, event, next) => {
    if (counted(event.tool)) {
      await update($, calls, count => count + 1);
      await $.ui.status(`Example calls: ${await read($, calls)}`);
    }
    if (event.tool === 'Bash' && event.command === 'echo before') {
      return next({...event, command: 'echo after'});
    }
    return next(event);
  });
  on('ui.render', {component: 'ToolUse', props: {tool: 'Bash'}}, async ($, event, next) => {
    const {Box, Text} = await $.ui.resolve(event);
    return {kind: Box, children: [{kind: Text, children: ['Shell checked']}, await next(event)]};
  });
}
