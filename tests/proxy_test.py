import pytest
from aiohttp import web, HttpVersion11


async def test_simple_request(proxy_server, swindon,
                              http_version, debug_routing):
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
        if debug_routing:
            assert client_resp.headers['X-Swindon-Route'] == 'proxy'
        else:
            assert 'X-Swindon-Route' not in client_resp.headers
        assert await client_resp.read() == b'OK'


@pytest.mark.xfail
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
