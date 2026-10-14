export const metadata = () => JSON.stringify(hooks.entries.map(entry => ({pattern:entry.pattern, recover:!!entry.recover, matcher:renderMatcher(entry.matcher)})));
export async function run(event, input, site, ask, cancelled, remaining) {
  const controller = new AbortController();
  Object.defineProperty(controller.signal, 'aborted', { get: cancelled });
  const request = async (method, params, hook = 0) => {
    const answer = JSON.parse(await ask(method, JSON.stringify(params), hook));
    if (!answer.ok) throw new Error(answer.error);
    return answer.value;
  };
  const location = JSON.parse(site);
  if (location.callback !== undefined) {
    const port = {request:(method, params) => request(method, params, 4294967295)};
    const api = dollar(port, location, location.plugin, callbacks);
    const inputValue = JSON.parse(input);
    if (location.callback === '__ui__') {
      const dispatch = (event, value, next) => hooks.dispatch(hooks.entries.filter(entry => patternMatches(entry.pattern, event)),
        0, value, api, location.origin, controller.signal, next);
      return JSON.stringify(await handlers.fire(location.session, inputValue, port, dispatch));
    }
    return JSON.stringify(await callbacks.run(location.callback, api, inputValue));
  }
  if (location.hook !== undefined) {
    const entry = hooks.entries[location.hook];
    const port = {request:(method, params) => request(method, params, location.hook)};
    const api = dollar(port, location, location.plugin, callbacks);
    let called = !!location.called;
    let continuation;
    const next = Object.assign(async value => {
      if (called) throw new Error('next may be called only once');
      if (controller.signal.aborted) throw new Error('intercept cancelled');
      called = true;
      continuation = request('next', value, location.hook);
      return continuation;
    }, {signal:controller.signal, origin:location.origin, error:location.error, budget:Object.freeze({ms:location.budget.ms, get remainingMs() {return remaining();}})});
    Object.defineProperty(next, 'called', {get:()=>called});
    const inputValue = frozen(JSON.parse(input));
    const invoke = location.recover ? entry.recover : entry.hook;
    const answer = !location.recover && entry.matcher !== undefined && !matches(entry.matcher, inputValue)
      ? await next(inputValue) : await invoke(api, inputValue, next);
    let result = answer === undefined && continuation ? await continuation : answer;
    if (event === 'ui.render' && result !== undefined) {
      result = handlers.mint(location.plugin.name, location.session, inputValue.requestId, location.generation, result, port);
    }
    return JSON.stringify({answered:result !== undefined, value:result ?? null});
  }
  const entries = hooks.entries.filter((entry, index) => patternMatches(entry.pattern, event) && !(location.skip ?? []).includes(index));
  const result = await hooks.dispatch(entries, 0, JSON.parse(input), dollar({ request }, location, location.plugin, callbacks),
    location.origin, controller.signal, value => request('next', value), entry => dollar({request: (method, params) => request(method, params, hooks.entries.indexOf(entry))}, location, location.plugin, callbacks));
  return JSON.stringify(result ?? null);
}
