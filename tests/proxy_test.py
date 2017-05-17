from aiohttp import web, HttpVersion11


async def test_simple_request(proxy_server, swindon,
                              http_version, debug_routing):
    url = swindon.url / 'proxy/hello'
    async with proxy_server.send('get', url, version=http_version) as inflight:
        assert not inflight.has_client_response, await inflight.client_resp

        assert inflight.req.method == 'GET'
        assert inflight.req.path == '/proxy/hello'
        assert inflight.req.version == HttpVersion11
        # original host (random port)
        assert inflight.req.headers['Host'].startswith('localhost:')

        srv_resp = web.Response(body=b'OK', content_type='text/test')
        client_resp = await inflight.send_resp(srv_resp)
        assert client_resp.status == 200
        assert client_resp.version == http_version
        assert client_resp.headers['Content-Type'] == 'text/test'
        if debug_routing:
            assert client_resp.headers['X-Swindon-Route'] == 'proxy'
        else:
            assert 'X-Swindon-Route' not in client_resp.headers
        assert await client_resp.read() == b'OK'

async def test_host_override(proxy_server, swindon,
                              http_version, debug_routing):
    url = swindon.url / 'proxy-w-host/hello'
    async with proxy_server.send('get', url, version=http_version) as inflight:
        assert not inflight.has_client_response, await inflight.client_resp

        assert inflight.req.method == 'GET'
        assert inflight.req.path == '/proxy-w-host/hello'
        assert inflight.req.version == HttpVersion11
        assert inflight.req.headers['Host'] == 'swindon.proxy.example.org'

        srv_resp = web.Response(body=b'OK', content_type='text/test')
        client_resp = await inflight.send_resp(srv_resp)
        assert client_resp.status == 200
        assert client_resp.version == http_version
        assert client_resp.headers['Content-Type'] == 'text/test'
        if debug_routing:
            assert client_resp.headers['X-Swindon-Route'] == 'proxy_w_host'
        else:
            assert 'X-Swindon-Route' not in client_resp.headers
        assert await client_resp.read() == b'OK'


async def test_method(proxy_server, swindon, request_method):
    url = swindon.url / 'proxy/hello'
    async with proxy_server.send(request_method, url) as inflight:
        assert not inflight.has_client_response, await inflight.client_resp

        assert inflight.req.method == request_method
        assert inflight.req.path == '/proxy/hello'
        assert inflight.req.version == HttpVersion11

        client_resp = await inflight.send_resp(web.Response(text='OK'))
        assert client_resp.status == 200


async def test_prefix(proxy_server, swindon):
    url = swindon.url / 'proxy-w-prefix/tail'
    async with proxy_server.send('GET', url) as inflight:
        assert not inflight.has_client_response, await inflight.client_resp

        assert inflight.req.method == 'GET'
        assert inflight.req.path == '/prefix/proxy-w-prefix/tail'
        assert inflight.req.version == HttpVersion11

        client_resp = await inflight.send_resp(web.Response(text='OK'))
        assert client_resp.status == 200


async def test_ip_header(proxy_server, swindon):
    url = swindon.url / 'proxy-w-ip-header'
    async with proxy_server.send("GET", url) as inflight:
        assert not inflight.has_client_response, await inflight.client_resp

        assert inflight.req.headers.getall('X-Some-Header') == ['127.0.0.1']

        client_resp = await inflight.send_resp(web.Response(text='OK'))
        assert client_resp.status == 200

    h = {"X-Some-Header": "1.2.3.4"}
    async with proxy_server.send("GET", url, headers=h) as inflight:
        assert not inflight.has_client_response, await inflight.client_resp

        assert set(inflight.req.headers.getall('X-Some-Header')) == {
            '1.2.3.4', '127.0.0.1'}

        client_resp = await inflight.send_resp(web.Response(text='OK'))
        assert client_resp.status == 200


async def test_request_id(proxy_server, swindon):
    url = swindon.url / 'proxy-w-request-id'
    async with proxy_server.send("GET", url) as inflight:
        assert not inflight.has_client_response, await inflight.client_resp

        assert len(inflight.req.headers.getall('X-Request-Id')[0]) == 32

        client_resp = await inflight.send_resp(web.Response(text='OK'))
        assert client_resp.status == 200


async def test_post_form(proxy_server, swindon):
    url = swindon.url / 'proxy/post'
    async with proxy_server.send('POST', url, data=b'Some body') as inflight:
        assert not inflight.has_client_response, await inflight.client_resp
        req = inflight.req
        assert await req.read() == b'Some body'

    data = {'field': 'value'}
    async with proxy_server.send('POST', url, data=data) as inflight:
        assert not inflight.has_client_response, await inflight.client_resp
        req = inflight.req
        assert dict(await req.post()) == {'field': 'value'}
