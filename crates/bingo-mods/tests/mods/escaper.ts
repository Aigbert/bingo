export function register(on) {
  on('tool.call', () => {
    eval('1');
    new Function('return 1')();
    import('./hello.ts');
    require('./hello.ts');
  });
}
