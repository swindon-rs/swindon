import pytest
import pathlib
import subprocess
import tempfile
import os
import string
import socket
import textwrap
import hashlib

import yarl
import aiohttp
import asyncio

from collections import namedtuple
from contextlib import contextmanager
from functools import partial
from aiohttp import web

ROOT = pathlib.Path('/work')


def pytest_addoption(parser):
    parser.addoption('--swindon-bin', default=[],
                     action='append',
                     help="Path to swindon binary to run")
    parser.addoption('--swindon-config',
                     default='./tests/config.yaml.tpl',
                     help=("Path to swindon config template,"
                           " default is `%(default)s`"))
    parser.addoption('--swindon-replication-config',
                     default='./tests/config-w-replication.yaml.tpl',
                     help=("Path to swindon config template"
                           " with chat replication enabled(!),"
                           " default is `%(default)s`"))
    parser.addoption('--kcov', default=None,
                     help="Path 'kcov' executable to collect coverage")
    parser.addoption('--rust-log',
                     default='debug,tokio_core=warn',
                     help=("Set RUST_LOG for swindon, default is"
                           " \"%(default)s\""))


SWINDON_BIN = []


def pytest_configure(config):
    bins = config.getoption('--swindon-bin')[:]
    SWINDON_BIN[:] = bins or ['target/debug/swindon']
    for _ in range(len(SWINDON_BIN)):
        p = SWINDON_BIN.pop(0)
        p = ROOT / p
        assert p.exists(), p
        SWINDON_BIN.append(str(p))

# Fixtures


@pytest.fixture(params=[
    'GET', 'PATCH', 'POST', 'PUT', 'UPDATED', 'DELETE', 'XXX'])
def request_method(request):
    """Parametrized fixture changing request method
    (GET / POST / PATCH / ...).
    """
    return request.param


@pytest.fixture(params=[aiohttp.HttpVersion11, aiohttp.HttpVersion10],
                ids=['http/1.1', 'http/1.0'])
def http_version(request):
    return request.param


@pytest.fixture(scope='session', params=[True, False],
                ids=['debug-routing', 'no-debug-routing'])
def debug_routing(request):
    return request.param


@pytest.fixture
def http_request(request_method, http_version, debug_routing, loop):

    async def inner(url, **kwargs):
        async with aiohttp.ClientSession(version=http_version, loop=loop) as s:
            async with s.request(request_method, url, **kwargs) as resp:
                data = await resp.read()
                assert resp.version == http_version
                assert_headers(resp.headers, debug_routing)
                return resp, data
    return inner


def assert_headers(headers, debug_routing):
    assert 'Content-Type' in headers
    assert 'Content-Length' in headers
    assert 'Date' in headers
    assert 'Server' in headers
    if debug_routing:
        assert 'X-Swindon-Route' in headers
    else:
        assert 'X-Swindon-Route' not in headers

    assert len(headers.getall('Content-Type')) == 1
    assert len(headers.getall('Content-Length')) == 1
    assert len(headers.getall('Date')) == 1
    assert headers.getall('Server') == ['swindon/func-tests']


SwindonInfo = namedtuple('SwindonInfo', 'proc url proxy api api2')


@pytest.fixture(scope='session')
def TESTS_DIR():
    return os.path.dirname(__file__)


@pytest.fixture(scope='session')
def unused_port():
    used = set()

    def find():
        while True:
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
                s.bind(('127.0.0.1', 0))
                port = s.getsockname()[1]
                if port in used:
                    continue
                used.add(port)
                return port
    return find


@pytest.fixture(scope='session')
def swindon_ports(unused_port, debug_routing, swindon_bin):
    class Dict(dict):
        def __missing__(self, key):
            self[key] = val = {
                'main': unused_port(),
                'proxy': unused_port(),
                'session_pool_1': unused_port(),
                'session_pool_2': unused_port(),
                'replication': unused_port(),
            }
            return val
    return Dict()


@pytest.fixture(scope='session', params=SWINDON_BIN)
def swindon_bin(request):
    return request.param


@pytest.fixture(scope='session')
def swindon(_proc, request, debug_routing,
            swindon_bin, swindon_ports, TESTS_DIR):
    default = swindon_ports['default']

    def to_addr(port):
        return '127.0.0.1:{}'.format(port)

    config = request.config.getoption('--swindon-config')
    rust_log = request.config.getoption('--rust-log')
    options = dict(
        swindon_port=default['main'],
        proxy_port=default['proxy'],
        session_pool1_port=default['session_pool_1'],
        session_pool2_port=default['session_pool_2'],

        listen_address=to_addr(default['main']),
        debug_routing=str(debug_routing).lower(),
        proxy_address=to_addr(default['proxy']),
        spool_address1=to_addr(default['session_pool_1']),
        spool_address2=to_addr(default['session_pool_2']),
        TESTS_DIR=TESTS_DIR,
    )
    with run_swindon(_proc, swindon_bin, config, rust_log, default['main'],
                     **options) as inst_info:
        yield inst_info


