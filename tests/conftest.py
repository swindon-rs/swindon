import hashlib
import json
import os
import pathlib
import pytest
import re
import shutil
import socket
import string
import subprocess
import tempfile
import textwrap
import time

import yarl
import aiohttp
import asyncio
import async_timeout

from itertools import count
from collections import namedtuple
from contextlib import contextmanager
from functools import partial
from aiohttp import web, HttpVersion
from concurrent.futures import TimeoutError
from multidict import CIMultiDictProxy, CIMultiDict

from werkzeug.wrappers import Request, Response
from werkzeug.serving import BaseWSGIServer


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


@pytest.fixture
def user_id(_c=count(1)):
    return 'u:{}'.format(next(_c))


@pytest.fixture(params=['GET', 'HEAD'])
def static_request_method(request):
    """Parametrized fixture changing request method
    (GET / ).
    """
    return request.param

@pytest.fixture(params=[
    'GET', 'PATCH', 'POST', 'PUT', 'UPDATED', 'DELETE', 'XXX'])
def proxy_request_method(request):
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
def http_request(proxy_request_method, http_version, debug_routing, loop):

    async def inner(url, **kwargs):
        async with aiohttp.ClientSession(version=http_version, loop=loop) as s:
            async with s.request(proxy_request_method, url, **kwargs) as resp:
                data = await resp.read()
                assert resp.version == http_version
                assert_headers(resp.headers, debug_routing)
                return resp, data
    return inner

# all head requests must equal to get requests
@pytest.fixture
def get_request(static_request_method, http_version, debug_routing, loop):

    async def inner(url, **kwargs):
        async with aiohttp.ClientSession(version=http_version, loop=loop) as s:
            async with s.request(static_request_method, url, **kwargs) as resp:
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


SwindonInfo = namedtuple('SwindonInfo',
    'proc url proxy api api2 api3 api4')


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


@pytest.fixture(scope='module')
def swindon_ports(unused_port, debug_routing, swindon_bin):
    class Dict(dict):
        def __missing__(self, key):
            self[key] = val = {
                'main': unused_port(),
                'proxy': unused_port(),
                'session_pool_1': unused_port(),
                'session_pool_2': unused_port(),
                'session_pool_3': unused_port(),
                'session_pool_4': unused_port(),
                'replication': unused_port(),
            }
            return val
    return Dict()


@pytest.fixture(scope='session', params=SWINDON_BIN)
def swindon_bin(request):
    return request.param


@pytest.fixture(scope='module')
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
        session_pool3_port=default['session_pool_3'],
        session_pool4_port=default['session_pool_4'],

        listen_address=to_addr(default['main']),
        debug_routing=str(debug_routing).lower(),
        proxy_address=to_addr(default['proxy']),
        spool_address1=to_addr(default['session_pool_1']),
        spool_address2=to_addr(default['session_pool_2']),
        spool_address3=to_addr(default['session_pool_3']),
        spool_address4=to_addr(default['session_pool_4']),
        TESTS_DIR=TESTS_DIR,
    )
    with run_swindon(_proc, swindon_bin, config, rust_log, default['main'],
                     **options) as inst_info:
        yield inst_info


