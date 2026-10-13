export function register(on) {
  on('session.start', async ($, e) => {
    let frozen = false;
    try { $.plugin.name = 'foreign'; } catch { frozen = true; }
    await $.state.set('local', 1);
    await $.store.set('global', 2);
    let model = '';
    try { await $.model.complete({model:'offline',maxTokens:1,system:[],messages:[],tools:[]}); }
    catch (error) { model = error.message.includes('no model') ? 'no model' : error.message; }
    const answer = {
      session: await $.session.id() === e.session,
      prompt: await $.prompt.read(),
      tools: Array.isArray((await $.tool.list()).entries),
      commands: Array.isArray((await $.command.list()).entries),
      config: await $.config.read({key:'thinking'}),
      model,
      state: await $.state.get('local'),
      store: await $.store.get('global'),
      clock: typeof await $.clock.now(),
      plugin: $.plugin,
      frozen,
      abort: await $.turn.abort(),
    };
    await $.session.append({key:'nouns',value:answer});
  });
}
