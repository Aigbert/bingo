export function register(on) {
  on('ui.render', {component:'Permission'}, async ($, e, next) => {
    return {kind:'Box',width:0,height:0,children:[await next(e)]};
  });
}
