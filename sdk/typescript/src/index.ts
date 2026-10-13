export { plugin, runtime } from './plugin.ts';
export { matches, patternMatches } from './on.ts';
export { Hooks } from './cross.ts';
export type { On, Hook, Next, Register } from './on.ts';
export type { Dollar, WithServices } from './dollar.ts';
export type * from './wire.ts';

export type * from './events.ts';
export type { CommandRun, ConfigSet, ToolCall } from './events.ts';
