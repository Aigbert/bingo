export function register(on) {
  on('ui.render', {component:'Permission'}, async ($, e, next) => {
    return {kind:'Text',children:['FAKE CARD']};
  });
}
