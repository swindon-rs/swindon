import pytest
import aiohttp


def assert_headers(headers, debug_routing):
    assert 'Content-Type' in headers
    assert 'Content-Length' in headers
    assert 'Date' in headers
    assert 'Server' in headers
    if debug_routing:
        assert 'X-Swindon-Route' in headers
        assert 'X-Swindon-File-Path' in headers
    else:
        assert 'X-Swindon-Route' not in headers
        assert 'X-Swindon-File-Path' not in headers

    assert len(headers.getall('Content-Type')) == 1
    assert len(headers.getall('Content-Length')) == 1
    assert len(headers.getall('Date')) == 1
    assert headers.getall('Server') == ['swindon/func-tests']


@pytest.mark.xfail(
    raises=aiohttp.ServerDisconnectedError,
    reason="Could not serve static index")
async def test_index(swindon, request_method, http_version, debug_routing):
    async with aiohttp.ClientSession(version=http_version) as s:
        meth = request_method
        url = swindon.url / 'static'
        async with s.request(meth, url) as resp:
            assert resp.status == 200
            assert resp.version == http_version
            assert resp.headers['Content-Type'] == 'application/octet-stream'
            assert resp.headers['Content-Length'] == '4096'
            data = await resp.read()    # XXX: connection gets closed
            assert data != b''
            assert_headers(resp.headers, debug_routing)


async def test_ok(swindon, request_method, http_version, debug_routing):
    url = swindon.url / 'static' / 'static_file.txt'
    meth = request_method
    async with aiohttp.ClientSession(version=http_version) as s:
        async with s.request(meth, url) as resp:
            assert resp.status == 200
            assert resp.version == http_version
            assert resp.headers['Content-Type'] == 'text/plain'
            assert resp.headers['Content-Length'] == '17'
            data = await resp.read()
            assert data == b'Static file test\n'
            assert_headers(resp.headers, debug_routing)


@pytest.mark.xfail(reason="Server name is static; expected one from config")
async def test_permission(swindon, request_method, http_version,
                          debug_routing):
    msg = (b'<!DOCTYPE html><html><head>'
           b'<title>404 Not Found</title></head>'
           b'<body><h1>404 Not Found</h1><hr>'
           b'<p>Yours faithfully,<br>swindon/func-tests</p></body></html>')
    url = swindon.url / 'static' / 'no-permission'
    async with aiohttp.ClientSession(version=http_version) as s:
        meth = request_method
        async with s.request(meth, url) as resp:
            assert resp.status == 404
            data = await resp.read()
            assert data == msg
            assert resp.headers['Content-Type'] != 'text/no/permission'
            assert resp.headers['Content-Length'] == str(len(msg))
            assert_headers(resp.headers, debug_routing)


async def test_extra_headers(
        swindon, request_method, http_version, debug_routing):
    url = swindon.url / 'static-w-headers' / 'static_file.html'
    meth = request_method
    async with aiohttp.ClientSession(version=http_version) as s:
        async with s.request(meth, url) as resp:
            assert resp.status == 200
            assert resp.version == http_version
            assert resp.headers['Content-Type'] == 'text/html'
            assert resp.headers['Content-Length'] == '17'
            assert resp.headers['X-Some-Header'] == 'some value'
            data = await resp.read()
            assert data == b'Static file test\n'
            assert_headers(resp.headers, debug_routing)


@pytest.mark.xfail(reason="!Static allow multiple Content-Type headers")
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
            assert_headers(resp.headers, debug_routing)
