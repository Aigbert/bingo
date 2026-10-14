#!/usr/bin/env bash
# Architecture and file-size discipline. Exit non-zero on any violation.
set -euo pipefail
cd "$(dirname "$0")/.."

fail=0
say() { printf '%s\n' "$*"; }

# 1. Dependency direction (ADR-0001), from cargo metadata.
python3 - <<'PY' || fail=1
import json, subprocess, sys
meta = json.loads(subprocess.check_output(["cargo", "metadata", "--format-version", "1", "--no-deps"]))
members = {p["name"]: p for p in meta["packages"]}
def deps(name):
    return {d["name"] for d in members[name]["dependencies"] if d.get("kind") in (None, "normal")}
ws = set(members)
# A library (ADR-0012 §1) registers nothing and sits below the plugins: it depends on the
# sdk and on other libraries (ADR-0042 §2 — cargo itself refuses a cycle), and any plugin
# may depend on it.
libraries = {n for n in ws if (members[n].get("metadata") or {}).get("bingo", {}).get("tier") == "library"}
# A composition (ADR-0020, amended 2026-09-07) is a piece of the binary that
# was moved out of it: it may depend on whatever the binary may, and only the
# binary may depend on it.
compositions = {n for n in ws if (members[n].get("metadata") or {}).get("bingo", {}).get("tier") == "composition"}
plugins = {n for n in ws if n not in ("bingo", "bingo-sdk", "bingo-core") and n not in libraries and n not in compositions}
bad = []
for n in ws:
    if n.startswith("bingo-mod-") and n not in plugins:
        bad.append(f"{n} must be a plugin, never a library or composition")
for n in libraries:
    for d in deps(n) & ws:
        if d != "bingo-sdk" and d not in libraries:
            bad.append(f"{n} -> {d} (a library depends on bingo-sdk and other libraries only)")
for n in ws - {"bingo"} - compositions:
    for d in deps(n) & compositions:
        bad.append(f"{n} -> {d} (a composition is the binary's own; only bingo may depend on it)")
for n in plugins:
    for d in deps(n) & ws:
        if d == "bingo-core":
            bad.append(f"{n} -> bingo-core (plugins depend on bingo-sdk only)")
        elif d in plugins:
            bad.append(f"{n} -> {d} (plugin -> plugin; use a service trait via the sdk)")
for d in deps("bingo-core") & ws:
    # ADR-0087 permits the pure digest library for approval fingerprints.
    if d not in {"bingo-sdk", "bingo-digest"}:
        bad.append(f"bingo-core -> {d} (the kernel never imports a plugin)")
for n in ws:
    if n != "bingo-surface-tui":
        for d in deps(n):
            if d in ("ratatui", "crossterm"):
                bad.append(f"{n} -> {d} (only bingo-surface-tui may depend on the terminal stack)")
if bad:
    print("dependency rule violations:")
    for b in bad: print("  " + b)
    sys.exit(1)
print("dependency direction ok")
PY

# 2. Kernel and sdk purity: no heavy crates anywhere in their resolved normal tree.
for crate in bingo-sdk bingo-core; do
  if cargo tree -p "$crate" -e normal --prefix none 2>/dev/null | awk '{print $1}' | sort -u | grep -Ex 'reqwest|rmcp|ratatui|crossterm|image|syntect' ; then
    say "$crate resolves a forbidden crate (see above)"; fail=1
  fi
done

# 3. File size: non-test lines per .rs file (warn 700, fail 1000).
while IFS= read -r f; do
  n=$(awk '/^#\[cfg\(test\)\]/{exit} {c++} END{print c+0}' "$f")
  if [ "$n" -gt 1000 ]; then say "FAIL $f: $n non-test lines (>1000)"; fail=1
  elif [ "$n" -gt 700 ]; then say "warn $f: $n non-test lines (>700)"; fi
done < <(find crates -name '*.rs' -not -path '*/target/*')

# 4. Feature nouns must not appear in the kernel or sdk (identifiers only, case-insensitive).
nouns=$(grep -rEinw 'room|team|hire|experience|schedule' crates/bingo-sdk/src crates/bingo-core/src --include='*.rs' 2>/dev/null | grep -vE '^[^:]+:[0-9]+:\s*//' || true)
if [ -n "$nouns" ]; then
  say "feature noun found in kernel/sdk code:"; say "$nouns" | head; fail=1
