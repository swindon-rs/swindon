import aiohttp


async def test_ok(swindon, http_request, debug_routing):
    resp, data = await http_request(swindon.url / 'empty.gif')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'image/gif'
    assert resp.headers['Content-Length'] == '26'
    assert resp.headers['Server'] == 'swindon/func-tests'
    if debug_routing:
        assert resp.headers['X-Swindon-Route'] == 'empty_gif'
    assert len(data) == 26


async def test_request_methods(swindon, http_request):
    resp, data = await http_request(swindon.url / 'empty.gif')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'image/gif'
    assert resp.headers['Content-Length'] == '26'
    assert resp.headers['Server'] == 'swindon/func-tests'
    assert len(data) == 26


async def test_request_HEAD(swindon, loop):
    async with aiohttp.ClientSession(loop=loop) as s:
        async with s.head(swindon.url / 'empty.gif') as resp:
            assert resp.status == 200
            assert resp.headers['Content-Type'] == 'image/gif'
            assert resp.headers['Content-Length'] == '26'
            assert resp.headers['Server'] == 'swindon/func-tests'
            data = await resp.content.read()
            assert len(data) == 0


async def test_extra_headers(swindon, http_request):
    resp, data = await http_request(swindon.url / 'empty-w-headers.gif')
    assert resp.status == 200
    assert resp.headers['X-Some-Header'] == 'some value'


async def test_headers_override(swindon, http_request):
    url = swindon.url / 'empty-w-content-length.gif'
    resp, data = await http_request(url)
    assert resp.status == 200
    clen = [val for key, val in resp.raw_headers
            if key == b'CONTENT-LENGTH']
    assert len(clen) == 1
    assert resp.headers['Content-Length'] == '26'

    ctype = [val for key, val in resp.raw_headers
             if key == b'CONTENT-TYPE']
    assert len(ctype) == 1
    assert ctype[0] == b'image/other'
