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
    with tempfile.TemporaryDirectory(prefix="m198-drive-") as directory:
        work = Path(directory)
        socket = "m198-drive"
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
        (work / "cwd/AGENTS.md").write_text("# Fixture\nUse clear English.\n")
        memories = work / "home/.bingo/data/memory/user"
        memories.mkdir(parents=True)
        (memories / "fixture.md").write_text("---\nname: fixture\ndescription: Temporary drive memory\ntype: user\n---\nRemember the fixture.\n")
        script = work / "fake.json"
        script.write_text(json.dumps({"responses": [
            {"steps": [{"toolCall": {"name": "TaskCreate", "input": {"subject": "Review fixture", "owner": "bingo"}}}]},
            {"steps": [{"toolCall": {"name": "TaskCreate", "input": {"subject": "Test fixture", "owner": "bingo"}}}]},
            {"steps": [{"text": "Ember primitives are ready."}]},
            {"steps": [{"text": "The scheduled fixture ran at home."}]}
        ]}))
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

        probe = subprocess.run(["tmux", "-L", socket, "list-sessions"], env=env,
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        if probe.returncode == 0:
            raise RuntimeError("m198-drive already exists; refusing to share its server")
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
            def keys(*values):
                tmux("send-keys", "-t", "drive", *values)

            def close(expected="/ commands"):
                tmux("send-keys", "-t", "drive", "-l", "\x1b[27u")
                await_text(expected)

            def capture(name):
                for extension, escapes in (("txt", False), ("ansi", True)):
                    text = pane(escapes).replace(work.name, "m198-drive-XXXXXXXX")
                    (OUT / f"{name}-{width}x{height}.{extension}").write_text(text)

            tmux("send-keys", "-t", "drive", "-l", "/schedule every 30m Check the drive fixture")
            keys("Enter")
            await_text("scheduled")
            for command, head in (("model", "Model"), ("status", "Status"),
                                  ("plugins", "Plugins"), ("resume", "Sessions"),
                                  ("help", "Keys"), ("tasks", "Tasks"),
                                  ("schedule", "Schedule"), ("memory", "Memory"),
                                  ("permission", "Permission mode"), ("wake", "Wake up later")):
                tmux("send-keys", "-t", "drive", "-l", "/" + command)
                keys("Enter")
                await_text(head)
                time.sleep(0.4)
                keys("Down")
                time.sleep(0.4)
                if command == "resume":
                    tmux("send-keys", "-t", "drive", "-l", "Ember")
                    await_text("Ember")
                capture(command)
                if command in ("model", "permission"):
                    if command == "model":
                        keys("Up")
                        time.sleep(0.4)
                    keys("Enter")
                    await_text("/ commands")
                    capture(command + "-acted")
                elif command == "status":
                    keys("Enter")
                    await_text("Permission mode")
                    capture(command + "-acted")
                    close()
                elif command == "plugins":
                    keys("Space")
                    await_text("anthropic off")
                    capture(command + "-acted")
                    keys("Space")
                    await_text("anthropic on")
                    close()
                elif command == "resume":
                    keys("Enter")
                    await_text("resumed ·")
                    capture(command + "-acted")
                    await_text("/ commands")
                elif command == "help":
                    for _ in range(3):
                        keys("NPage")
                        time.sleep(0.2)
                    await_text("ctrl+c")
                    capture(command + "-scrolled")
                    close()
                elif command == "memory":
                    keys("BSpace")
                    await_text("forgot")
                    capture(command + "-acted")
                    close()
                elif command == "schedule":
                    keys("Space")
                    await_text("paused")
                    capture(command + "-paused")
                    keys("Enter")
                    await_text("The scheduled fixture ran at home.")
                    await_text("/ commands")
                    capture(command + "-acted")
                elif command == "wake":
                    keys("Enter")
                    await_text("wakes in 1m")
                    capture(command + "-acted")
                else:
                    keys("Enter")
                    close()
                print(f"PASS {width}x{height}: {command} opened, moved, closed", flush=True)
            tmux("send-keys", "-t", "drive", "-l", "/mcp")
            keys("Enter")
            await_text("/mcp")
            await_text("server")
            keys("Down", "Up")
            capture("result")
            close()
            print(f"PASS {width}x{height}: result opened, scrolled, closed", flush=True)
            for chord, head in (("C-t", "Tasks"), ("C-r", "Sessions")):
                tmux("send-keys", "-t", "drive", "-l", "draft survives the sheet")
                keys(chord)
                await_text(head)
                keys("Down")
                close("draft survives the sheet")
                capture("draft-" + head.lower())
                keys("C-u")
                print(f"PASS {width}x{height}: {head} escape restores draft", flush=True)
            if width in (100, 80):
                tmux("send-keys", "-t", "drive", "-l", "\x1b[27u")
                tmux("send-keys", "-t", "drive", "-l", "\x1b[27u")
                await_text("Rewind")
                capture("rewind")
                keys("Down")
                time.sleep(0.4)
                keys("Enter")
                await_text("rewound")
                capture("rewind-acted")
                print(f"PASS {width}x{height}: rewind opened, moved, acted and closed", flush=True)
        finally:
            if started:
                tmux("kill-server")
                print(f"CLEAN {width}x{height}: owned tmux server stopped; temporary files removed")


if __name__ == "__main__":
    drive(100, 32)
    drive(80, 24)