fi
# 4b. `agent` is a plugin noun too (ADR-0010): as an identifier in the kernel it is a leak;
#     in a string it is prose (the system prompt calls the product an agent) or a test's surface name;
#     a string continued with a trailing backslash is a string line too.
#     ADR-0076 opens one door: an external agent is an endpoint the kernel drives, and the
#     seam's own modules (and tests) name it. Anywhere else the word is still a leak.
seam='^crates/bingo-sdk/src/(agent|lib|plugin)\.rs:|^crates/bingo-core/src/turn(/agent|/endpoint)?\.rs:|^crates/bingo-core/src/host/(choose|refresh|registry|catalog)\.rs:|^crates/bingo-core/src/(test_support|session/tests)\.rs:|/tests/'
agents=$(grep -rEinw 'agents?' crates/bingo-sdk/src crates/bingo-core/src --include='*.rs' 2>/dev/null | grep -vE '^[^:]+:[0-9]+:\s*//' | grep -v '"' | grep -vE '\\$' | grep -vE "$seam" || true)
if [ -n "$agents" ]; then
  say "agent noun found in kernel/sdk code:"; say "$agents" | head; fail=1
fi

# 4b. A surface is a client of the one event stream (ADR-0002, ADR-0007): no private mirror enums.
mirrors=$(grep -rEn '^\s*(pub(\([a-z]+\))?\s+)?enum\s+[A-Za-z]*Event\b' crates/bingo-surface-*/src --include='*.rs' 2>/dev/null || true)
if [ -n "$mirrors" ]; then
  say "event mirror enum in a surface crate (surfaces fold bingo_sdk::Event, they do not redefine it):"; say "$mirrors" | head; fail=1
fi

# 4c. The kernel knows no tool by name: a plugin's tool named in kernel or sdk code — a prompt line,
#     a match arm — is a leak, and only a test fixture may spell one. Test modules are cut off at
#     `#[cfg(test)]`, as the cohesion check cuts them.
python3 - <<'PY' || fail=1
import re, sys, pathlib
# Names that are only ever a tool's, anywhere; names that are also words (Read, Write, Edit)
# only quoted or backticked as a whole token.
names = r"\b(SpawnAgent|SendMessage|ListAgents|ListModels|Listen|Seat|Unseat|CloseRoom|TaskCreate|TaskUpdate|TaskGet|TaskList|AskUserQuestion|WebFetch|WebSearch|Bash|Glob|Grep|Skill)\b|[`\"](Read|Write|Edit)[`\"]"
bad = []
for f in list(pathlib.Path("crates/bingo-sdk/src").rglob("*.rs")) + list(pathlib.Path("crates/bingo-core/src").rglob("*.rs")):
    if "test" in f.name or "tests" in f.parts:
        continue
    src = f.read_text(encoding="utf-8").split("#[cfg(test)]")[0]
    for n, line in enumerate(src.splitlines(), 1):
        if line.lstrip().startswith("//"):
            continue
        if re.search(names, line):
            bad.append(f"{f}:{n}: {line.strip()}")
if bad:
    print("a tool named in kernel/sdk code:")
    for b in bad: print("  " + b)
    sys.exit(1)
print("kernel names no tool")
PY

# 4d. The kernel spells no permission mode (ADR-0039 §2): the policy owns its own
#     vocabulary, and a door asks it for a stance rather than learning its words.
modes=$(grep -rEinw 'bypass|bypassPermissions|dontAsk' crates/bingo-sdk/src crates/bingo-core/src --include='*.rs' 2>/dev/null | grep -vE '^[^:]+:[0-9]+:\s*//' || true)
if [ -n "$modes" ]; then
  say "a permission mode named in kernel/sdk code:"; say "$modes" | head; fail=1
fi

# 4e. Shared service users declare their dependency (ADR-0069 §3).
python3 - <<'PYSERVICES' || fail=1
import pathlib, re, sys
contracts = {}
for path in pathlib.Path("crates/bingo-services/src").glob("*.rs"):
    match = re.search(r'ServiceKey::new\("([^"\n]+)"\)', path.read_text())
    if match:
        contracts[path.stem] = match.group(1)
