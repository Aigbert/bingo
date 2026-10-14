mod support;
use serde_json::json;
use support::Fixture;

#[tokio::test]
async fn shared_helpers_run_in_quickjs_and_log_is_one_operation() {
    let source = r#"
import {atom, derive, read, update, memberOf} from 'bingo';
const count = atom('logs', 0);
const doubled = derive(async $ => 2 * await read($, count));
export function register(on) {
  on('ui.log', async ($, e, next) => {
    await update($, count, n => n + 1);
    return next(e);
  });
  on('fixture.echo', async ($, e) => {
    await $.ui.log('a local log');
    return {count:await read($, count), doubled:await read($, doubled), member:memberOf(['yes'])('yes')};
  });
}
"#;
    let fixture = Fixture::new(source, &["state", "ui"], json!({})).await;
    assert_eq!(
        fixture.call("fixture", "echo", json!({})).await,
        json!({"count":1,"doubled":2,"member":true})
    );
    fixture.plugin.reload(&fixture.root).await.unwrap();
    assert_eq!(
        fixture.call("fixture", "echo", json!({})).await,
        json!({"count":2,"doubled":4,"member":true})
    );
    fixture.host.shutdown().await;
}
