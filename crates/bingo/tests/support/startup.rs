use super::*;

fn startup(action: &str) {
    let home = tempfile::tempdir().unwrap();
    let plugin = home.path().join("startup");
    std::fs::create_dir(&plugin).unwrap();
    std::fs::write(
        plugin.join("plugin.json"),
        r#"{"name":"startup","version":"1","module":"main.ts","uses":["state","command"]}"#,
    )
    .unwrap();
    std::fs::write(
        plugin.join("main.ts"),
        r#"export function register(on) {
          on('session.start', async $ => {
            for (let i = 0; i < 64; i++) await $.state.set('first', i);
            await $.command.register({name:'ready',hint:'Ready',aliases:[],args:{kind:'none'},instant:true},
              async () => ({kind:'applied'}));
          });
          on('session.*', async $ => {
            for (let i = 0; i < 64; i++) await $.state.set('last', i);
            await $.state.set('ready', true);
          });
        }"#,
    )
    .unwrap();
    std::fs::write(
        plugin.join("startup.test.ts"),
        format!(
            r#"import {{test,expect,mock,reload}} from 'bingo/testing';
              test('startup completes', async $ => {{
                {action}
                await $.command.run({{name:'ready',args:{{}}}});
                expect((await $.state.get('ready'))?.value).toBe(true);
              }});"#
        ),
    )
    .unwrap();
    assert!(success(&run(home.path(), "test", &plugin)).starts_with("PASS"));
}

#[test]
fn cases_wait_for_session_start() {
    startup("");
}

#[test]
fn replacement_sessions_wait_for_session_start() {
    startup("await mock.session({});");
}

#[test]
fn reload_waits_for_session_start() {
    startup("await $.state.set('ready', false); await reload();");
}

#[test]
fn replacement_waits_for_other_modules_with_wildcard_start_hooks() {
    let home = tempfile::tempdir().unwrap();
    let plugin = home.path().join("primary");
    let peer = plugin.join(".bingo/plugins/peer");
    std::fs::create_dir_all(&peer).unwrap();
    std::fs::write(
        plugin.join("plugin.json"),
        r#"{"name":"primary","version":"1","module":"main.ts","uses":["command"]}"#,
    )
    .unwrap();
    std::fs::write(plugin.join("main.ts"), "export function register(on) {}").unwrap();
    std::fs::write(
        peer.join("plugin.json"),
        r#"{"name":"peer","version":"1","module":"main.ts","uses":["state","command"]}"#,
    )
    .unwrap();
    std::fs::write(peer.join("main.ts"), r#"export function register(on) {
      on('session.*', async $ => {
        for (let i = 0; i < 64; i++) await $.state.set('starting', i);
        await $.command.register({name:'peer',hint:'Peer',aliases:[],args:{kind:'none'},instant:true},
          async () => ({kind:'applied'}));
      });
    }"#).unwrap();
    std::fs::write(
        plugin.join("startup.test.ts"),
        r#"import {test,mock} from 'bingo/testing';
      test('other modules finish startup', async $ => {
        await mock.session({cwd:$.plugin.root});
        await $.command.run({name:'peer',args:{}});
      });"#,
    )
    .unwrap();
    assert!(success(&run(home.path(), "test", &plugin)).starts_with("PASS"));
}
