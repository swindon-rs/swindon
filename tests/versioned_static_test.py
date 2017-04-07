import pytest
import aiohttp


ALL_EQUAL = ["versioned", "versioned-fallback"]


@pytest.mark.parametrize("path", ALL_EQUAL)
async def test_no_index(swindon, http_request, debug_routing, path):
    resp, data = await http_request(swindon.url / path)
    assert resp.status == 403
    assert resp.headers['Content-Type'] == 'text/html'


@pytest.mark.parametrize("path", ALL_EQUAL)
async def test_by_version1(swindon, http_request, debug_routing, path):
    resp, data = await http_request(swindon.url / path /
        'test.html?r=aabbbbbb')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/html'
    assert data == b'<!DOCTYPE html>\n<title>Hello</title>\n'


@pytest.mark.parametrize("path", ALL_EQUAL)
async def test_by_version2(swindon, http_request, debug_routing, path):
    resp, data = await http_request(swindon.url / path /
        'test.html?r=bbaaaaaa')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/html'
    assert data == b'<!DOCTYPE html>\n<title>Greetings</title>\n'


@pytest.mark.parametrize("path", ALL_EQUAL)
async def path_encoding(swindon, http_request, debug_routing, path):
    resp, data = await http_request(swindon.url / path /
        'a+b.html?r=aabbbbbb')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert data == b'a+b at aabbbbbb\n'


async def path_encoding_fallback(swindon, http_request, debug_routing, path):
    resp, data = await http_request(swindon.url / path /
        'a+b.html')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert data == b'a+b\n'

async def path_other_params(swindon, http_request, debug_routing, path):
    resp, data = await http_request(swindon.url / path /
        'a+b.html?some=param&another=param')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert data == b'a+b\n'

async def path_crappy_query(swindon, http_request, debug_routing, path):
    resp, data = await http_request(swindon.url / path /
        'a+b.html?just_some_garbage')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert data == b'a+b\n'


async def test_no_version_forbidden(swindon, http_request, debug_routing):
    resp, data = await http_request(swindon.url /
        'versioned/test.html')
    assert resp.status == 404
    assert resp.headers['Content-Type'] == 'text/html'
