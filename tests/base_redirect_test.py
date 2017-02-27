import aiohttp


async def test_ok(swindon, request_method, http_version, debug_routing):
    url = 'http://example.com:{}/empty.gif'.format(swindon.url.port)
    kw = {"allow_redirects": False}

    async with aiohttp.ClientSession(version=http_version) as s:
        async with s.request(request_method, url, **kw) as resp:
            assert resp.status == 302
            assert resp.headers.getall("Location") == [
                "http://localhost/empty.gif"
                ]
            if debug_routing:
                assert 'X-Swindon-Route' in resp.headers
            else:
                assert 'X-Swindon-Route' not in resp.headers
            assert await resp.read() == b''
