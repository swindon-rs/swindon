import pytest
import pathlib
import subprocess
import tempfile
import os
import json

from aiohttp import web, test_utils
# Command line options


def pytest_addoption(parser):
    parser.addoption('--swindon-bin', default=[],
                     type=pathlib.Path,
                     action='append',
                     help="Path to swindon binary to run")


SWINDON_BIN = []


def pytest_configure(config):
    root = pathlib.Path('/work')
    bins = config.getoption('--swindon-bin')[:]
    SWINDON_BIN[:] = bins or [pathlib.Path('target/debug/swindon')]
    for _ in range(len(SWINDON_BIN)):
        p = SWINDON_BIN.pop(0)
        p = root / p
        assert p.exists(), p
        SWINDON_BIN.append(str(p))

# Fixtures


@pytest.fixture(params=SWINDON_BIN)
def swindon(_proc, request):
    swindon_bin = request.param
    fd, fname = tempfile.mkstemp()

    def configure(**config):
        config.setdefault('listen', [])
        if not config['listen']:
            config['listen'].append("127.0.0.1:8080")
        config.setdefault('debug-routing', True)
        os.write(fd, json.dumps(config, indent=2).encode("utf-8"))
        proc = _proc(swindon_bin,
                     '--verbose',
                     '--config',
                     fname,
                     # TODO: add config
                     stdout=subprocess.PIPE,
                     stderr=subprocess.STDOUT)
        addr = set(config['listen'])
        while True:
            assert proc.poll() is None, (proc.poll(), proc.stdout.read())
            line = proc.stdout.readline().decode('utf-8').strip()
            for a in list(addr):
                if line.endswith(a):
                    addr.discard(a)
            if not addr:
                break
        return proc
    try:
        yield configure
    finally:
        os.close(fd)
        os.remove(fname)


@pytest.fixture
def swindon_client(loop):
    clients = []

    async def go(__param, *args, **kwargs):
        if not isinstance(__param, web.Application):
            __param = __param(loop, *args, **kwargs)
        client = test_utils.TestClient(__param)
        await client.start_server()
        clients.append(client)
        return client

    async def finalize():
        while clients:
            await (clients.pop()).close()

    try:
        yield go
    finally:
        loop.run_until_complete(finalize())

# helpers


@pytest.fixture
def _proc():
    # Process runner
    processes = []

    def run(*cmdline, **kwargs):
        cmdline = list(map(str, cmdline))
        proc = subprocess.Popen(cmdline, **kwargs)
        processes.append(proc)
        return proc

    try:
        yield run
    finally:
        while processes:
            proc = processes.pop(0)
            proc.terminate()
            proc.wait()
