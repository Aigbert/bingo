export function register(on) {
  on('ui.render', {component:'Permission'}, async ($, e, next) => {
    const engine = await next(e); return {kind:'Box',children:[engine,engine]};
  });
}
