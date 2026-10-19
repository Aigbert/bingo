#!/usr/bin/env python3
"""Exercise M195 against a fake provider in an owned, disposable tmux server."""
import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
BIN = Path(os.environ.get('CARGO_TARGET_DIR', ROOT / 'target')) / 'debug/bingo'


def drive(width, height):
    with tempfile.TemporaryDirectory(prefix='m195-drive-') as directory:
        work = Path(directory)
        socket = work.name
        env = dict(os.environ)
        for key in ('OPENAI_API_KEY', 'OPENAI_BASE_URL', 'ANTHROPIC_API_KEY',
                    'ANTHROPIC_BASE_URL', 'NO_COLOR', 'BINGO_ASCII'):
            env.pop(key, None)
        for key, name in (('HOME', 'home'), ('XDG_DATA_HOME', 'data'), ('XDG_CONFIG_HOME', 'config')):
            (work / name).mkdir()
            env[key] = str(work / name)
        cwd = work / 'cwd'
        cwd.mkdir()
        (work / 'home/.bingo').mkdir()
        (work / 'home/.bingo/settings.json').write_text('{"update":{"check":false}}')
        (cwd / 'note.txt').write_text(''.join(f'source line {n}\n' for n in range(1, 31)))
        calls = [
            ('Read', {'file_path': 'note.txt', 'offset': 10, 'limit': 16}),
            ('Edit', {'file_path': 'note.txt', 'old_string': 'source line 12', 'new_string': 'revised line 12'}),
            ('Bash', {'command': "printf 'command line 1\\ncommand line 2\\ncommand line 3\\ncommand line 4\\ncommand line 5\\ncommand line 6\\ncommand line 7\\ncommand line 8\\ncommand line 9\\ncommand line 10\\ncommand line 11\\ncommand line 12\\ncommand line 13\\ncommand line 14\\n'"}),
        ]
        responses = [{'steps': [{'toolCall': {'name': name, 'input': args}}]} for name, args in calls]
        responses.append({'steps': [{'text': 'Detail drive finished. Find source or revised.'}]})
        script = work / 'fake.json'
        script.write_text(json.dumps({'responses': responses}))
        env.update(BINGO_FAKE_SCRIPT=str(script), BINGO_THEME='dark', BINGO_MOTION='on',
                   BINGO_GRAPHICS='off', COLORTERM='truecolor', TERM='xterm-256color')

        def tmux(*args):
            return subprocess.check_output(['tmux', '-L', socket, *args], env=env, text=True)

        def pane(escapes=False):
            return tmux('capture-pane', '-p', *(['-e'] if escapes else []), '-t', 'drive')

        def keys(*args):
            tmux('send-keys', '-t', 'drive', *args)
            time.sleep(0.12)

        def escape():
            keys('-l', '\x1b[27u')

        def await_text(needle):
            end = time.monotonic() + 30
            while time.monotonic() < end:
                text = pane()
                if needle in text:
                    return
                time.sleep(0.05)
            raise AssertionError(f'Expected {needle!r}:\n{text}')

        def capture(name):
            time.sleep(0.3)
            for extension, escapes in [('txt', False), ('ansi', True)]:
                text = pane(escapes).replace(work.name, 'm195-drive-XXXXXXXX')
                if escapes:
                    assert '38;2;' in text
                (OUT / f'{name}-{width}x{height}.{extension}').write_text(text)

        started = False
        try:
            command = shlex.join([str(BIN), '--cwd', str(cwd), '--permission-mode', 'bypassPermissions'])
            tmux('-f', '/dev/null', 'new-session', '-d', '-s', 'drive', '-c', str(cwd),
                 '-x', str(width), '-y', str(height), command)
            started = True
            await_text('fake-1')
            keys('-l', 'Read, edit, and print the source lines.')
            keys('Enter')
            await_text('Detail drive finished.')
            capture('done')
            keys('C-o')
            await_text('step')
            capture('detail')
            keys('Enter')
            await_text('10   source line 10')
            await_text('9 more lines')
            capture('read-well')
            keys('Down', 'Enter')
            await_text('revised line 12')
            capture('diff-well')
            keys('Down', 'Enter')
            await_text('2 earlier lines')
            await_text('command line 14')
            capture('command-well')
            keys('Up')
            escape()
            capture('folded')
            assert '2 earlier lines' not in pane()
            keys('C-f')
            keys('-l', 'source')
            await_text('of')
            capture('find')
            keys('Up')
            capture('find-up')
            escape()
            capture('find-back')
            assert 'matches' not in pane().splitlines()[-1]
            print(f'PASS {width}x{height}: detail, read/diff/command wells, find, escape; fake provider', flush=True)
        finally:
            if started:
                tmux('kill-server')
                print(f'CLEAN {width}x{height}: owned server stopped; temporary HOME/data/cwd removed', flush=True)


if __name__ == '__main__':
    drive(100, 32)
    drive(80, 24)
