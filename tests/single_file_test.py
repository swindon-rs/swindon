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


@pytest.mark.parametrize('method', [
    'GET', 'PATCH', 'POST', 'PUT', 'UPDATE', 'DELETE', 'XXXX'])
async def test_request_method(swindon, method):
    async with aiohttp.ClientSession() as s:
        async with s.request(method, swindon.url / 'static-file') as resp:
            assert resp.status == 200
            assert resp.headers['Content-Type'] == 'text/plain'
            assert resp.headers['Content-Length'] == '17'
            data = await resp.read()
            assert data == b'Static file test\n'
