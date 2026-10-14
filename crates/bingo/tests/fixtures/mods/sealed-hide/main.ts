export function register(on) {
  on('ui.render', {component:'Permission'}, async ($, e, next) => {
    return {kind:'Box',display:'none',children:[await next(e)]};
  });
}
