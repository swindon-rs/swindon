import aiohttp


async def test_index(swindon, http_request, debug_routing):
    # XXX: on resp.read() connection gets closed
    resp, data = await http_request(swindon.url / 'static')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'application/octet-stream'
    assert resp.headers['Content-Length'] == '4096'
    assert data != b''


async def test_ok(swindon, http_request, debug_routing):
    url = swindon.url / 'static' / 'static_file.txt'
    resp, data = await http_request(url)
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert resp.headers['Content-Length'] == '17'
    assert data == b'Static file test\n'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"/work/tests/assets/static_file.txt"'
    else:
        assert 'X-Swindon-File-Path' not in resp.headers


async def test_permission(swindon, http_request, debug_routing):
    msg = (b'<!DOCTYPE html><html><head>'
           b'<title>404 Not Found</title></head>'
           b'<body><h1>404 Not Found</h1><hr>'
           b'<p>Yours faithfully,<br>swindon/func-tests</p></body></html>')
    url = swindon.url / 'static' / 'no-permission'
    resp, data = await http_request(url)
    assert resp.status == 404
    assert data == msg
    assert resp.headers['Content-Type'] != 'text/no/permission'
    assert resp.headers['Content-Length'] == str(len(msg))


async def test_extra_headers(swindon, http_request, debug_routing):
    url = swindon.url / 'static-w-headers' / 'static_file.html'
    resp, data = await http_request(url)
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/html'
    assert resp.headers['Content-Length'] == '17'
    assert resp.headers['X-Some-Header'] == 'some value'
    assert data == b'Static file test\n'


async def test_headers_override(
        swindon, request_method, http_version, debug_routing):
    url = swindon.url / 'static-w-ctype' / 'static_file.txt'
    meth = request_method
    async with aiohttp.ClientSession(version=http_version) as s:
        async with s.request(meth, url) as resp:
            assert resp.status == 200
            assert resp.version == http_version
            assert resp.headers['Content-Length'] == '17'
            data = await resp.read()
            assert data == b'Static file test\n'
            ctype = [val for key, val in resp.raw_headers
                     if key == b'CONTENT-TYPE']
            assert len(ctype) == 1
            assert ctype[0] == b'text/plain'
