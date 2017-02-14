import asyncio
from aiohttp import web, HttpVersion11


async def poll(cb, loop):
    while True:
        await asyncio.sleep(1, loop=loop)
        print(cb())


async def test_simple_request(proxy_server, swindon, http_version):
    url = swindon.url / 'proxy/hello'
    async with proxy_server.send('get', url, version=http_version) as inflight:
        assert not inflight.has_client_response, await inflight.client_resp

        assert inflight.req.method == 'GET'
        assert inflight.req.path == '/proxy/hello'
        assert inflight.req.version == HttpVersion11

        srv_resp = web.Response(body=b'OK', content_type='text/test')
        client_resp = await inflight.send_resp(srv_resp)
        assert client_resp.status == 200
        assert client_resp.version == http_version
        assert client_resp.headers['Content-Type'] == 'text/test'
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
