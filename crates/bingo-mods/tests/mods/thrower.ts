export function register(on) {
  on('tool.call', () => { throw new Error('fixture failure'); })
    .catch(($, e, next) => {
      if (next.error.kind !== 'throw') throw new Error('wrong error kind');
      return next(e);
    });
}
