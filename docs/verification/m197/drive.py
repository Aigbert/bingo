#!/usr/bin/env python3
"""Exercise M197's asks in the dock against a fake provider, in an owned tmux server."""
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
SOCKET = 'm197-drive'
GUARD = 0.5
GIT_ID = ['-c', 'user.name=drive', '-c', 'user.email=drive@example.test']


def git(cwd, *args):
    subprocess.run(['git', *GIT_ID, *args], cwd=cwd, check=True, capture_output=True)


def repository(work, cwd):
    """A repository whose `origin/main` has one commit the local `main` lacks."""
    remote = work / 'remote.git'
    git(work, 'init', '-q', '--bare', '-b', 'main', str(remote))
    git(cwd, 'init', '-q', '-b', 'main')
    (cwd / 'note.txt').write_text(''.join(f'source line {n}\n' for n in range(1, 9)))
    git(cwd, 'add', 'note.txt')
    git(cwd, 'commit', '-q', '-m', 'Start the note')
    git(cwd, 'commit', '-q', '--allow-empty', '-m', 'Keep the remote history')
    git(cwd, 'remote', 'add', 'origin', str(remote))
    git(cwd, 'push', '-q', 'origin', 'main')
    git(cwd, 'reset', '-q', '--hard', 'HEAD~1')


def script(work):
    question = {'questions': [
        {'question': 'Which store should the cache use?', 'header': 'Store', 'options': [
            {'label': 'Memory', 'description': 'fast, gone on restart'},
            {'label': 'Disk', 'description': 'survives a restart'}]},
        {'question': 'How long should an entry live?', 'header': 'Lifetime', 'options': [
            {'label': 'An hour', 'description': 'fresh enough'},
            {'label': 'A day', 'description': 'fewer misses'}]}]}
    calls = [
        ('Read', {'file_path': 'note.txt'}),
        ('Edit', {'file_path': 'note.txt', 'old_string': 'source line 3', 'new_string': 'revised line 3'}),
        ('Bash', {'command': 'echo first'}),
        ('Bash', {'command': 'rm -rf build'}),
        ('Bash', {'command': 'git push --force origin main'}),
        ('AskUserQuestion', question),
    ]
    responses = [{'steps': [{'toolCall': {'name': name, 'input': args}}]} for name, args in calls]
    responses.append({'steps': [{'text': 'Ask drive finished.'}]})
    path = work / 'fake.json'
    path.write_text(json.dumps({'responses': responses}))
    return path


def drive(width, height):
    with tempfile.TemporaryDirectory(prefix='m197-drive-') as directory:
        work = Path(directory)
        env = dict(os.environ)
        for key in ('OPENAI_API_KEY', 'OPENAI_BASE_URL', 'ANTHROPIC_API_KEY',
                    'ANTHROPIC_BASE_URL', 'NO_COLOR', 'BINGO_ASCII', 'TMUX'):
            env.pop(key, None)
        for key, name in (('HOME', 'home'), ('XDG_DATA_HOME', 'data'), ('XDG_CONFIG_HOME', 'config')):
            (work / name).mkdir()
            env[key] = str(work / name)
        cwd = work / 'cwd'
        cwd.mkdir()
        (work / 'home/.bingo').mkdir()
        (work / 'home/.bingo/settings.json').write_text('{"update":{"check":false}}')
        repository(work, cwd)
        env.update(BINGO_FAKE_SCRIPT=str(script(work)), BINGO_THEME='dark', BINGO_MOTION='on',
                   BINGO_GRAPHICS='off', COLORTERM='truecolor', TERM='xterm-256color')

        def tmux(*args):
            return subprocess.check_output(['tmux', '-L', SOCKET, *args], env=env, text=True)

        def pane(escapes=False):
            return tmux('capture-pane', '-p', *(['-e'] if escapes else []), '-t', 'drive')

        def keys(*args):
            tmux('send-keys', '-t', 'drive', *args)
            time.sleep(0.15)

        def typed(text):
            keys('-l', text)

        def await_text(*needles):
            end = time.monotonic() + 30
            text = ''
            while time.monotonic() < end:
                text = pane()
                if all(needle in text for needle in needles):
                    time.sleep(GUARD)
                    return
                time.sleep(0.05)
            raise AssertionError(f'Expected {needles!r}:\n{text}')

        def capture(name):
            time.sleep(0.3)
            for extension, escapes in [('txt', False), ('ansi', True)]:
                text = pane(escapes).replace(work.name, 'm197-drive-XXXXXXXX')
                (OUT / f'{name}-{width}x{height}.{extension}').write_text(text)

        started = False
        try:
            command = shlex.join([str(BIN), '--cwd', str(cwd)])
            tmux('-f', '/dev/null', 'new-session', '-d', '-s', 'drive', '-c', str(cwd),
                 '-x', str(width), '-y', str(height), command)
            started = True
            await_text('fake-1')
            typed('Revise the note, run a few commands, then ask about the cache.')
            keys('Enter')

            await_text('No, and tell it why', 'revised line 3', '↑↓ choose')
            capture('edit')
            keys('Enter')

            await_text('echo first', 'Change it first')
            capture('bash')
            keys('Down', 'Down', 'Down', 'Enter')
            await_text('⏎ send')
            capture('bash-change')
            typed(' second')
            keys('Enter')
            # The kernel gates a changed call again, so the new command asks once more.
            await_text('$ echo first second', 'Change it first')
            capture('bash-changed')
            keys('Enter')

            await_text('rm -rf build', 'No, and tell it why')
            typed('use cargo clean')
            await_text('use cargo clean')
            capture('no-reason')
            keys('Enter')

            await_text('git push --force origin main', 'rewrites remote history')
            capture('force-push')
            typed('keep the remote history')
            keys('Enter')

            await_text('Which store should the cache use?', '1 of 2')
            capture('question')
            keys('Enter')
            await_text('How long should an entry live?', '2 of 2')
            capture('question-second')
            keys('Down', 'Enter')

            await_text('Ask drive finished.')
            capture('done')
            done = pane()
            assert 'echo first second' in done, done
            print(f'PASS {width}x{height}: edit ask, bash changed first, no with a reason, '
                  'force push with stakes, two-tab question; fake provider', flush=True)
        finally:
            if started:
                tmux('kill-server')
                print(f'CLEAN {width}x{height}: tmux -L {SOCKET} stopped; temporary HOME/data/cwd removed',
                      flush=True)


if __name__ == '__main__':
    drive(100, 32)
    drive(80, 24)
