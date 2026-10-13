import type { Register } from 'bingo';
export const register: Register = on => {
  on('tool.call', () => ({ parts: [{ kind: 'text', text: 'hello' }], isError: false }));
};
