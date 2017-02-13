import pytest
import aiohttp


async def test_ok(swindon, debug_routing):
    async with aiohttp.ClientSession() as s:
        async with s.get(swindon.url / 'static-file') as resp:
            assert resp.status == 200
            assert resp.headers['Content-Type'] == 'text/plain'
            assert resp.headers['Content-Length'] == '17'
            data = await resp.read()
            assert data == b'Static file test\n'
            if debug_routing:
                resp.headers['X-Swindon-Route'] == 'single_file'
            else:
                'X-Swindon-Route' not in resp.headers


async def test_request_method(swindon, request_method, http_version):
    async with aiohttp.ClientSession(version=http_version) as s:
        url = swindon.url / 'static-file'
        async with s.request(request_method, url) as resp:
            assert resp.status == 200
            assert resp.headers['Content-Type'] == 'text/plain'
            assert resp.headers['Content-Length'] == '17'
            data = await resp.read()
            assert data == b'Static file test\n'


@pytest.mark.xfail(reason="Server name is static; expected one from config")
async def test_missing_file(swindon, request_method, http_version):
    msg = (b'<!DOCTYPE html><html><head>'
           b'<title>404 Not Found</title></head>'
           b'<body><h1>404 Not Found</h1><hr>'
           b'<p>Yours faithfully,<br>swindon/func-tests</p></body></html>')
    async with aiohttp.ClientSession(version=http_version) as s:
        meth = request_method
        async with s.request(meth, swindon.url / 'missing-file') as resp:
            assert resp.status == 404
            data = await resp.read()
            assert data == msg
            assert resp.headers['Content-Type'] != 'text/is/missing'
            assert resp.headers['Content-Length'] == str(len(msg))


@pytest.mark.xfail(reason="Server name is static; expected one from config")
async def test_permission(swindon, request_method, http_version):
    msg = (b'<!DOCTYPE html><html><head>'
           b'<title>404 Not Found</title></head>'
           b'<body><h1>404 Not Found</h1><hr>'
           b'<p>Yours faithfully,<br>swindon/func-tests</p></body></html>')
    async with aiohttp.ClientSession(version=http_version) as s:
        meth = request_method
        async with s.request(meth, swindon.url / 'no-permission') as resp:
            assert resp.status == 404
            data = await resp.read()
            assert data == msg
            assert resp.headers['Content-Type'] != 'text/no/permission'
            assert resp.headers['Content-Length'] == str(len(msg))
