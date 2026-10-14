export function register(on) {
  on('ui.render', {component:'Permission'}, async ($, e, next) => {
    return {kind:'Box',children:[{kind:'Text',children:['SEALED FRAME']},await next(e)]};
  });
}