@pytest.fixture(scope='session')
def swindon_two(_proc, request, debug_routing,
                swindon_bin, swindon_ports, TESTS_DIR):
    """Swindon instance with enabled chat replication."""
    peer1 = swindon_ports['peer1']
    peer2 = swindon_ports['peer2']

    def to_addr(port):
        return '127.0.0.1:{}'.format(port)

    config = request.config.getoption('--swindon-replication-config')
    rust_log = request.config.getoption('--rust-log')
    options1 = dict(
        swindon_port=peer1['main'],
        proxy_port=peer1['proxy'],
        session_pool1_port=peer1['session_pool_1'],
        session_pool2_port=peer1['session_pool_2'],

        listen_address=to_addr(peer1['main']),
        debug_routing=str(debug_routing).lower(),
        proxy_address=to_addr(peer1['proxy']),
        spool_address1=to_addr(peer1['session_pool_1']),
        spool_address2=to_addr(peer1['session_pool_2']),
        replication_listen_address=to_addr(peer1['replication']),
        replication_peer_address=to_addr(peer2['replication']),
        TESTS_DIR=TESTS_DIR,
    )
    options2 = dict(
        swindon_port=peer2['main'],
        proxy_port=peer1['proxy'],  # NOTE: using peer1 proxy addr
        session_pool1_port=peer2['session_pool_1'],
        session_pool2_port=peer2['session_pool_2'],

        listen_address=to_addr(peer2['main']),
        debug_routing=str(debug_routing).lower(),
        proxy_address=to_addr(peer1['proxy']),  # NOTE: using peer1 proxy addr
        spool_address1=to_addr(peer2['session_pool_1']),
        spool_address2=to_addr(peer2['session_pool_2']),
        replication_listen_address=to_addr(peer2['replication']),
        replication_peer_address=to_addr(peer1['replication']),
        TESTS_DIR=TESTS_DIR,
    )
    with run_swindon(_proc, swindon_bin, config, rust_log,
                     peer1['main'], peer1['replication'], **options1) as peer1:
        with run_swindon(_proc, swindon_bin, config, rust_log,
                         peer2['main'], peer2['replication'],
                         **options2) as peer2:
            yield peer1, peer2


@contextmanager
def run_swindon(_proc, bin, config, log, *wait_ports, **options):
    no_permission_file = pathlib.Path(tempfile.gettempdir())
    no_permission_file /= 'no-permission.txt'
    if not no_permission_file.exists():
        no_permission_file.touch(0o000)

    fd, fname = tempfile.mkstemp()

    conf_template = pathlib.Path(config)
    with (ROOT / conf_template).open('rt') as f:
        tpl = string.Template(f.read())

    config = tpl.substitute(**options)
    assert _check_config(config, returncode=0, __swindon_bin=bin) == ''

    os.write(fd, config.encode('utf-8'))

    proc = _proc(bin,
                 '--verbose',
                 '--config',
                 fname,
                 env={'RUST_LOG': log,
                      'RUST_BACKTRACE': os.environ.get('RUST_BACKTRACE', '0')},
                 )
    wait_ports = set(wait_ports)
    while wait_ports:
        port = wait_ports.pop()
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            try:
                s.connect(('127.0.0.1', port))
            except ConnectionRefusedError:
                wait_ports.add(port)
                continue
            break

    url = yarl.URL('http://localhost:{swindon_port}'.format(**options))
    proxy = yarl.URL('http://localhost:{proxy_port}'.format(**options))
    api1 = yarl.URL('http://localhost:{session_pool1_port}'.format(**options))
    api2 = yarl.URL('http://localhost:{session_pool2_port}'.format(**options))
    try:
        yield SwindonInfo(proc, url, proxy, api1, api2)
    finally:
        os.close(fd)
        os.remove(fname)


@pytest.fixture(params=SWINDON_BIN)
def check_config(request):
    return partial(_check_config, __swindon_bin=request.param)


