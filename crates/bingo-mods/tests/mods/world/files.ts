export function register(on) {
  on('world.echo', async ($, e) => {
    try {
      return {value: await $.fs.read({path: e.path})};
    } catch (error) {
      return {error: String(error)};
    }
  });
}
