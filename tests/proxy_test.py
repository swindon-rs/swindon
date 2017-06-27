import asyncio
import async_timeout

from aiohttp import HttpVersion11


async def test_simple_request(proxy_server, swindon,
                              http_version, debug_routing):
    url = swindon.url / 'proxy/hello'
    async with proxy_server(version=http_version) as proxy:
        handler = proxy.send('GET', url, timeout=5)
        req = await handler.request()
        assert req.path == '/proxy/hello'
        assert req.version == HttpVersion11
        # original host (random port)
        assert req.headers['Host'].startswith('localhost:')

        resp = await handler.response(b'OK', content_type='text/test')
        assert resp.status == 200
        assert resp.version == http_version
        assert resp.headers['Content-Type'] == 'text/test'
        if debug_routing:
            assert resp.headers['X-Swindon-Route'] == 'proxy'
        else:
            assert 'X-Swindon-Route' not in resp.headers
        assert await resp.read() == b'OK'


async def test_host_override(proxy_server, swindon,
                             http_version, debug_routing):
    url = swindon.url / 'proxy-w-host/hello'
    async with proxy_server(version=http_version) as proxy:
        handler = proxy.send('get', url, timeout=5)

        req = await handler.request()
        assert req.method == 'GET'
        assert req.path == '/proxy-w-host/hello'
        assert req.version == HttpVersion11
        assert req.headers['Host'] == 'swindon.proxy.example.org'

        resp = await handler.response(b'OK', content_type='text/test')
        assert resp.status == 200
        assert resp.version == http_version
        assert resp.headers['Content-Type'] == 'text/test'
        if debug_routing:
            assert resp.headers['X-Swindon-Route'] == 'proxy_w_host'
        else:
            assert 'X-Swindon-Route' not in resp.headers
        assert await resp.read() == b'OK'


async def test_method(proxy_server, swindon, request_method):
    url = swindon.url / 'proxy/hello'
    async with proxy_server() as proxy:
        handler = proxy.send(request_method, url, timeout=5)

        req = await handler.request()
        assert req.method == request_method
        assert req.path == '/proxy/hello'
        assert req.version == HttpVersion11

        resp = await handler.response('OK')
        assert resp.status == 200


async def test_prefix(proxy_server, swindon):
    url = swindon.url / 'proxy-w-prefix/tail'
    async with proxy_server() as proxy:
        handler = proxy.send('GET', url, timeout=5)

        req = await handler.request()
        assert req.method == 'GET'
        assert req.path == '/prefix/proxy-w-prefix/tail'
        assert req.version == HttpVersion11

        resp = await handler.response('OK')
        assert resp.status == 200


async def test_ip_header(proxy_server, swindon, request):
    is_wsgi = request.node.keywords.get('wsgi') is not None
    url = swindon.url / 'proxy-w-ip-header'
    async with proxy_server() as proxy:
        handler = proxy.send("GET", url, timeout=5)

        req = await handler.request()
        assert req.headers.getall('X-Some-Header') == ['127.0.0.1']

        resp = await handler.response('OK')
        assert resp.status == 200

        h = {"X-Some-Header": "1.2.3.4"}
        handler = proxy.send("GET", url, headers=h, timeout=5)

        req = await handler.request()
        if is_wsgi:
            # last header wins
            assert set(req.headers.getall('X-Some-Header')) == {'1.2.3.4'}
        else:
            assert set(req.headers.getall('X-Some-Header')) == {
                '1.2.3.4', '127.0.0.1'}

        resp = await handler.response('OK')
        assert resp.status == 200


async def test_request_id(proxy_server, swindon):
    url = swindon.url / 'proxy-w-request-id'
    async with proxy_server() as proxy:
        handler = proxy.send("GET", url, timeout=5)

        req = await handler.request()
        assert len(req.headers['X-Request-Id']) == 32

        resp = await handler.response('OK')
        assert resp.status == 200


async def test_post_form(proxy_server, swindon):
    url = swindon.url / 'proxy/post'
    async with proxy_server() as proxy:
        handler = proxy.send('POST', url, data=b'Some body', timeout=5)

        req = await handler.request()
        assert await req.read() == b'Some body'
        resp = await handler.response('OK')
        assert resp.status == 200

        data = {'field': 'value'}
        handler = proxy.send('POST', url, data=data, timeout=5)

        req = await handler.request()
        assert dict(await req.post()) == {'field': 'value'}
        resp = await handler.response('OK')
        assert resp.status == 200


async def test_request_timeout(proxy_server, swindon, loop):
    url = swindon.url / 'proxy-w-timeout'
    async with proxy_server() as proxy:
        handler, client_resp = proxy.send('GET', url, timeout=5)
        assert not client_resp.done(), await client_resp

        req = await handler.request(timeout=5)
        assert req.path == '/proxy-w-timeout'
        await asyncio.sleep(2, loop=loop)

        assert client_resp.done()
        with async_timeout.timeout(5, loop=loop):
            resp = await client_resp
        assert resp.status == 502