@pytest.fixture(scope='module')
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
        session_pool3_port=peer1['session_pool_3'],
        session_pool4_port=peer1['session_pool_4'],

        listen_address=to_addr(peer1['main']),
        debug_routing=str(debug_routing).lower(),
        proxy_address=to_addr(peer1['proxy']),
        spool_address1=to_addr(peer1['session_pool_1']),
        spool_address2=to_addr(peer1['session_pool_2']),
        spool_address3=to_addr(peer1['session_pool_3']),
        spool_address4=to_addr(peer1['session_pool_4']),
        replication_listen_address=to_addr(peer1['replication']),
        replication_peer_address=to_addr(peer2['replication']),
        TESTS_DIR=TESTS_DIR,
    )
    options2 = dict(
        swindon_port=peer2['main'],
        proxy_port=peer1['proxy'],  # NOTE: using peer1 proxy addr
        session_pool1_port=peer2['session_pool_1'],
        session_pool2_port=peer2['session_pool_2'],
        session_pool3_port=peer2['session_pool_3'],
        session_pool4_port=peer2['session_pool_4'],

        listen_address=to_addr(peer2['main']),
        debug_routing=str(debug_routing).lower(),
        proxy_address=to_addr(peer1['proxy']),  # NOTE: using peer1 proxy addr
        spool_address1=to_addr(peer2['session_pool_1']),
        spool_address2=to_addr(peer2['session_pool_2']),
        spool_address3=to_addr(peer2['session_pool_3']),
        spool_address4=to_addr(peer2['session_pool_4']),
        replication_listen_address=to_addr(peer2['replication']),
        replication_peer_address=to_addr(peer1['replication']),
        TESTS_DIR=TESTS_DIR,
    )
    with run_swindon(_proc, swindon_bin, config, rust_log,
                     peer1['main'], peer1['replication'], **options1) as peer1:
        with run_swindon(_proc, swindon_bin, config, rust_log,
                         peer2['main'], peer2['replication'],
                         **options2) as peer2:
            time.sleep(1.5)
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
    assert _check_config(config, returncode=0, __swindon_bin=bin) in ('',
        # TODO(tailhook) maybe elimitate this warning?
        #                or make explicit tests for it?
        '[swindon::config::read] WARN: '
        'handler"proxy_w_request_id": request_id_header is deprecated '
        'in !Proxy setting, '
        'it must be specified in http destination\n')

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
    api3 = yarl.URL('http://localhost:{session_pool3_port}'.format(**options))
    api4 = yarl.URL('http://localhost:{session_pool4_port}'.format(**options))
    try:
        yield SwindonInfo(proc, url, proxy,
            api1, api2, api3, api4)
    finally:
        os.close(fd)
        os.remove(fname)


@pytest.fixture(params=SWINDON_BIN)
def check_config(request):
    return partial(_check_config, __swindon_bin=request.param)


@pytest.fixture(params=SWINDON_BIN)
def check_fingerprint(request):
    return partial(_check_fingerprint, __swindon_bin=request.param)


@contextmanager
def _write_configs(cfg, files={}):
    cfg = textwrap.dedent(cfg)
    if files:
        dir = tempfile.mkdtemp()
        try:
            main = os.path.join(dir, 'main.yaml')
            with open(os.path.join(dir, 'main.yaml'), 'wt') as f:
                f.write(cfg)
            for (name, text) in files.items():
                with open(os.path.join(dir, name + '.yaml'), 'wt') as f:
                    f.write(text)
            yield main
        finally:
            shutil.rmtree(dir)
    else:
        with tempfile.NamedTemporaryFile('wt') as f:
            f.write(cfg)
            f.flush()
            yield f.name


def _check_config(cfg='', returncode=1, files={}, *, __swindon_bin):
    with _write_configs(cfg, files) as main:
        return _run_check(__swindon_bin, main, returncode)

