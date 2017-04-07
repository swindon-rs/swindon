import pytest
import aiohttp


ALL_EQUAL = ["versioned", "versioned-fallback"]


@pytest.mark.parametrize("path", ALL_EQUAL)
async def test_no_index(swindon, http_request, debug_routing, path):
    resp, data = await http_request(swindon.url / path)
    assert resp.status == 404
    assert resp.headers['Content-Type'] == 'text/html'


@pytest.mark.parametrize("path", ALL_EQUAL)
async def test_by_version1(swindon, http_request, debug_routing,
        path, TESTS_DIR):
    resp, data = await http_request((swindon.url / path /
        'test.html').with_query(r='aabbbbbb'))
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/html'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/hashed/aa/bbbbbb-test.html"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert data == b'<!DOCTYPE html>\n<title>Hello</title>\n'


@pytest.mark.parametrize("path", ALL_EQUAL)
async def test_by_version2(swindon, http_request, debug_routing,
        path, TESTS_DIR):
    resp, data = await http_request((swindon.url / path /
        'test.html').with_query(r='bbaaaaaa'))
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/html'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/hashed/bb/aaaaaa-test.html"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert data == b'<!DOCTYPE html>\n<title>Greetings</title>\n'


async def test_version_404(swindon, http_request, debug_routing, TESTS_DIR):
    resp, data = await http_request((swindon.url / "versioned" /
        'non-existent-file.html').with_query(r='aabbbbbb'))
    assert resp.status == 404
    assert resp.headers['Content-Type'] == 'text/html'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/hashed/aa/bbbbbb-non-existent-file.html"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert 'X-Swindon-Deny' not in resp.headers


async def test_no_version(swindon, http_request, debug_routing, TESTS_DIR):
    resp, data = await http_request((swindon.url / "versioned" /
        'test.html').with_query(r='ccdddddd'))
    assert resp.status == 404
    assert resp.headers['Content-Type'] == 'text/html'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/hashed/cc/dddddd-test.html"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert 'X-Swindon-Deny' not in resp.headers


@pytest.mark.parametrize("path", ALL_EQUAL)
async def test_encoding(swindon, http_request, debug_routing, path, TESTS_DIR):
    resp, data = await http_request((swindon.url / path / 'a+b.txt')
        .with_query(r='aabbbbbb'))
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/hashed/aa/bbbbbb-a+b.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert data == b'a+b at aabbbbbb\n'


@pytest.mark.xfail(reason="no fallback implemented")
async def test_encoding_fallback(swindon, http_request, debug_routing,
        TESTS_DIR):
    resp, data = await http_request(swindon.url / 'versioned-fallback/a+b.txt')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/a+b.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert data == b'a+b\n'


@pytest.mark.xfail(reason="no fallback implemented")
async def test_fallback_404(swindon, http_request, debug_routing, TESTS_DIR):
    resp, data = await http_request(swindon.url /
        'versioned-fallback/non-existent-file.html')
    assert resp.status == 404
    assert resp.headers['Content-Type'] == 'text/html'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/non-existent-file.html"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert 'X-Swindon-Deny' not in resp.headers


@pytest.mark.xfail(reason="no fallback implemented")
async def test_other_params(swindon, http_request, debug_routing, TESTS_DIR):
    resp, data = await http_request((swindon.url /
        'versioned-fallback/a+b.txt')
        .with_query(some='param', another='param'))
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/a+b.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert data == b'a+b\n'


@pytest.mark.xfail(reason="no fallback implemented")
async def test_crappy_query(swindon, http_request, debug_routing, TESTS_DIR):
    resp, data = await http_request(
        str(swindon.url / 'versioned-fallback' / 'a+b.txt')
        + '?just_some_garbage')
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
    resp, data = await http_request((swindon.url / 'versioned/test.html')
        .with_query(r='xxx'))
    assert resp.status == 404
    assert resp.headers['Content-Type'] == 'text/html'
    if debug_routing:
        assert resp.headers['X-Swindon-Deny'] == "bad-version"
    else:
        assert 'X-Swindon-Deny' not in resp.headers
