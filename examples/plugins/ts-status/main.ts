import { plugin } from '../../../sdk/typescript/src/index.ts';
await plugin(new URL('./hooks/register.ts', import.meta.url));
