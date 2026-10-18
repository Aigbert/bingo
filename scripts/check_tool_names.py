#!/usr/bin/env python3
"""Check the tool-name boundary, excluding test items rather than file tails."""
import pathlib
import re
import sys

TOOLS = ('SpawnAgent SendMessage ListAgents ListModels Listen Seat Unseat CloseRoom '
         'TaskCreate TaskUpdate TaskGet TaskList AskUserQuestion WebFetch WebSearch '
         'Bash Glob Grep Skill StopAgent DismissAgent SetThinking OpenRoom ShowPage '
         'BashOutput KillShell SendFile').split()
KERNEL = re.compile(r'\b(?:' + '|'.join(TOOLS) + r')\b|[`"](?:Read|Write|Edit)[`"]')
DRAWING = re.compile(r'"(?:' + '|'.join(TOOLS + ['Read', 'Write', 'Edit']) + r')"')
MODULE = re.compile(r'#\[cfg\(test\)\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*([;{])')
# Strings and comments cannot open or close an inline module.
TOKEN = re.compile(r"""'(?:\\(?:u\{[0-9a-fA-F]+\}|.)|[^'\\])'|r(\#*)".*?"\1|"(?:\\.|[^"\\])*"|//[^\n]*|/\*.*?\*/|[{}]""", re.S)


def production(source):
    external = []
    for match in reversed(list(MODULE.finditer(source))):
        end = match.end()
        if match[2] == ';':
            path = re.search(r'#\[path\s*=\s*"([^"]+)"\]', match[0])
            external.append(path[1] if path else match[1])
        else:
            depth = 1
            for token in TOKEN.finditer(source, end):
                depth += (token[0] == '{') - (token[0] == '}')
                if depth == 0:
                    end = token.end()
                    break
        removed = source[match.start():end]
        source = source[:match.start()] + re.sub(r'[^\n]', ' ', removed) + source[end:]
    return source, external


def violations(source, pattern):
    source, _ = production(source)
    return [(n, line.strip()) for n, line in enumerate(source.splitlines(), 1)
            if not line.lstrip().startswith('//') and pattern.search(line)]


def self_test():
    source = '#[cfg(test)] mod tests { fn fixture() { let x = "Bash"; } }\nconst NAME: &str = "Bash";'
    assert violations(source, DRAWING) == [(2, 'const NAME: &str = "Bash";')]
    assert not violations(source.split('\n')[0], DRAWING)
    assert violations('let prompt = "Use Bash, Grep, Skill and `Read`.";', KERNEL)
    assert production('#[cfg(test)] mod samples;\nfn kept() {}')[1] == ['samples']
    assert 'fn kept' in production('#[cfg(test)] mod samples;\nfn kept() {}')[0]
    chars = "#[cfg(test)] mod tests { fn fixture() { let quote = '\"'; let brace = '}'; let x = \"Bash\"; } }\nconst NAME: &str = \"Bash\";"
    assert violations(chars, DRAWING) == [(2, 'const NAME: &str = "Bash";')]
    attributed = '#[cfg(test)]\n#[path = "fixtures.rs"]\nmod samples;\nconst NAME: &str = "Bash";'
    assert production(attributed)[1] == ['fixtures.rs']
    assert violations(attributed, DRAWING) == [(4, 'const NAME: &str = "Bash";')]
    inline = '#[cfg(test)]\n#[allow(dead_code)]\nmod tests { const NAME: &str = "Bash"; }\nconst NAME: &str = "Bash";'
    assert violations(inline, DRAWING) == [(4, 'const NAME: &str = "Bash";')]
    print('tool-name check self-test ok')


def check():
    bad = []
    for crate in ['bingo-sdk', 'bingo-core', 'bingo-surface-tui', 'bingo-mod-ledger']:
        root = pathlib.Path('crates') / crate / 'src'
        sources = {path: path.read_text() for path in root.rglob('*.rs')}
        excluded = set()
        for path, source in sources.items():
            parent = path.parent if path.stem in ('lib', 'mod') else path.with_suffix('')
            for name in production(source)[1]:
                if name.endswith('.rs'):
                    excluded.add(path.parent / name)
                    continue
                excluded.add(parent / (name + '.rs'))
                excluded.update((parent / name).rglob('*.rs'))
        pattern = KERNEL if crate in ('bingo-sdk', 'bingo-core') else DRAWING
        for path, source in sources.items():
            if path in excluded:
                continue
            bad.extend(f'{path}:{n}: {line}' for n, line in violations(source, pattern))
    if bad:
        print('a tool named in kernel/sdk/surface/drawing code:\n' + '\n'.join(bad))
        return 1
    print('kernel and step drawings name no tool')
    return 0


if __name__ == '__main__':
    self_test()
    sys.exit(check())
