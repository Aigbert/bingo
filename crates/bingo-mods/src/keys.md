# Keys and built-in mods

A built-in feature is a `bingo-mod-<noun>` Rust plugin. Use
`crates/bingo-mod-hello` as the template: its small `lib.rs` registers one
command, a hidden handler command, and a drawing link; each noun owns its
module and its tests. The fixture is absent from the release list.

Register a default drawing with `Contribution::Interceptor` and
`bingo_sdk::plugin::Placement::Builtin`. Outside links run first, ordinary
in-process `Outer` links follow, and built-ins run in declaration order just
before the surface's `Engine` fallback. Placement never grants authority:
a replacement command still requires `rewrite`. Existing links use `Outer`.

A Rust link returns the same `View` vocabulary as a JavaScript mod. It imports
neither a surface nor another plugin. A ready render is drawn in the same
pass; a pending external render keeps the surface responsive and is drawn
when its answer arrives.

```js
export function register(on) {
  on('ui.render', {component: 'Toast'}, async ($, event, next) => {
    const tree = await next(event);
    return {...tree, children: [h('Text', {}, 'Wrapped'), ...tree.children]};
  });
}
```

Calling `next` receives the built-in's tree. Answering alone replaces it.
Keep an inherited `onKey` on the root when wrapping an interactive tree.
`enabledPlugins: {"hello": false}` removes the fixture's command, default
binding and link together; the surface still draws its own fallback.

Commands may declare `keys: ["ctrl+x"]`; a binding that submits arguments
names them, as `/model` does with `{"chord": "alt+m", "args": "next"}`. The
`keys` setting maps a chord to a command line, including arguments:
`{"alt+x": "/model next"}`. Each settings layer replaces this map as a whole.
An accepted setting for a line removes the default binding of that same line;
defaults otherwise fill unclaimed keys in catalogue order. Conflicts report both commands. Static surface keys are
refused. The shortcut panel includes the resulting command bindings.

Chords use lowercase names with `ctrl+`, `alt+`, then `shift+`, for example
`alt+h`, `ctrl+x`, `shift+f2`. Escape, interrupt, composer editing and existing
screen keys stay with the surface until those screens move into their own
mods. A terminal may intercept a chord before bingo receives it.

A focused pane can own its keys with one root `Box.onKey`. The value is a
minted `Action`, using the same hidden `<plugin>:ui` command as other controls.
JavaScript supplies a callback: `h('Box', {onKey: ({key}) => ({handled: key ===
'x'})}, ...)`. The wire action uses `event: "press"` and `value: {"key":"x"}`.
The handler returns `CommandOutcome::Handled` in Rust, or `{handled:false}`
in JavaScript to pass the key back to pane controls and then the composer.
Escape and ctrl+c never reach `onKey`. A pending acknowledgement is discarded
when its focus or session has changed or a safety key has been pressed.

Handlers retain the supervised action/mailbox path; rendering alone has the
immediate polling path. No new `HostApi` method is needed. A future interaction
site can use the same root-handler contract once it has a focus owner.
