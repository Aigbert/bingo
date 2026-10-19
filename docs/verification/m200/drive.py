#!/usr/bin/env python3
"""M200's hands-on drive: this build, a private tmux server, the fake provider
and temporary HOME, data dir and cwd. Hire two teammates, walk in from the crew
sheet, ctrl+g to an ask, answer it in the dock, esc home; in a room, seat, post with an @,
read the answer, unseat, and close."""
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
SOCKET = "m200-drive"

# One script serves every session in the process, handed out in the order the
# sessions ask. Foreground hires keep that order fixed: home waits on each
# teammate, and the room's member answers only when the post wakes it, and
# only through SendMessage: what it says in its own session stays there.
SCRIPT = {"responses": [
    {"steps": [{"toolCall": {"name": "SpawnAgent", "input": {
        "name": "scout", "prompt": "find every caller of drain()", "background": False}}}]},
    {"steps": [{"text": "Found 4 callers of drain()."}]},
    {"steps": [{"text": "Scout is done."}]},
    {"steps": [{"toolCall": {"name": "SpawnAgent", "input": {
        "name": "reviewer", "prompt": "run the queue tests", "background": False}}}]},
    {"steps": [{"toolCall": {"name": "Bash", "input": {"command": "echo queue-tests-pass"}}}]},
    {"steps": [{"text": "The queue tests pass."}]},
    {"steps": [{"text": "Reviewer is done."}]},
    {"steps": [{"toolCall": {"name": "SendMessage", "input": {
        "to": "#design", "text": "The comment matches the code."}}}]},
    {"steps": [{"text": "Answered in #design."}]},
]}


def environment(work):
    env = dict(os.environ)
    for name in ("OPENAI_API_KEY", "OPENAI_BASE_URL", "ANTHROPIC_API_KEY", "ANTHROPIC_BASE_URL",
                 "NO_COLOR", "BINGO_ASCII", "TMUX"):
        env.pop(name, None)
    for name, child in (("HOME", "home"), ("XDG_DATA_HOME", "data"), ("XDG_CONFIG_HOME", "config")):
        (work / child).mkdir()
        env[name] = str(work / child)
    (work / "cwd").mkdir()
    (work / "home/.bingo").mkdir()
    (work / "home/.bingo/settings.json").write_text('{"update":{"check":false}}')
    script = work / "fake.json"
    script.write_text(json.dumps(SCRIPT))
    env.update(BINGO_FAKE_SCRIPT=str(script), BINGO_THEME="light", BINGO_MOTION="off",
               BINGO_GRAPHICS="off", COLORTERM="truecolor", TERM="xterm-256color")
    return env


class Drive:
    def __init__(self, work, env, size):
        self.work, self.env, self.size = work, env, size

    def tmux(self, *args):
        return subprocess.check_output(["tmux", "-L", SOCKET, *args], env=self.env, text=True)

    def pane(self, escapes=False):
        return self.tmux("capture-pane", "-p", *(["-e"] if escapes else []), "-t", "drive")

    def wait(self, needle, gone=False):
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            text = self.pane()
            if (needle in text) != gone:
                return text
            time.sleep(0.05)
        raise AssertionError(f"timed out waiting for {needle!r} (gone={gone}):\n{text}")

    def footer_drops(self, name):
        """The footer's last line no longer names someone who left."""
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            footer = [line for line in self.pane().splitlines() if line.strip()][-1]
            if name not in footer:
                return
            time.sleep(0.05)
        raise AssertionError(f"the footer still names {name!r}: {footer}")

    def keys(self, *values):
        self.tmux("send-keys", "-t", "drive", *values)

    def type(self, text):
        self.tmux("send-keys", "-t", "drive", "-l", text)

    def esc(self):
        self.type("\x1b[27u")

    def say(self, text, then):
        self.type(text)
        self.keys("Enter")
        return self.wait(then)

    def shot(self, name):
        width, height = self.size
        for extension, escapes in (("txt", False), ("ansi", True)):
            text = self.pane(escapes).replace(str(self.work), "$TMPDIR/m200-drive-XXXXXXXX")
            (OUT / f"{name}-{width}x{height}.{extension}").write_text(text)
        print(f"PASS {width}x{height}: {name}", flush=True)


def hire_and_answer(d):
    d.say("send scout to find the callers", "Scout is done.")
    d.wait("/ commands")
    d.shot("hired-scout")
    d.say("send reviewer to run the tests", "reviewer needs you")
    d.shot("reviewer-asks")
    d.keys("C-g")
    d.wait("reviewer ·")
    d.shot("walked-to-reviewer")
    # The ask waits in the reviewer's dock with Yes chosen. Its guard re-arms
    # on the walk in, so the enter comes after the guard's 400 ms.
    time.sleep(0.5)
    d.keys("Enter")
    d.wait("The queue tests pass.")
    d.shot("reviewer-answered")
    d.esc()
    d.wait("Reviewer is done.")
    d.wait("/ commands")
    d.shot("home")


def walk_in_from_the_sheet(d):
    d.say("/agents", "Crew")
    d.shot("crew-sheet")
    d.keys("Enter")
    d.wait("esc back")
    d.shot("walked-in")
    d.esc()
    d.wait("/ commands")


def room(d):
    d.say("/room design", "#design")
    d.say("/agents", "Crew")
    for _ in range(4):
        d.keys("Down")
        time.sleep(0.1)
    d.wait("#design")
    d.keys("Enter")
    d.wait("Post to #design")
    d.say("/seat reviewer:0", "reviewer joined")
    d.shot("room-seated")
    d.type("@rev")
    time.sleep(0.3)
    d.keys("Tab")
    d.type(" does the comment match the code?")
    d.keys("Enter")
    d.wait("The comment matches the code.")
    d.shot("room-answered")
    d.say("/unseat reviewer", "reviewer left")
    d.footer_drops("reviewer")
    d.shot("room-unseated")
    d.say("/close settled", "This room is closed")
    d.shot("room-closed")
    d.esc()
    d.wait("/ commands")


def drive(width, height):
    with tempfile.TemporaryDirectory(prefix="m200-drive-") as directory:
        work = Path(directory)
        env = environment(work)
        d = Drive(work, env, (width, height))
        probe = subprocess.run(["tmux", "-L", SOCKET, "list-sessions"], env=env,
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        if probe.returncode == 0:
            raise RuntimeError(f"{SOCKET} already exists; refusing to share its server")
        started = False
        try:
            command = shlex.join([str(BIN), "--cwd", str(work / "cwd")])
            d.tmux("-f", "/dev/null", "new-session", "-d", "-s", "drive", "-c", str(work / "cwd"),
                   "-x", str(width), "-y", str(height), command)
            started = True
            d.wait("/ commands")
            hire_and_answer(d)
            walk_in_from_the_sheet(d)
            room(d)
        finally:
            if started:
                d.tmux("kill-server")
                print(f"CLEAN {width}x{height}: owned tmux server stopped; temporary files removed")


if __name__ == "__main__":
    drive(100, 32)
    drive(80, 24)
