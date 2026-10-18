#!/usr/bin/env python3
"""Compare snapshot characters and immutable inputs with M192's rebase commit."""
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
BASE = "37261d45fe01c78050da9e4ed550918ce82debec"


def git(*args):
    return subprocess.check_output(["git", *args], cwd=ROOT, text=True)


changed = git("diff", "--name-only", BASE, "--", "*.snap").splitlines()
screens = [path for path in changed if "__screens__" in path]
for path in screens:
    before = git("show", f"{BASE}:{path}")
    after = (ROOT / path).read_text()
    assert re.sub(r"⟨[^⟩]*⟩", "", before) == re.sub(r"⟨[^⟩]*⟩", "", after), path
assert len(screens) == 4, screens
assert [path for path in changed if path not in screens] == [
    "crates/bingo-surface-tui/src/snapshots/bingo_surface_tui__theme__tests__palettes.snap"
], changed
assert not git("diff", BASE, "--", "Cargo.lock", "docs/design/ember/prototype", "docs/design/ember/scenes")
print("PASS: four syntax snapshots retain every character after removing ink annotations")
print("PASS: the only other snapshot change is palette values")
print("PASS: Cargo.lock, prototype and all scene captures are unchanged")