bad = []
for crate in pathlib.Path("crates").iterdir():
    if crate.name == "bingo-services" or not (crate / "src").is_dir():
        continue
    sources = []
    for path in (crate / "src").rglob("*.rs"):
        if "tests" in path.parts or "test" in path.name:
            continue
        # A test-only module declaration can precede the production manifest.
        text = re.sub(r"#\[cfg\(test\)\]\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+;", "", path.read_text())
        sources.append(text.split("#[cfg(test)]")[0])
    source = "\n".join(sources)
    declarations = " ".join(re.findall(r"(?:provides|requires|uses):\s*&\[([^]]*)\]", source, re.S))
    for module, key in contracts.items():
        if not re.search(r"bingo_services::(?:" + module + r"\b|\{[^}]*\b" + module + r"\b)", source):
            continue
        aliases = {module} | set(re.findall(r"bingo_services::" + module + r"\s+as\s+(\w+)", source))
        declared = any(re.search(r"\b" + alias + r"::(?:KEY\.name\(\)|CAPABILITY)", declarations) for alias in aliases)
        declared |= f'"{key}"' in declarations or f'"service:{key}"' in declarations
        if not declared:
            bad.append(f"{crate.name}: {key} is used but absent from provides, requires and uses")
if bad:
    print("undeclared shared service lookups:")
    print("\n".join("  " + line for line in bad))
    sys.exit(1)
print("shared service declarations ok")
PYSERVICES

# 4f. A TUI draws placements and library records, never a producer's private contract.
python3 - <<'PYTUI' || fail=1
import pathlib, re, sys
root = pathlib.Path("crates/bingo-surface-tui/src")
lib = (root / "lib.rs").read_text()
test_modules = set(re.findall(r"#\[cfg\(test\)\]\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)", lib))
tools = set("SpawnAgent SendMessage ListAgents ListModels Listen Seat Unseat CloseRoom TaskCreate TaskUpdate TaskGet TaskList AskUserQuestion WebFetch WebSearch Bash Glob Grep Skill Read Write Edit".split())
bad = []
for path in root.rglob("*.rs"):
    relative = path.relative_to(root)
    if relative.parts[0].removesuffix(".rs") in test_modules or "tests" in relative.parts or path.name == "acp.rs":
        continue
    source = re.sub(r"#\[cfg\(test\)\]\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+;", "", path.read_text())
    source = source.split("#[cfg(test)]")[0]
    for number, line in enumerate(source.splitlines(), 1):
        if line.lstrip().startswith("//"):
            continue
        for literal in re.findall(r'"([^"\n]*)"', line):
            if literal in tools or (literal.startswith(("bingo.", "_bingo.", "bingo-mod-", "bingo_mod_")) and not (path.name == "lib.rs" and literal == "bingo.surface.tui")):
                bad.append(f"{path}:{number}: {literal}")
if bad:
    print("TUI names a plugin or tool:")
    print("\n".join("  " + finding for finding in bad))
    sys.exit(1)
print("TUI names no plugin or tool")
PYTUI

# 5. Struct field count ≤ 16 and inherent impl spread ≤ 3 files per type (best effort, grep-based;
#    the third file is ADR-0011 §4: the session actor is its loop, its interactions and its inputs).
python3 - <<'PY' || fail=1
import re, sys, pathlib
bad = []
impls = {}
for f in pathlib.Path("crates").rglob("*.rs"):
    src = f.read_text(encoding="utf-8")
    src = src.split("#[cfg(test)]")[0]
    for m in re.finditer(r"pub(?:\([^)]*\))?\s+struct\s+(\w+)[^{;]*\{([^}]*)\}", src, re.S):
        fields = [l for l in m.group(2).splitlines() if re.match(r"\s*(pub(?:\([^)]*\))?\s+)?\w+\s*:", l)]
        if len(fields) > 16:
            bad.append(f"{f}: struct {m.group(1)} has {len(fields)} fields (>16)")
    for m in re.finditer(r"^\s*impl(?:<[^>]*>)?\s+(\w+)(?:<[^>]*>)?\s*\{", src, re.M):
        impls.setdefault((f.parts[1], m.group(1)), set()).add(str(f))
for (crate, ty), files in impls.items():
    if len(files) > 3:
        bad.append(f"{crate}: inherent impl of {ty} spread over {len(files)} files: {sorted(files)}")
if bad:
    print("cohesion violations:")
    for b in bad: print("  " + b)
    sys.exit(1)
print("cohesion ok")
PY

# 6. Function length: physical lines from `fn` to its closing brace (warn 60, fail 120).
python3 - <<'PY' || fail=1
import re, sys, pathlib
WARN, FAIL = 60, 120
bad = 0
def bodies(src):
    # Yield (name, start_line, end_line) for every fn with a block body.
    lines = src.splitlines()
    i = 0
    while i < len(lines):
        m = re.match(r"\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+|const\s+|unsafe\s+)*fn\s+(\w+)", lines[i])
        if not m:
            i += 1; continue
        depth, j, opened = 0, i, False
        while j < len(lines):
            code = re.sub(r'"(?:\\.|[^"\\])*"', '""', lines[j]).split("//")[0]
            if not opened and ";" in code and "{" not in code:
                break  # a trait signature, no body
            for ch in code:
                if ch == "{": depth += 1; opened = True
                elif ch == "}": depth -= 1
            if opened and depth == 0:
                yield m.group(1), i + 1, j + 1
                break
            j += 1
        i = j + 1 if opened else i + 1
