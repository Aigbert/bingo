export function register(on) {
  on('model.complete', async ($, e) => $.model.complete(e));
}
