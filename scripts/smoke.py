"""Exercise built binaries on each release OS, using temporary server configuration."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import urllib.request
import urllib.error
import socket

prefix = os.environ.get("SMOKE_PATH_PREFIX", "")
server, cli = map(lambda p: str(Path(p).resolve()), sys.argv[1:])
with tempfile.TemporaryDirectory(prefix='pocketshare-') as tmp:
    root = Path(tmp)
    env = dict(os.environ, XDG_CONFIG_HOME=tmp, APPDATA=tmp, LOCALAPPDATA=tmp)
    def run(*args):
        return subprocess.check_output([cli, *args], env=env, text=True).strip()
    # First-run configuration must be parseable on Windows as well.
    first = subprocess.Popen([server], env=env, stdout=subprocess.DEVNULL)
    try:
        generated = root / 'pocket-share-server' / 'config.toml'
        for _ in range(100):
            if generated.exists() and generated.stat().st_size:
                break
            time.sleep(.1)
    finally:
        first.terminate()
        first.wait(timeout=10)
    import tomllib
    tomllib.loads(generated.read_text(encoding="utf-8"))
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    config = root / 'smoke.toml'
    config.write_text(f'path_prefix = "{prefix}"\nlisten_addr = "127.0.0.1:{port}"\ndata_dir = "{root.as_posix()}/data"\n')
    process = subprocess.Popen([server, '--config', str(config)], env=env)
    base = f'http://127.0.0.1:{port}{prefix}'
    try:
        for _ in range(100):
            try:
                with urllib.request.urlopen(base + '/health', timeout=1) as response:
                    assert response.read() == b'ok'
                break
            except OSError:
                time.sleep(.1)
        else:
            raise RuntimeError('server did not start')
        try:
            urllib.request.urlopen(base + '/list')
            raise AssertionError('unauthenticated request accepted')
        except urllib.error.HTTPError as error:
            assert error.code == 401
        run('login', (root / 'data/api_key').read_text(), '--server', base)
        key = run('put', 'cross-platform smoke test')
        assert run('get', key) == 'cross-platform smoke test'
        source = root / 'upload.bin'
        source.write_bytes(bytes(range(256)) * 8)
        file_key = run('put', str(source))
        target = root / 'download.bin'
        run('get', file_key, str(target))
        assert target.read_bytes() == source.read_bytes()
        assert key in run('list')
        run('delete', key)
        assert key not in run('list')
        with urllib.request.urlopen(base + '/') as response:
            assert b'PocketShare' in response.read()
        assert (root / 'data/pocketshare.sqlite').is_file()
        for route in ['/', '/manifest.webmanifest', '/sw.js']:
            with urllib.request.urlopen(base + route) as response:
                body = response.read().decode()
                assert '__POCKETSHARE_PATH_PREFIX__' not in body
                if prefix:
                    assert prefix in body
        print('Binary smoke tests passed')
    finally:
        process.terminate()
        process.wait(timeout=10)
