//! Opt-in visual verification on an owned tmux socket and local wiremock endpoints.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
mod support;
use serde_json::json;
use std::{path::PathBuf, process::Command};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};
struct Terminal {
    home: tempfile::TempDir,
    socket: String,
}
impl Terminal {
    fn command(&self) -> Command {
        let mut cmd = support::command::at("tmux");
        cmd.args(["-L", &self.socket, "-f", "/dev/null"]);
        cmd
    }
    fn run(&self, args: &[&str]) -> String {
        let out = self.command().args(args).output().unwrap();
        assert!(
            out.status.success(),
            "tmux {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }
    fn keys(&self, keys: &[&str]) {
        let mut args = vec!["send-keys", "-t", "picker"];
        args.extend(keys);
        self.run(&args);
    }
    fn text(&self, text: &str) {
        self.run(&["send-keys", "-t", "picker", "-l", text]);
    }
    fn screen(&self) -> String {
        self.run(&["capture-pane", "-p", "-t", "picker"])
    }
    async fn wait(&self, text: &str) -> String {
        tokio::time::timeout(std::time::Duration::from_secs(25), async {
            loop {
                let screen = self.screen();
                if screen
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .contains(text)
                {
                    return screen;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("waiting for {text}:\n{}", self.screen()))
    }
    async fn capture(&self, text: &str, name: &str, width: u16, height: u16) {
        let screen = self.wait(text).await;
        let dir = PathBuf::from("/tmp/m166-picker-captures");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{name}-{width}x{height}.txt")), screen).unwrap();
    }
    async fn absent(&self, text: &str) {
        tokio::time::timeout(std::time::Duration::from_secs(25), async {
            while self.screen().contains(text) {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        })
        .await
        .expect("screen changed");
    }
    async fn close_picker(&self) {
        self.keys(&["Escape"]);
        self.absent("↑↓ pick").await;
        self.wait("? for shortcuts").await;
    }
    async fn line(&self, line: &str) {
        self.text(line);
        self.keys(&["Enter"]);
    }
}
impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.command().arg("kill-server").output();
    }
}
fn start(endpoint: &str, width: u16, height: u16) -> Terminal {
    let home = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(home.path().join(".bingo")).unwrap();
    let settings = format!(
        r#"provider = "fake"
model = "fake-1"
maxTokens = 1000
pinned = ["fake/fake-1", "openai/gpt-5"]
[update]
check = false
[openai]
baseUrl = "{endpoint}"
apiKey = "dummy-local-only"
[openai.instances.work]
baseUrl = "{endpoint}/down"
apiKey = "dummy-local-only"
[models."openai/gpt-5"]
contextWindow = 32000
maxOutput = 1000
"#
    );
    std::fs::write(home.path().join(".bingo/settings.toml"), settings).unwrap();
    let script = json!({"responses":[{"steps":[{"text":format!("{}\nfirst response complete", "history ".repeat(35000))}]},{"steps":[{"text":"second response complete"}]}],"side":[{"steps":[{"text":"A short summary of the earlier history."}]}]});
    let path = home.path().join("script.json");
    std::fs::write(&path, script.to_string()).unwrap();
    let terminal = Terminal {
        socket: format!(
            "m166-{}",
            home.path().file_name().unwrap().to_string_lossy()
        ),
        home,
    };
    let mut command = terminal.command();
    command
        .envs(support::home::home_env(terminal.home.path()))
        .env("BINGO_FAKE_SCRIPT", path)
        .env("BINGO_MOTION", "off");
    command.args([
        "new-session",
        "-d",
        "-s",
        "picker",
        "-x",
        &width.to_string(),
        "-y",
        &height.to_string(),
    ]);
    command
        .arg(env!("CARGO_BIN_EXE_bingo"))
        .args(["--no-print-on-exit", "--cwd"])
        .arg(terminal.home.path());
    assert!(command.status().unwrap().success());
    terminal
}
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires an installed tmux; uses only a new isolated socket"]
async fn local_model_picker_scenes() {
    let endpoint = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[{"id":"gpt-5"}]})))
        .mount(&endpoint)
        .await;
    Mock::given(method("GET"))
        .and(path("/down/v1/models"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&endpoint)
        .await;
    for (width, height) in [(108, 30), (80, 24)] {
        let term = start(&endpoint.uri(), width, height);
        choose_and_pin(&term, width, height).await;
        add_by_paste(&term, &endpoint.uri(), width, height).await;
        term.line("/model work/").await;
        term.wait("fix work").await;
        term.keys(&["Enter"]);
        term.capture("use anyway", "down", width, height).await;
        term.keys(&["Escape"]);
        term.wait("/model work/").await;
        term.close_picker().await;
        term.line("/logout openai-2").await;
        term.wait("Signed out of openai-2").await;
        term.line("/model openai-2/gpt").await;
        term.wait("sign in, then use").await;
        term.keys(&["Enter"]);
        term.wait("paste a key").await;
        term.keys(&["Enter"]);
        term.wait("Sign in to openai-2").await;
        term.text("\x1b[200~dummy-local-only\x1b[201~");
        assert!(!term.wait("••").await.contains("dummy-local-only"));
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        term.keys(&["Enter"]);
        term.capture("openai-2/gpt-5", "signin-chain", width, height)
            .await;
        term.absent("↑↓ pick").await;
        term.wait("? for shortcuts").await;
        term.keys(&["M-t"]);
        term.capture("⇄ thinking low", "effort", width, height)
            .await;
    }
}

async fn add_by_paste(term: &Terminal, endpoint: &str, width: u16, height: u16) {
    term.keys(&["+"]);
    term.wait("paste").await;
    term.text("\x1b[200~sk-dummy-never-unmasked\x1b[201~");
    let screen = term.wait("••").await;
    assert!(!screen.contains("sk-dummy-never-unmasked"));
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    term.keys(&["Enter"]);
    term.capture("add provider", "add-by-paste", width, height)
        .await;
    assert!(!term.screen().contains("sk-dummy-never-unmasked"));
    term.keys(&["Down", "Down", "Enter"]);
    term.wait("provider address").await;
    term.text(&format!("\x1b[200~{}\x1b[201~", endpoint));
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    term.keys(&["Enter"]);
    term.wait("add provider").await;
    term.keys(&["Down", "Down", "Enter"]);
    term.wait("Provider added.").await;
    term.keys(&["Escape"]);
    term.wait("/model openai-2/").await;
    term.close_picker().await;
}

async fn choose_and_pin(term: &Terminal, width: u16, height: u16) {
    term.wait("? for shortcuts").await;
    term.line("fill the history").await;
    term.wait("first response complete").await;
    term.line("one more").await;
    term.wait("second response complete").await;
    term.line("/model").await;
    term.capture("pinned · alt+m", "open", width, height).await;
    term.text("openai/");
    term.wait("/model openai/").await;
    term.capture("gpt-5", "long", width, height).await;
    term.keys(&["Tab"]);
    term.capture("● here and new sessions", "keep", width, height)
        .await;
    term.keys(&["Enter"]);
    term.capture("compacted", "compact-switch", width, height)
        .await;
    term.keys(&["M-m"]);
    term.wait("fake-1").await;
    term.line("/model").await;
    term.wait("pinned · alt+m").await;
    term.keys(&["*"]);
    term.absent("★ ● fake-1").await;
    term.capture("/model", "pin", width, height).await;
}
