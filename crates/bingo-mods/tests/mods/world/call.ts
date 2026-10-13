export function register(on) {
  const counts = {};
  for (const name of ['fs.read','fs.write','fs.list','fs.exists','fs.stat','fs.ancestors','http.fetch','env.get','env.set','process.run','process.spawn','process.read','process.write','process.kill']) {
    on(name, async ($, e, next) => { counts[name] = (counts[name] ?? 0) + 1; return next(e); });
  }
  on('world.echo', async ($, e) => {
    if (e.counts) return counts;
    try {
      const value = await $[e.noun][e.method]({...e.input, session:e.session});
      return {value:e.noun === 'process' && e.method === 'spawn' ? value.id : value};
    } catch (error) { return {error: String(error)}; }
  });
}
