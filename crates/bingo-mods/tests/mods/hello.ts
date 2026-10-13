import type { Register } from 'bingo';
export const register: Register = on => {
  on('tool.call', () => ({ parts: [{ type: 'text', text: 'hello' }], isError: false }));
};
