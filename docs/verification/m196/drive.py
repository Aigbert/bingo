#!/usr/bin/env python3
"""Exercise acting keys with isolated fake-provider sessions on our own tmux server."""
import base64
import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
BIN = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")) / "debug/bingo"
SOCKET = "m196-drive"


def drive(width, height):
    with tempfile.TemporaryDirectory(prefix="m196-drive-") as temporary:
        work = Path(temporary)
        env = dict(os.environ)
        for name in ("OPENAI_API_KEY", "OPENAI_BASE_URL", "ANTHROPIC_API_KEY", "ANTHROPIC_BASE_URL", "NO_COLOR", "BINGO_ASCII"):
            env.pop(name, None)
        for name, child in (("HOME", "home"), ("XDG_DATA_HOME", "data"), ("XDG_CONFIG_HOME", "config")):
            (work / child).mkdir()
            env[name] = str(work / child)
        for child in ("cwd", "bin", "home/.bingo"):
            (work / child).mkdir()
        settings = work / "home/.bingo/settings.json"
        settings.write_text(json.dumps({"update": {"check": False}}))
        # Only our child sees this clipboard double; the person's clipboard is never read.
        png = work / "shot.png"
        png.write_bytes(base64.b64decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+j8ioAAAAASUVORK5CYII="))
        stub = work / "bin/osascript"
        stub.write_text("#!/usr/bin/env python3\nimport sys,re,shutil\nm=re.search(r'POSIX file \\\"([^\\\"]+)\\\"', ' '.join(sys.argv))\nif m: shutil.copyfile(" + repr(str(png)) + ", m[1])\n")
        stub.chmod(0o755)
        env.update(PATH=str(work / "bin") + os.pathsep + env["PATH"], TERM="xterm-256color", COLORTERM="truecolor", BINGO_THEME="dark", BINGO_MOTION="on", BINGO_GRAPHICS="off")

        def tmux(*args):
            return subprocess.check_output(["tmux", "-L", SOCKET, *args], env=env, text=True, stderr=subprocess.DEVNULL)

        if subprocess.run(["tmux", "-L", SOCKET, "list-sessions"], capture_output=True).returncode == 0:
            raise RuntimeError("m196-drive is already in use; refusing to touch it")
        started = False
        session = "acting"

        def pane(escapes=False):
            return tmux("capture-pane", "-p", *(["-e"] if escapes else []), "-t", session)

        def wait(needle):
            deadline = time.monotonic() + 25
            while time.monotonic() < deadline:
                text = pane()
                if needle in text:
                    return text
                time.sleep(.05)
            raise AssertionError(f"Missing {needle!r}:\n{text}")

        def keys(*chords):
            tmux("send-keys", "-t", session, *chords)

        def type_text(text):
            tmux("send-keys", "-t", session, "-l", text)

        def capture(name):
            for extension, escapes in (("txt", False), ("ansi", True)):
                text = pane(escapes).replace(temporary, "<drive>")
                (OUT / f"{name}-{width}x{height}.{extension}").write_text(text)

        def open_session(name, responses):
            nonlocal started, session
            session = name
            script = work / f"{name}.json"
            script.write_text(json.dumps({"responses": responses}))
            child_env = dict(env, BINGO_FAKE_SCRIPT=str(script))
            command = shlex.join(["env", *[f"{k}={v}" for k, v in child_env.items() if k in {"HOME", "XDG_DATA_HOME", "XDG_CONFIG_HOME", "PATH", "BINGO_FAKE_SCRIPT", "BINGO_THEME", "BINGO_MOTION", "BINGO_GRAPHICS", "TERM", "COLORTERM"}], str(BIN), "--cwd", str(work / "cwd"), "--allowed-tools", "Bash,Read"])
            tmux("-f", "/dev/null", "new-session", "-d", "-s", session, "-x", str(width), "-y", str(height), "-c", str(work / "cwd"), command)
            started = True
            tmux("set-option", "-t", session, "remain-on-exit", "on")
            wait("/ commands")

        slow = {"steps": [{"delay": {"ms": 60000}}, {"text": "Stopped before this."}]}
        try:
            open_session("acting", [slow, slow])
            type_text("work until stopped")
            keys("Enter")
            wait("esc stop")
            type_text("steer the work")
            keys("Enter")
            wait("at the next step")
            type_text("then update CHANGELOG")
            keys("Tab")
            wait("queued")
            capture("queued")
            keys("Up")
            wait("then update CHANGELOG▍")
            capture("recalled")
            keys("Tab")
            wait("queued")
            keys("Escape")
            wait("what you queued is back in the input")
            capture("stopped")
            text = pane()
            assert "steer the work" in text and "then update CHANGELOG" in text
            keys("Escape")
            wait("cleared")
            keys("Up")
            wait("then update CHANGELOG")
            keys("Down")
            wait("Reply, or / for commands")
            keys("Escape")
            wait("esc again to rewind")
            keys("Escape")
            wait("Rewind")
            capture("rewind")
            keys("Escape")
            type_text("?")
            wait("keys")
            capture("keys")
            keys("Escape")
            type_text("look at this")
            keys("C-v")
            wait("[image 1] look at this")
            capture("image")
            keys("Enter")
            wait("esc stop")
            capture("image-sent")
            keys("C-c")
            wait("Reply, or / for commands")
            keys("C-c")
            wait("ctrl+c again to quit")
            capture("exit-armed")
            keys("C-c")

            open_session("shells", [{"steps": [{"toolCall": {"name": "Bash", "input": {"command": "sleep 3; printf 'test result: ok. 2 passed; 0 failed\\n'"}}}]}, {"steps": [{"text": "The shell is in background."}]}, {"steps": [{"text": "The background tests passed."}]}])
            type_text("!echo shell-landed")
            keys("Enter")
            wait("! echo shell-landed")
            capture("shell")
            type_text("run the test command")
            keys("Enter")
            wait("ctrl+b background")
            keys("C-b")
            wait("1 shell")
            wait("in background")
            capture("background")
            wait("finished in the background")
            capture("background-done")
            type_text("!seq 1 50")
            keys("Enter")
            wait("50")
            keys("PageUp")
            wait("latest")
            capture("scrolled")
            keys("Escape")
            type_text("draft")
            keys("C-j")
            type_text("second line")
            wait("second line")
            keys("Up", "Up")
            wait("draft")
            assert "draft" in pane() and "second line" in pane()
            capture("multiline")
            keys("C-c", "C-d")
            print(f"PASS {width}x{height}: enter/tab/up/down/esc/esc esc/ctrl+c/ctrl+d/!/ctrl+b/ctrl+v/?")
        finally:
            if started:
                tmux("kill-server")
            print(f"CLEAN {width}x{height}: owned server and temporary HOME/data/cwd")


for size in ((100, 32), (80, 24)):
    drive(*size)
