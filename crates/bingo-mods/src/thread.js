export const metadata = () => JSON.stringify(hooks.entries.map(entry => entry.pattern));
export async function run(event, input, site, ask, cancelled) {
  const controller = new AbortController();
  Object.defineProperty(controller.signal, 'aborted', { get: cancelled });
  const request = async (method, params, hook = 0) => {
    const answer = JSON.parse(await ask(method, JSON.stringify(params), hook));
    if (!answer.ok) throw new Error(answer.error);
    return answer.value;
  };
  const location = JSON.parse(site);
  const entries = hooks.entries.filter((entry, index) => patternMatches(entry.pattern, event) && !(location.skip ?? []).includes(index));
  const result = await hooks.dispatch(entries, 0, JSON.parse(input), dollar({ request }, location),
    location.origin, controller.signal, value => request('next', value), entry => dollar({request: (method, params) => request(method, params, hooks.entries.indexOf(entry))}, location));
  return JSON.stringify(result ?? null);
}