for f in sorted(pathlib.Path("crates").rglob("*.rs")):
    if f.name == "tests.rs" or "tests" in f.parts or "test_support" in f.name:
        continue  # a test may be as long as its scenario
    src = f.read_text(encoding="utf-8").split("#[cfg(test)]")[0]
    for name, start, end in bodies(src):
        n = end - start + 1
        if n > FAIL:
            print(f"FAIL {f}:{start} fn {name} is {n} lines (>{FAIL})"); bad += 1
        elif n > WARN:
            print(f"warn {f}:{start} fn {name} is {n} lines (>{WARN})")
sys.exit(1 if bad else 0)
PY

# 7. Record length (CLAUDE.md, Records): a plan warns above 150 lines, an ADR above 120.
#    A warning, never a failure — the records already written are history, and history is
#    not rewritten to satisfy a check added after it.
long_records() { # directory, name glob, limit
  while IFS= read -r f; do
    n=$(awk 'END{print NR}' "$f")
    if [ "$n" -gt "$3" ]; then say "warn $f: $n lines (>$3)"; fi
  done < <(find "$1" -name "$2" | sort)
}
long_records docs/plans 'M*.md' 150
long_records docs/adr '0*.md' 120

# 8. Advisory test discipline: new hosts should use the SDK double, and
#    liveness should time out while waiting rather than assert machine speed.
python3 - <<'PYTEST'
import pathlib, re
allowed = {
    "crates/bingo-core/src/host.rs",
    "crates/bingo-core/src/host/unavailable.rs",
    "crates/bingo-core/src/session/commands.rs",
    "crates/bingo-core/src/test_support.rs",
    "crates/bingo-core/src/test_support/peers.rs",
    "crates/bingo-surface-rpc/src/client.rs",
    "crates/bingo-surface-rpc/tests/wire/host.rs",
    "crates/bingo-sdk/src/testing.rs",
    "crates/bingo-sdk/src/testing/api.rs",
}
for path in sorted(pathlib.Path("crates").rglob("*.rs")):
    source = path.read_text(encoding="utf-8")
    if str(path) not in allowed:
        for match in re.finditer(r"impl\s+(?:\w+::)*HostApi\s+for\b", source):
            line = source.count("\n", 0, match.start()) + 1
            print(f"warn {path}:{line}: use bingo_sdk::testing::TestHost or explain the retained host")
    is_test_file = any("test" in part for part in path.parts)
    start = 0 if is_test_file else source.find("#[cfg(test)]")
    if start < 0:
        continue
    for match in re.finditer(r"assert!\s*\([^;]*?\);", source[start:], re.S):
        if re.search(r"elapsed\(\)(?:\.as_\w+\(\))?\s*<", match.group()):
            line = source.count("\n", 0, start + match.start()) + 1
            print(f"warn {path}:{line}: assert behavior or wrap the wait in a timeout")
print("test host and wall-clock advisory checks complete")
PYTEST

# Kernel catalogue entries have one declaration site (ADR-0078).
python3 - <<'PYCATALOGUE' || fail=1
import pathlib, re, sys
bad = []
for path in pathlib.Path("crates").rglob("*.rs"):
    if str(path) == "crates/bingo-sdk/src/catalogue.rs":
        continue
    source = path.read_text()
    uses_entry = re.search(r"use\s+[^;]*catalogue[^;]*\bEntry\b", source)
    for match in re.finditer(r"(?:(?:bingo_sdk::)?catalogue::)?\bEntry\s*\{", source):
        if "catalogue::" not in match.group() and not uses_entry:
            continue
        prefix = source[max(0, match.start() - 80):match.start()]
        if re.search(r"(?:struct|impl|->\s*&?(?:\s*'\w+)?)\s*$", prefix):
            continue
        line = source.count("\n", 0, match.start()) + 1
        bad.append(f"{path}:{line}: declare catalogue Entry literals in bingo-sdk/src/catalogue.rs")
for problem in bad:
    print(problem)
if bad:
    sys.exit(1)
print("catalogue entry declarations ok")
PYCATALOGUE

[ "$fail" -eq 0 ] && say "discipline ok" || { say "discipline FAILED"; exit 1; }