def _check_fingerprint(cfg='', files={}, *, __swindon_bin):
    with _write_configs(cfg, files) as main:
        res = subprocess.run([
            __swindon_bin,
            '--check-config',
            '--verbose',
            '--config',
            main,
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            # encoding='utf-8',
            timeout=15,
            )
        assert res.returncode == 0, res
        return res.stdout


def _run_check(bin, file, returncode):
    res = subprocess.run([
        bin,
        '--check-config',
        '--config',
        file,
        ],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        # encoding='utf-8',
        timeout=15,
        )
    assert not res.stdout, res
    assert res.returncode == returncode, res
    # removes log timestamps
    return re.sub(r"(?m)^[\d-]{10}T[\d:]{8}Z ", "", res.stderr.decode('utf-8'))


# helpers


@pytest.fixture(scope='module')
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


class _BaseServer:

    def __init__(self, loop):
        self.loop = loop
        self.queue = asyncio.Queue(loop=loop)
        self.futures = []
        self.websockets = []
        self._session = None

    async def _handle(self, request):
        fut = self.loop.create_future()
        self.futures.append(fut)
        await self.queue.put(request)
        return await fut

    async def __aenter__(self):
        assert self._session
        await self._session.__aenter__()

    async def __aexit__(self, *error):
        while self.futures:
            self.futures.pop().cancel()
        while self.websockets:
            await (self.websockets.pop()).close()
        await self.stop_server()
        sess, self._session = self._session, None
        await sess.__aexit__(*error)

    async def start_server(self, **client_session_kwargs):
        self._session = aiohttp.ClientSession(
            loop=self.loop, **client_session_kwargs)

    async def stop_server(self):
        pass

    async def request(self, method, url, timeout=None, **kwargs):
        with async_timeout.timeout(timeout, loop=self.loop):
            async with self._session.request(method, url, **kwargs) as resp:
                await resp.read()
                return resp

    async def ws_connect(self, url, **kwargs):
        ws = await self._session.ws_connect(url, **kwargs)
        self.websockets.append(ws)
        return ws

    async def wait_request(self):
        return await self.queue.get()

    async def set_response(self, args, kwargs):
        assert self.futures
        fut = self.futures.pop(0)
        if not fut.done():
            fut.set_result((args, kwargs))

    def send(self, method, url, **kwargs):
        client_resp = asyncio.ensure_future(
            self.request(method, url, **kwargs),
            loop=self.loop)
        req_fut = asyncio.ensure_future(
            self.wait_request(),
            loop=self.loop)
        client_resp.add_done_callback(lambda x: req_fut.cancel())
        handler = _Handler(req_fut, self.set_response, loop=self.loop)
        return _HandlerTuple((handler, client_resp))

    def start_ws(self, url, **kwargs):
        ws_fut = asyncio.ensure_future(
            self.ws_connect(url,
                protocols=['v1.swindon-lattice+json'],
                **kwargs),
            loop=self.loop)
        req_fut = asyncio.ensure_future(
            self.wait_request(),
            loop=self.loop)
        ws_fut.add_done_callback(lambda x: req_fut.cancel())
        handler = _Handler(req_fut, self.set_response,
                           self.wait_request, self.loop)
        return _HandlerTuple((handler, ws_fut))

    def start_ws_old(self, url, **kwargs):
        ws_fut = asyncio.ensure_future(
            self.ws_connect(url, **kwargs),
            loop=self.loop)
        req_fut = asyncio.ensure_future(
            self.wait_request(),
            loop=self.loop)
        ws_fut.add_done_callback(lambda x: req_fut.cancel())
        handler = _Handler(req_fut, self.set_response,
                           self.wait_request, self.loop)
        return _HandlerTuple((handler, ws_fut))


class _Handler:

    def __init__(self, request_fut, response_setter,
                 request_waiter=None, loop=None):
        self.req = request_fut
        self.req_waiter = request_waiter
        self.response_setter = response_setter
        self.loop = loop

    async def request(self, *, timeout=5):
        with async_timeout.timeout(timeout, loop=self.loop):
            if self.req is None and self.req_waiter:
                self.req = self.req_waiter()
            req = await self.req
            self.req = None
            return req

    async def response(self, *args, **kwargs):
        await self.response_setter(args, kwargs)

    async def json_response(self, data):
        await self.response(json.dumps(data),
                            status=200,
                            content_type='application/json')


class _HandlerTuple(tuple):

    @property
    def handler(self):
        return self[0]

    @property
    def client_response(self):
        return self[1]

    async def request(self, *args, **kwargs):
        return await self[0].request(*args, **kwargs)

    async def response(self, *args, **kwargs):
        await self[0].response(*args, **kwargs)
        return await self[1]

    async def json_response(self, *args, **kwargs):
        await self[0].json_response(*args, **kwargs)
        return await self[1]


class AiohttpServer(_BaseServer):

    def __init__(self, loop, **kwargs):
        super().__init__(loop, **kwargs)
        self._server = None

    async def start_server(self, port, **kwargs):
        await super().start_server(**kwargs)
        assert self._server is None

        async def handler(request):
            args, kwargs = await self._handle(request)
            fields = ('text', 'status', 'body', 'reason',
                      'headers', 'content_type', 'charset')
            kw = dict(zip(fields, args))
            if isinstance(kw.get('text'), bytes):
                kw['body'] = kw.pop('text')
            return web.Response(**kw, **kwargs)

        self._factory = web.Server(handler, loop=self.loop)
        self._server = await self.loop.create_server(
            self._factory, '127.0.0.1', port,
            reuse_address=True,
            reuse_port=hasattr(socket, 'SO_REUSEPORT'))

    async def stop_server(self):
        await super().stop_server()
        assert self._server is not None
        server, self._server = self._server, None
        await self._factory.shutdown(1)
        server.close()
        await server.wait_closed()


class WsgiServer(_BaseServer):
    def __init__(self, loop, timeout=15, **kwargs):
        super().__init__(loop, **kwargs)
        self.loop = loop
        self.timeout = timeout

        @Request.application
        def app(request):
            fut = asyncio.run_coroutine_threadsafe(
                self._handle(_WSGIRequest(request)), self.loop)
            fields = ('response', 'status', 'headers',
                      'mimetype', 'content_type',
                      'direct_passthrough')
            try:
                args, kwargs = fut.result()
                kw = dict(zip(fields, args))
                if kwargs.get('text'):
                    kwargs['response'] = kwargs.pop('text')
                return Response(**kw, **kwargs)
            except TimeoutError:
                return Response(status=502)
            except Exception:
                return Response(status=500)
        self.wsgi_app = app
        self.server = None
        self._requests = []

    async def start_server(self, port, **kwargs):
        await super().start_server(**kwargs)
        self.server = BaseWSGIServer('localhost', port, self.wsgi_app)
        self.server.timeout = self.timeout
        return self

    async def stop_server(self):
        await super().stop_server()
        self.server.server_close()
        while self._requests:
            self._requests.pop().cancel()
        await asyncio.sleep(0, loop=self.loop)

    async def wait_request(self):
        fut = self.loop.run_in_executor(None, self.server.handle_request)
        self._requests.append(fut)
        return await super().wait_request()

    async def set_response(self, args, kwargs):
        await super().set_response(args, kwargs)
        fut = self._requests.pop(0)
        await fut


class _WSGIRequest:

    def __init__(self, request):
        self._request = request
        self._path = request.path
        self._method = request.method
        v = request.environ['SERVER_PROTOCOL']
        self._version = HttpVersion(*map(int, v.split('/')[1].split('.')))
        self._headers = CIMultiDictProxy(CIMultiDict(list(request.headers)))
        self._data = request.get_data()
        # XXX: werkzeug's form data returns dict of lists
        self._form = {k: v[0] if len(v) == 1 else v
                      for k, v in request.form.items()}

    @property
    def path(self):
        return self._path

    @property
    def method(self):
        return self._method

    @property
    def version(self):
        return self._version

    @property
    def headers(self):
        return self._headers

    async def read(self):
        return self._data

    async def post(self):
        return self._form

    async def json(self):
        return json.loads(self._data.decode('utf-8'))


@pytest.fixture
def wsgi_server(loop):
    return WsgiServer(loop)


@pytest.fixture
def async_server(loop):
    return AiohttpServer(loop)


@pytest.fixture(params=[
    pytest.param(AiohttpServer, marks=pytest.mark.async),
    pytest.param(WsgiServer, marks=pytest.mark.wsgi),
], ids=[
    'upstream[async]',
    'upstream[wsgi]',
], scope='module')
def proxy_server(request, swindon, loop):
    server = request.param(loop)

    class _ServerWrapper:
        def __init__(self, port=swindon.proxy.port, **client_session_kwargs):
            self.port = port
            self.client_session_kwargs = client_session_kwargs

        async def __aenter__(self):
            await server.start_server(
                port=self.port, **self.client_session_kwargs)
            await server.__aenter__()
            return self

        __aexit__ = server.__aexit__
        send = server.send
        swindon_chat = server.start_ws_old
        swindon_lattice = server.start_ws

    return _ServerWrapper


@pytest.fixture(scope='module')
def loop():
    loop = asyncio.new_event_loop()
    asyncio.set_event_loop(None)
    yield loop
    loop.stop()
    loop.run_forever()
