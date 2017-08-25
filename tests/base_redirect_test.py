import aiohttp


async def test_ok(swindon, proxy_request_method, http_version,
        debug_routing, loop):
    url = 'http://example.com:{}/empty.gif'.format(swindon.url.port)
    kw = {"allow_redirects": False}

    async with aiohttp.ClientSession(version=http_version, loop=loop) as s:
        async with s.request(proxy_request_method, url, **kw) as resp:
            assert resp.status == 301
            assert resp.headers.getall("Location") == [
                "http://localhost/empty.gif"
                ]
            if debug_routing:
                assert 'X-Swindon-Route' in resp.headers
            else:
                assert 'X-Swindon-Route' not in resp.headers
            assert await resp.read() == b''
