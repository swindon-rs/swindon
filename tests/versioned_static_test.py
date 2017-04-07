import pytest
import aiohttp


ALL_EQUAL = ["versioned", "versioned-fallback"]


@pytest.mark.parametrize("path", ALL_EQUAL)
async def test_no_index(swindon, http_request, debug_routing, path):
    resp, data = await http_request(swindon.url / path)
    assert resp.status == 403
    assert resp.headers['Content-Type'] == 'text/html'
    if debug_routing:
        assert resp.headers['X-Swindon-Deny'] == "no-index"
    else:
        assert 'X-Swindon-Deny' not in resp.headers


@pytest.mark.parametrize("path", ALL_EQUAL)
async def test_by_version1(swindon, http_request, debug_routing, path):
    resp, data = await http_request(swindon.url / path /
        'test.html?r=aabbbbbb')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/html'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/hashed/aa/bbbbbb-test.html.html"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert data == b'<!DOCTYPE html>\n<title>Hello</title>\n'


@pytest.mark.parametrize("path", ALL_EQUAL)
async def test_by_version2(swindon, http_request, debug_routing, path):
    resp, data = await http_request(swindon.url / path /
        'test.html?r=bbaaaaaa')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/html'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/hashed/bb/aaaaaa-test.html"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert data == b'<!DOCTYPE html>\n<title>Greetings</title>\n'


async def test_version_404(swindon, http_request, debug_routing, path):
    resp, data = await http_request(swindon.url / path /
        'non-existent-file.html?r=aabbbbbb')
    assert resp.status == 404
    assert resp.headers['Content-Type'] == 'text/html'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/hashed/aa/bbbbbb/non-existent-file.html"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert 'X-Swindon-Deny' not in resp.headers


async def test_no_version(swindon, http_request, debug_routing, path):
    resp, data = await http_request(swindon.url / path /
        'test.html?r=aabbbbbb')
    assert resp.status == 404
    assert resp.headers['Content-Type'] == 'text/html'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/hashed/aa/bbbbbb/non-existent-file.html"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert 'X-Swindon-Deny' not in resp.headers


@pytest.mark.parametrize("path", ALL_EQUAL)
async def path_encoding(swindon, http_request, debug_routing, path):
    resp, data = await http_request(swindon.url / path /
        'a+b.txt?r=aabbbbbb')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/hashed/aa/bbbbbb-a+b.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert data == b'a+b at aabbbbbb\n'


async def path_encoding_fallback(swindon, http_request, debug_routing, path):
    resp, data = await http_request(swindon.url / path /
        'a+b.txt')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/a+b.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert data == b'a+b\n'


async def path_fallback_404(swindon, http_request, debug_routing, path):
    resp, data = await http_request(swindon.url / path /
        'non-existent-file.html')
    assert resp.status == 404
    assert resp.headers['Content-Type'] == 'text/html'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/non-existent-file.html"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert 'X-Swindon-Deny' not in resp.headers


async def path_other_params(swindon, http_request, debug_routing, path):
    resp, data = await http_request(swindon.url / path /
        'a+b.txt?some=param&another=param')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/a+b.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert data == b'a+b\n'

async def path_crappy_query(swindon, http_request, debug_routing, path):
    resp, data = await http_request(swindon.url / path /
        'a+b.txt?just_some_garbage')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/a+b.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert data == b'a+b\n'


async def test_no_version_forbidden(swindon, http_request, debug_routing):
    resp, data = await http_request(swindon.url /
        'versioned/test.html')
    assert resp.status == 404
    assert resp.headers['Content-Type'] == 'text/html'
    if debug_routing:
        assert resp.headers['X-Swindon-Deny'] == "no-version"
    else:
        assert 'X-Swindon-Deny' not in resp.headers

async def test_bad_version_forbidden(swindon, http_request, debug_routing):
    resp, data = await http_request(swindon.url /
        'versioned/test.html?r=xxx')
    assert resp.status == 404
    assert resp.headers['Content-Type'] == 'text/html'
    if debug_routing:
        assert resp.headers['X-Swindon-Deny'] == "bad-version"
    else:
        assert 'X-Swindon-Deny' not in resp.headers
