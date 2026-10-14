type Body = ($: any) => unknown;
type Case = {name: string; body: Body; skip: boolean; at: string};
const cases: Case[] = [];
const groups: string[] = [];
let context: {request: (method: string, params: unknown) => Promise<any>; site: any} | undefined;
const add = (name: string, body: Body, skip: boolean) => {
  const stack = new Error().stack ?? '';
  const at = stack.split('\n').find(line => /\.test\.tsx?:/.test(line))?.trim() ?? '';
  cases.push({name: [...groups, name].join(' > '), body, skip, at});
};
export const test = Object.assign((name: string, body: Body) => add(name, body, false), {
  skip: (name: string, body: Body = () => {}) => add(name, body, true),
});
export function describe(name: string, body: () => void) {
  groups.push(name);
  try { body(); } finally { groups.pop(); }
}
function equal(a: any, b: any, partial = false): boolean {
  if (Object.is(a,b)) return true;
  if (a === null || b === null || typeof a !== 'object' || typeof b !== 'object') return false;
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  const keys = Object.keys(b);
  return (partial || Object.keys(a).length === keys.length) && keys.every(key => Object.hasOwn(a,key) && equal(a[key],b[key],partial));
}
function thrown(value: unknown, expected: unknown): boolean {
  const message = value instanceof Error ? value.message : String(value);
  if (expected === undefined) return true;
  if (expected instanceof RegExp) return expected.test(message);
  if (typeof expected === 'function') return value instanceof expected;
  return message.includes(String(expected));
}
function match(actual: any, invert: boolean, rejected = false): any {
  const check = (yes: boolean, expected: unknown) => {
    if (yes === invert) throw new Error(`expected ${JSON.stringify(actual)} ${invert ? 'not ' : ''}to match ${String(JSON.stringify(expected))}`);
  };
  return {
    get not() { return match(actual,!invert,rejected); },
    get rejects() {
      return new Proxy({}, {get: (_, key: string) => {
        if (key === 'not') return rejectedProxy(actual, !invert);
        return (...args: any[]) => rejection(actual,invert,key,args);
      }});
    },
    toBe(expected: unknown) { check(Object.is(actual,expected),expected); },
    toEqual(expected: unknown) { check(equal(actual,expected),expected); },
    toMatchObject(expected: unknown) { check(equal(actual,expected,true),expected); },
    toContain(expected: unknown) { check(typeof actual === 'string' ? actual.includes(String(expected)) : Array.isArray(actual) && actual.some(v => Object.is(v,expected)),expected); },
    toThrow(expected?: unknown) {
      if (rejected) { check(thrown(actual,expected),expected); return; }
      if (typeof actual !== 'function') throw new Error('toThrow expects a function');
      let caught = false; let error;
      try { actual(); } catch (value) { caught = true; error = value; }
      check(caught && thrown(error,expected),expected);
    },
  };
}
function rejectedProxy(value: unknown, invert: boolean): any {
  return new Proxy({}, {get: (_, key: string) => key === 'not' ? rejectedProxy(value,!invert) : (...args: any[]) => rejection(value,invert,key,args)});
}
async function rejection(value: any, invert: boolean, key: string, args: any[]) {
  let rejected = false; let error;
  try { await value; } catch (caught) { rejected = true; error = caught; }
  if (!rejected) throw new Error('expected promise to reject');
  match(error,invert,true)[key](...args);
}
export function expect(value: unknown) { return match(value,false); }
function request(method: string, params: unknown = null) {
  if (!context) throw new Error('mock and reload must run inside a test');
  return context.request(`testing/${method}`,params);
}
export const mock = {
  clock: {set: (value: string) => request('clock.set',value), advance: (ms: number) => request('clock.advance',ms)},
  store: {seed: (values: Record<string, unknown>) => request('store.seed',values), read: (key: string) => request('store.read',key)},
  env: (values: Record<string,string>) => request('env',values),
  session: Object.assign(async (options: unknown) => { const site = await request('session',options); Object.assign(context!.site,site); }, { waitForSlot: (plugin: string, slot: string) => request('session.waitForSlot',{plugin,slot}), snapshot: () => request('session.snapshot'), submit: (text: string) => request('session.submit',text) }),
};
export const reload = () => request('reload');
export const list = () => cases.map(({name,skip,at}) => ({name,skip,at}));
export async function run(index: number, $: any, port: any, site: any) {
  const entry = cases[index];
  if (!entry) throw new Error('unknown test');
  if (entry.skip) return {status:'skip'};
  context = {request:port,site};
  try { await entry.body($); return {status:'pass'}; }
  catch (error) { return {status:'fail', message:String(error), at:error?.stack?.split('\n').find((line: string) => /\.test\.tsx?:/.test(line))?.trim() ?? entry.at}; }
  finally { context = undefined; }
}
export function mount(component: any, props: any = {}, viewport: any = {}) {
  return request('mount',{component,props,viewport}).then((id: string) => {
    const call = (method: string, params: unknown = null) => request(`mount.${method}`,{id,params});
    return {tree: () => call('tree'), drawn: () => call('drawn'), find: (key: string) => call('find',key),
      press: (key: string) => call('press',key), input: (key: string,text: string) => call('input',{key,text}),
      select: (key: string,value: string) => call('select',{key,value}), advance: (ms: number) => call('advance',ms),
      resize: (columns: number,rows: number) => call('resize',{columns,rows})};
  });
}
