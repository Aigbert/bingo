#!/usr/bin/env python3
"""Exercise edge flows with the fake provider in an owned tmux server."""
import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
BIN = ROOT / 'target/debug/bingo'
SOCKET = 'm199-drive'


def drive(width, height):
    with tempfile.TemporaryDirectory(prefix='m199-drive-') as temporary:
        work = Path(temporary)
        env = dict(os.environ)
        for key in ('OPENAI_API_KEY', 'OPENAI_BASE_URL', 'ANTHROPIC_API_KEY', 'ANTHROPIC_BASE_URL', 'NO_COLOR', 'BINGO_ASCII'):
            env.pop(key, None)
        for key, child in (('HOME', 'home'), ('XDG_DATA_HOME', 'data'), ('XDG_CONFIG_HOME', 'config')):
            (work / child).mkdir()
            env[key] = str(work / child)
        (work / 'cwd').mkdir()
        (work / 'home/.bingo').mkdir()
        (work / 'home/.bingo/settings.json').write_text(json.dumps({'update': {'check': False}}))
        env.update(TERM='xterm-256color', COLORTERM='truecolor', BINGO_THEME='dark', BINGO_MOTION='on', BINGO_GRAPHICS='off')
        if subprocess.run(['tmux', '-L', SOCKET, 'list-sessions'], capture_output=True).returncode == 0:
            raise RuntimeError('owned socket is already in use')
        session = 'edges'
        started = False

        def tmux(*args):
            return subprocess.check_output(['tmux', '-L', SOCKET, *args], env=env, text=True, stderr=subprocess.DEVNULL)

        def pane():
            return tmux('capture-pane', '-p', '-t', session)

        def wait(text):
            until = time.monotonic() + 30
            while time.monotonic() < until:
                shown = pane()
                if text in shown:
                    return shown
                time.sleep(.05)
            raise AssertionError(f'missing {text!r}:\n{shown}')

        def keys(*chords):
            tmux('send-keys', '-t', session, *chords)

        def text(value):
            tmux('send-keys', '-t', session, '-l', value)

        def capture(name):
            (OUT / f'{name}-{width}x{height}.txt').write_text(pane().replace(temporary, '<drive>'))

        def open_session(name, responses, extra=()):
            nonlocal session, started
            session = name
            script = work / f'{name}.json'
            script.write_text(json.dumps({'responses': responses}))
            child = dict(env, BINGO_FAKE_SCRIPT=str(script))
            names = {'HOME', 'XDG_DATA_HOME', 'XDG_CONFIG_HOME', 'BINGO_FAKE_SCRIPT', 'BINGO_THEME', 'BINGO_MOTION', 'BINGO_GRAPHICS', 'TERM', 'COLORTERM'}
            command = shlex.join(['env', *[f'{k}={v}' for k, v in child.items() if k in names], str(BIN), '--provider', 'fake', '--cwd', str(work / 'cwd'), '--allowed-tools', 'Bash,Read', *extra])
            tmux('-f', '/dev/null', 'new-session', '-d', '-s', session, '-x', str(width), '-y', str(height), '-c', str(work / 'cwd'), command)
            started = True
            tmux('set-option', '-t', session, 'remain-on-exit', 'on')

        def leave():
            wait('/ commands')
            keys('C-c')
            wait('again')
            keys('C-c')
            wait('resume with bingo --resume')

        try:
            open_session('fresh', [{'steps': [{'text': 'First answer.'}]}])
            wait('/ commands')
            time.sleep(1)
            capture('welcome')
            text('first edge')
            keys('Enter')
            wait('First answer.')
            leave()
            capture('receipt')
            open_session('recent', [{'steps': [{'text': 'Unused.'}]}])
            wait('recent')
            keys('Up')
            wait('resume')
            capture('welcome-recent')
            keys('Enter')
            wait('resumed')
            capture('resumed')
            leave()
            open_session('retry', [
                {'steps': [{'error': {'kind': 'rateLimited', 'retryAfterMs': 3500}}]},
                {'steps': [{'text': 'Recovered from rate limit.'}]},
            ])
            wait('/ commands')
            text('retry this edge')
            keys('Enter')
            wait('rate limited')
            capture('retry')
            wait('Recovered from rate limit.')
            capture('retry-recovered')
            leave()
            open_session('compact', [
                {'steps': [{'text': f'Context answer {n}.'}]}
                for n in range(8)
            ])
            wait('/ commands')
            for n in range(8):
                text(f'keep context {n}')
                keys('Enter')
                wait(f'Context answer {n}.')
            text('/compact')
            keys('Enter')
            wait('compacted')
            capture('compacted')
            leave()
            open_session('background', [
                {'steps': [{'toolCall': {'name': 'Bash', 'input': {'command': 'sleep 3; echo background-done'}}}]},
                {'steps': [{'text': 'The shell is running independently.'}]},
                {'steps': [{'text': 'The background result arrived.'}]},
            ])
            wait('/ commands')
            text('run a background shell')
            keys('Enter')
            wait('ctrl+b')
            keys('C-b')
            wait('in background')
            capture('background')
            wait('finished in the background')
            capture('background-notified')
            wait('/ commands')
            leave()
            print(f'PASS {width}x{height}: fresh, recent/resume, retry/recovery, compact, background, receipt')
        finally:
            if started:
                tmux('kill-server')
            print(f'CLEAN {width}x{height}: owned server and temporary HOME/data/cwd')


if __name__ == '__main__':
    for size in ((100, 32), (80, 24)):
        drive(*size)
