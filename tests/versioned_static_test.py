import pytest
import aiohttp


async def test_no_index(swindon, http_request, debug_routing):
    resp, data = await http_request(swindon.url / 'versioned')
    assert resp.status == 403
    assert resp.headers['Content-Type'] == 'text/html'


async def test_by_version1(swindon, http_request, debug_routing):
    resp, data = await http_request(swindon.url /
        'versioned/test.html?r=aabbbbbb')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/html'
    assert data == b'<!DOCTYPE html>\n<title>Hello</title>\n'


async def test_by_version2(swindon, http_request, debug_routing):
    resp, data = await http_request(swindon.url /
        'versioned/test.html?r=bbaaaaaa')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/html'
    assert data == b'<!DOCTYPE html>\n<title>Greetings</title>\n'


async def test_no_version_forbidden(swindon, http_request, debug_routing):
    resp, data = await http_request(swindon.url /
        'versioned/test.html')
    assert resp.status == 404
    assert resp.headers['Content-Type'] == 'text/html'
