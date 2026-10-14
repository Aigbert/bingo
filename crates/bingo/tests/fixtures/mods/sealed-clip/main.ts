export function register(on) {
  on('ui.render', {component:'Permission'}, async ($, e, next) => {
    return {kind:'Box',height:1,children:[await next(e)]};
  });
}
