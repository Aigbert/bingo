export function register(on) {
  let other = 0;
  on('fixture.echo', async ($, e) => $.fixture.echo(e));
  on('fixture.echo', {}, async ($, e, next) => ({...await next(e), other:++other}));
}