def _check_config(cfg='', returncode=1, *, __swindon_bin):
    cfg = textwrap.dedent(cfg)
    with tempfile.NamedTemporaryFile('wt') as f:
        f.write(cfg)
        f.flush()

        res = subprocess.run([
            __swindon_bin,
            '--check-config',
            '--config',
            f.name,
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            # encoding='utf-8',
            timeout=15,
            )
        assert not res.stdout, res
        assert res.returncode == returncode, res
        return res.stderr.decode('utf-8').replace(f.name, 'TEMP_FILE_NAME')


@pytest.fixture
def proxy_server(swindon, loop):
    ctx = ContextServer(loop)
    loop.run_until_complete(ctx.start_server(swindon.proxy.port))
    try:
        yield ctx
    finally:
        loop.run_until_complete(ctx.stop_server())


class ContextServer:

    def __init__(self, loop=None):
        self.loop = loop
        self.queue = asyncio.Queue(loop=loop)

        async def handler(request):
            fut = self.loop.create_future()
            await self.queue.put((request, fut))
            return await fut

        self.server = web.Server(handler, loop=loop)
        self._srv = None

    async def start_server(self, port):
        assert self._srv is None
        self._srv = await self.loop.create_server(
            self.server, '127.0.0.1', port,
            reuse_address=True,
            reuse_port=hasattr(socket, 'SO_REUSEPORT'))

    async def stop_server(self, *, shutdown=True):
        assert self._srv is not None
        srv, self._srv = self._srv, None
        if shutdown:
            await self.server.shutdown(1)
        srv.close()
        await srv.wait_closed()

    def send(self, method, url, **kwargs):
        assert self._srv

        async def _send_request():
            async with aiohttp.ClientSession(loop=self.loop) as sess:
                async with sess.request(method, url, **kwargs) as resp:
                    await resp.read()
                    return resp

        tsk = asyncio.ensure_future(_send_request(), loop=self.loop)
        return _RequestContext(self.queue, tsk, loop=self.loop)

    def swindon_chat(self, url, **kwargs):
        # TODO: return Websocket worker
        return _WSContext(self.queue, url, kwargs, loop=self.loop)


class _RequestContext:
    def __init__(self, queue, tsk, loop=None):
        self.queue = queue
        self.tsk = tsk
        self.loop = loop

    async def __aenter__(self):
        get = asyncio.ensure_future(self.queue.get(), loop=self.loop)
        await asyncio.wait([get, self.tsk],
                           return_when=asyncio.FIRST_COMPLETED,
                           loop=self.loop)
        # must receive request first
        # otherwise something is wrong and request completed first
        if get.done():
            self._req, self._fut = await get
        else:
            get.cancel()
            self._req = self._fut = None

        return Inflight(self._req, self._fut, self.tsk)

    async def __aexit__(self, exc_type, exc, tb):
        if self._fut and not self._fut.done():
            self._fut.cancel()
        if not self.tsk.done():
            self.tsk.cancel()


class _WSContext:
    def __init__(self, queue, url, kwargs, loop):
        self.queue = queue
        self.loop = loop
        self.url = url
        self.kwargs = kwargs
        self.sess = aiohttp.ClientSession(loop=loop)
        self.ws = None

    async def __aenter__(self):

        fut = asyncio.ensure_future(
            self.sess.ws_connect(self.url, **self.kwargs), loop=self.loop)

        def set_ws(f):
            try:
                self.ws = f.result()
            except Exception as err:
                self.queue.put_nowait((err, None))
        fut.add_done_callback(set_ws)

        return WSInflight(self.queue, fut)

    async def __aexit__(self, exc_type, exc, tb):
        # XXX: this hangs for a while...
        if self.ws:
            await self.ws.close()
        await self.sess.close()


_Inflight = namedtuple('Inflight', 'req srv_resp client_resp')

_WSInflight = namedtuple('WSInflight', 'queue websocket')


class Inflight(_Inflight):
    @property
    def has_client_response(self):
        return self.client_resp.done()

    async def send_resp(self, resp):
        if isinstance(resp, Exception):
            self.srv_resp.set_exception(resp)
        else:
            self.srv_resp.set_result(resp)
        return await self.client_resp


class WSInflight(_WSInflight):

    async def request(self):
        return await self.queue.get()


# helpers


@pytest.fixture(scope='session')
def _proc(request):
    # Process runner
    processes = []
    PWD = pathlib.Path(__file__).parent.parent

    def run(*cmdline, **kwargs):
        kcov_cmd = []
        cmdline = list(map(str, cmdline))
        if request.config.getoption('--kcov'):
            exe = pathlib.Path(cmdline[0])
            h = hashlib.md5()
            for s in cmdline:
                h.update(s.encode('utf-8'))
            h = h.digest().hex()[:8]
            target = PWD / 'target/cov' / '{}-{}'.format(exe.name, h)
            if not target.exists():
                target.mkdir(parents=True)
            kcov_cmd = [
                str(request.config.getoption('--kcov')),
                '--include-path',
                str(PWD),
                '--verify',
                str(target),
            ]
        proc = subprocess.Popen(kcov_cmd + cmdline, **kwargs)
        processes.append(proc)
        return proc

    try:
        yield run
    finally:
        while processes:
            proc = processes.pop(0)
            proc.terminate()
            proc.wait()
