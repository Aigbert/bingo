#!/usr/bin/env bash
# Originals are local inputs, never repository fixtures. No source is printed.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
exec python3 - "$ROOT" "$@" <<'PY'
import difflib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def destination(root, value):
    out = Path(value).resolve()
    temporary = (Path(tempfile.gettempdir()).resolve(), Path('/tmp').resolve())
    if out.is_relative_to(root) or not any(out.is_relative_to(p) for p in temporary):
        raise ValueError('output must be in a temporary directory outside the repository')
    out.mkdir(parents=True, exist_ok=True)
    if any(out.iterdir()):
        raise ValueError('output directory must be empty')
    return out


def port(source, target):
    before = source.read_bytes()
    after = before.replace(b"'claude-code'", b"'bingo'").replace(b'"claude-code"', b'"bingo"')
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(after)
    changes = list(difflib.unified_diff(before.decode().splitlines(), after.decode().splitlines(), n=0))
    removed = sum(line.startswith('-') and not line.startswith('---') for line in changes)
    added = sum(line.startswith('+') and not line.startswith('+++') for line in changes)
    assert source.read_bytes() == before
    print(f'{source.name}: diff -U0: -{removed}/+{added} lines; package specifier only; source unchanged', flush=True)


def layout(root, source, out, name, identity, extension, uses):
    directory = out / identity
    file = f'{name}.{extension}'
    port(source / file, directory / 'hooks' / file)
    contract = source / f'{name}-state.d.ts'
    if contract.is_file():
        port(contract, directory / 'types/index.d.ts')
    else:
        (directory / 'types').mkdir()
        (directory / 'types/index.d.ts').write_text('export interface PluginState {}\n')
    manifest = dict(name=identity, version='1', module=f'hooks/{file}', uses=uses)
    (directory / 'plugin.json').write_text(json.dumps(manifest) + '\n')
    (directory / 'parity-entry.ts').write_text(f"export {{register}} from './hooks/{file}';\n")
    for test in sorted((root / 'examples/plugins' / name).glob('*.parity.test.ts')):
        shutil.copyfile(test, directory / test.name)
    return directory


def main():
    if len(sys.argv) != 4:
        raise ValueError('usage: scripts/port_examples.sh <originals> <temporary-output>')
    root, source = (Path(value).resolve() for value in sys.argv[1:3])
    for file in ('band.tsx', 'band-state.d.ts', 'pane.tsx', 'pane-state.d.ts', 'tool-call.ts'):
        if not (source / file).is_file():
            raise ValueError(f'missing input: {file}')
    out = destination(root, sys.argv[3])
    binary = Path(os.environ.get('BINGO_BIN', root / 'target/debug/bingo')).resolve()
    isolated = out / '.home'
    (isolated / '.bingo').mkdir(parents=True)
    (isolated / '.bingo/settings.toml').write_text('[update]\ncheck = false\n')
    env = dict(os.environ, HOME=str(isolated), USERPROFILE=str(isolated))
    env.pop('BINGO_FAKE_SCRIPT', None)
    examples = (
        ('tool-call', 'tool-call', 'ts', ['ui']),
        ('band', 'turn-band', 'tsx', ['state', 'ui', 'clock']),
        ('pane', 'tool-calls', 'tsx', ['state', 'ui', 'command']),
    )
    failures = 0
    for name, identity, extension, uses in examples:
        directory = layout(root, source, out, name, identity, extension, uses)
        for verb in ('validate', 'test'):
            result = subprocess.run([str(binary), 'plugins', verb, str(directory)], cwd=out, env=env)
            print(f'{name} {verb}: exit {result.returncode}', flush=True)
            failures += result.returncode != 0
    return bool(failures)


try:
    sys.exit(main())
except (OSError, ValueError) as error:
    print(f'port-examples: {error}', file=sys.stderr)
    sys.exit(1)
PY
