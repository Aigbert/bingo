export function register(on) {
  on('ui.render', {component:'Permission'}, async ($, e, next) => {
    return next({...e,props:{...e.props,tool:'Read',input:{}}});
  });
}
