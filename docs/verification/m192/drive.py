#!/usr/bin/env python3
"""Drive only this build, a private tmux server and temporary fake-provider data."""
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


def drive(width, height):
    with tempfile.TemporaryDirectory(prefix="m192-drive-") as directory:
        work = Path(directory)
        socket = work.name
        env = dict(os.environ)
        for name in ("OPENAI_API_KEY", "OPENAI_BASE_URL", "ANTHROPIC_API_KEY", "ANTHROPIC_BASE_URL",
                     "NO_COLOR", "BINGO_ASCII"):
            env.pop(name, None)
        for name, child in (("HOME", "home"), ("XDG_DATA_HOME", "data"), ("XDG_CONFIG_HOME", "config")):
            (work / child).mkdir()
            env[name] = str(work / child)
        (work / "cwd").mkdir()
        (work / "home/.bingo").mkdir()
        (work / "home/.bingo/settings.json").write_text('{"update":{"check":false}}')
        script = work / "fake.json"
        script.write_text(json.dumps({"responses": [{"steps": [{"text": "Ember primitives are ready."}]}]}))
        env.update(BINGO_FAKE_SCRIPT=str(script), BINGO_THEME="light", BINGO_MOTION="off",
                   BINGO_GRAPHICS="off", COLORTERM="truecolor", TERM="xterm-256color")

        def tmux(*args):
            return subprocess.check_output(["tmux", "-L", socket, *args], env=env, text=True)

        def pane(escapes=False):
            return tmux("capture-pane", "-p", *(["-e"] if escapes else []), "-t", "drive")

        def await_text(needle):
            deadline = time.monotonic() + 20
            while time.monotonic() < deadline:
                text = pane()
                if needle in text:
                    return text
                time.sleep(0.05)
            raise AssertionError(f"Timed out waiting for {needle!r}:\n{text}")

        started = False
        try:
            command = shlex.join([str(BIN), "--cwd", str(work / "cwd")])
            tmux("-f", "/dev/null", "new-session", "-d", "-s", "drive", "-c", str(work / "cwd"),
                 "-x", str(width), "-y", str(height), command)
            started = True
            await_text("/ commands")
            tmux("send-keys", "-t", "drive", "-l", "Check the Ember palette.")
            tmux("send-keys", "-t", "drive", "Enter")
            await_text("Ember primitives are ready.")
            await_text("/ commands")
            stem = f"light-{width}x{height}"
            for extension, escapes in (("txt", False), ("ansi", True)):
                text = pane(escapes).replace(work.name, "m192-drive-XXXXXXXX")
                if escapes:
                    assert "38;2;" in text, "The drive must capture truecolor ink"
                (OUT / f"{stem}.{extension}").write_text(text)
            print(f"PASS {width}x{height}: fake-provider reply captured; private HOME/data/cwd")
        finally:
            if started:
                tmux("kill-server")
                print(f"CLEAN {width}x{height}: owned tmux server stopped; temporary files removed")


if __name__ == "__main__":
    drive(100, 32)
    drive(80, 24)
