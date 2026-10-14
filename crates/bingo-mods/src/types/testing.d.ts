import type { Dollar } from './bingo/dollar.ts';
import type { Element } from './bingo/elements';
export interface Mount {
  tree(): Promise<Element>; drawn(): Promise<string>; find(key: string): Promise<Element | null>;
  press(key: string): Promise<void>; input(key: string, text: string): Promise<void>;
  select(key: string, value: string): Promise<void>; advance(ms: number): Promise<void>;
  resize(columns: number, rows: number): Promise<void>;
}
export type TestDollar = Omit<Dollar,'ui'> & {ui: Dollar['ui'] & {mount(component: string | {pane:string}, props?: unknown, viewport?: {columns?:number;rows?:number}): Promise<Mount>}};
export interface Expectation {
  readonly not: Expectation; readonly rejects: Expectation;
  toBe(expected: unknown): any; toEqual(expected: unknown): any; toMatchObject(expected: unknown): any;
  toContain(expected: unknown): any; toThrow(expected?: unknown): any;
}
export declare function expect(value: unknown): Expectation;
export declare const test: { (name: string, body: ($: TestDollar) => unknown): void; skip(name: string, body?: ($: TestDollar) => unknown): void };
export declare function describe(name: string, body: () => void): void;
export declare function reload(): Promise<void>;
export declare const mock: {
  clock: {set(time: string): Promise<void>; advance(ms: number): Promise<void>};
  store: {seed(values: Record<string,unknown>): Promise<void>; read(key: string): Promise<{value:unknown;version:number}|null>};
  env(values: Record<string,string>): Promise<void>;
  session: { (options: {model?: unknown; cwd?:string; permissions?: {allow?:string[];deny?:string[];ask?:string[]}}): Promise<void>; submit(text: string): Promise<void>; snapshot(): Promise<unknown> };
};
