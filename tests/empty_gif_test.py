import pytest
import aiohttp


async def test_ok(swindon, debug_routing):
    async with aiohttp.ClientSession() as sess:
        async with sess.get(swindon.url / 'empty.gif') as resp:
            assert resp.status == 200
            assert resp.headers['Content-Type'] == 'image/gif'
            assert resp.headers['Content-Length'] == '26'
            assert resp.headers['Server'] == 'swindon/func-tests'
            if debug_routing:
                assert resp.headers['X-Swindon-Route'] == 'empty_gif'
            else:
                assert 'X-Swindon-Route' not in resp.headers
            data = await resp.content.read()
            assert len(data) == 26


async def test_request_methods(swindon, request_method, http_version):
    async with aiohttp.ClientSession(version=http_version) as s:
        meth = request_method
        async with s.request(meth, swindon.url / 'empty.gif') as resp:
            assert resp.status == 200
            assert resp.headers['Content-Type'] == 'image/gif'
            assert resp.headers['Content-Length'] == '26'
            assert resp.headers['Server'] == 'swindon/func-tests'
            data = await resp.content.read()
            assert len(data) == 26


async def test_request_HEAD(swindon):
    async with aiohttp.ClientSession() as s:
        async with s.head(swindon.url / 'empty.gif') as resp:
            assert resp.status == 200
            assert resp.headers['Content-Type'] == 'image/gif'
            assert resp.headers['Content-Length'] == '26'
            assert resp.headers['Server'] == 'swindon/func-tests'
            data = await resp.content.read()
            assert len(data) == 0


async def test_extra_headers(swindon, request_method, http_version):
    meth = request_method
    url = swindon.url / 'empty-w-headers.gif'
    async with aiohttp.ClientSession(version=http_version) as sess:
        async with sess.request(meth, url) as resp:
            assert resp.status == 200
            assert resp.headers['X-Some-Header'] == 'some value'


@pytest.mark.xfail(reason="!EmptyGif allow multiple Content-Type headers")
async def test_headers_override(swindon, request_method, http_version):
    meth = request_method
    url = swindon.url / 'empty-w-content-length.gif'
    async with aiohttp.ClientSession(version=http_version) as s:
        async with s.request(meth, url) as resp:
            assert resp.status == 200
            clen = [val for key, val in resp.raw_headers
                    if key == b'CONTENT-LENGTH']
            assert len(clen) == 1
            assert resp.headers['Content-Length'] == '26'

            ctype = [val for key, val in resp.raw_headers
                     if key == b'CONTENT-TYPE']
            assert len(ctype) == 1
            assert ctype[0] == b'image/other'
