async function call($, name, input) {
  const [noun, method] = name.split('.');
  if (noun === 'state' || noun === 'store') {
    if (method === 'get' || method === 'delete') return $[noun][method](input.key);
    if (method === 'set') return $[noun].set(input.key, input.value, input.ifVersion);
  }
  if (name === 'model.complete') return $.model.complete(input.request);
  if (name === 'tool.register') return $.tool.register(input.spec, async () => ({parts:[],isError:false}));
  if (name === 'command.register') return $.command.register(input.spec, async () => ({kind:'applied',message:'done'}));
  if (name === 'clock.sleep') return $.clock.sleep(input.ms);
  if (name === 'clock.after' || name === 'clock.every') return $.clock[method](input.ms, async () => {});
  return $[noun][method](input);
}
export function register(on) {
  on('fixture.call', ($, e) => call($, e.name, e.input));
  on('session.append', {kind:'dollar-fixture'}, async ($, e, next) => {
    let answer;
    try {answer = {value:await call($, e.payload.name, e.payload.input)};}
    catch(error) {answer = {error:error.message};}
    await $.session.append({key:'dollar-answer',value:{name:e.payload.name,...answer}});
    return next(e);
  });
}
