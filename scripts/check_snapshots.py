#!/usr/bin/env python3
"""List snapshot names with no source reference, including named snapshots."""
import pathlib
import re

sources = "\n".join(path.read_text() for path in pathlib.Path("crates").rglob("*.rs"))
words = set(re.findall(r"[\w.-]+", sources))
patterns = []
for template in re.findall(r'"([^"\n]*\{[^"\n]*)"', sources):
    if not re.match(r"[\w.-]{3}", template):
        continue
    parts = re.split(r"(\{[^{}]*\})", template)
    patterns.append(re.compile("".join(".+" if part.startswith("{") else re.escape(part)
                                       for part in parts)))

for path in sorted(pathlib.Path("crates").rglob("*.snap")):
    name = path.stem.split("__")[-1]
    # Insta appends a suffix for parameterized snapshots.
    name = name.split("@")[0]
    sized = re.sub(r"_\d+x\d+$", "", name)
    if name in words or sized in words or any(pattern.fullmatch(name) for pattern in patterns):
        continue
    print(f"warn {path}: snapshot name has no source reference")
