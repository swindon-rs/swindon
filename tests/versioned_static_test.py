import pytest
import aiohttp


ALL_EQUAL = ["versioned", "versioned-fallback"]


def data_check(data, method, expected):
    if method == "HEAD":
        assert data == b''
    else:
        assert data == expected


async def test_no_index_ver(swindon, get_request, debug_routing):
    resp, data = await get_request(swindon.url / "versioned")
    assert resp.status == 404
    assert resp.headers['Content-Type'] == 'text/html'
    assert 'Cache-Control' not in resp.headers['Content-Type']


async def test_no_index_fb(swindon, get_request, debug_routing):
    resp, data = await get_request(swindon.url / "versioned-fallback")
    assert resp.status == 403
    assert resp.headers['Content-Type'] == 'text/html'
    assert 'Cache-Control' not in resp.headers['Content-Type']

async def test_no_index_dir(swindon, get_request, debug_routing):
    resp, data = await get_request(swindon.url / "versioned-fallback/index")
    assert resp.status == 403
    assert resp.headers['Content-Type'] == 'text/html'
    assert 'Cache-Control' not in resp.headers['Content-Type']


@pytest.mark.parametrize("path", ALL_EQUAL)
async def test_by_version1(swindon, get_request, static_request_method,
        debug_routing, path, TESTS_DIR):
    resp, data = await get_request((swindon.url / path /
        'test.html').with_query(r='aabbbbbb'))
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/html'
    assert resp.headers['Cache-Control'] == 'public, max-age=31536000, immutable'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/hashed/aa/bbbbbb-test.html"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    data_check(data, static_request_method,
        b'<!DOCTYPE html>\n<title>Hello</title>\n')


async def test_no_such_version(swindon, get_request, static_request_method,
        debug_routing, TESTS_DIR):
    resp, data = await get_request((swindon.url / 'versioned-fallback' /
        'test.html').with_query(r='aaeebbbb'))
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/html'
    assert resp.headers['Cache-Control'] == 'no-cache, no-store, must-revalidate'
    if debug_routing:
        # TODO(tailhook) fix debug path
        # this isn't very good for debugging, but let's cope with that
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/hashed/aa/eebbbb-test.html"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    data_check(data, static_request_method,
        b'<!DOCTYPE html>\n<title>file-from-assets</title>\n')


@pytest.mark.parametrize("path", ALL_EQUAL)
async def test_by_version2(swindon, get_request, static_request_method,
        debug_routing, path, TESTS_DIR):
    resp, data = await get_request((swindon.url / path /
        'test.html').with_query(r='bbaaaaaa'))
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/html'
    assert resp.headers['Cache-Control'] == 'public, max-age=31536000, immutable'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/hashed/bb/aaaaaa-test.html"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    data_check(data, static_request_method,
        b'<!DOCTYPE html>\n<title>Greetings</title>\n')


async def test_version_404(swindon, get_request, debug_routing, TESTS_DIR):
    resp, data = await get_request((swindon.url / "versioned" /
        'non-existent-file.html').with_query(r='aabbbbbb'))
    assert resp.status == 404
    assert resp.headers['Content-Type'] == 'text/html'
    assert 'Cache-Control' not in resp.headers['Content-Type']
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/hashed/aa/bbbbbb-non-existent-file.html"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert 'X-Swindon-Deny' not in resp.headers


@pytest.mark.parametrize("path", ALL_EQUAL)
async def test_encoding(swindon, get_request, static_request_method,
        debug_routing, path, TESTS_DIR):
    resp, data = await get_request((swindon.url / path / 'a+b.txt')
        .with_query(r='aabbbbbb'))
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert resp.headers['Cache-Control'] == 'public, max-age=31536000, immutable'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/hashed/aa/bbbbbb-a+b.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    data_check(data, static_request_method, b'a+b at aabbbbbb\n')


async def test_encoding_fallback(swindon, get_request, static_request_method,
        debug_routing, TESTS_DIR):
    resp, data = await get_request(swindon.url / 'versioned-fallback/a+b.txt')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/a+b.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    data_check(data, static_request_method, b'a+b\n')


async def test_fallback_404(swindon, get_request, debug_routing, TESTS_DIR):
    resp, data = await get_request(swindon.url /
        'versioned-fallback/non-existent-file.html')
    assert resp.status == 404
    assert resp.headers['Content-Type'] == 'text/html'
    assert 'Cache-Control' not in resp.headers['Content-Type']
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/non-existent-file.html"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    assert 'X-Swindon-Deny' not in resp.headers


async def test_no_version(swindon, get_request, static_request_method,
        debug_routing, TESTS_DIR):
    resp, data = await get_request((swindon.url /
        'versioned-fallback/a+b.txt')
        .with_query(some='param', another='param'))
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/a+b.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    data_check(data, static_request_method, b'a+b\n')


async def test_other_params(swindon, get_request, static_request_method,
        debug_routing, TESTS_DIR):
    resp, data = await get_request((swindon.url /
        'versioned-fallback/a+b.txt')
        .with_query(some='param', another='param'))
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/a+b.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    data_check(data, static_request_method, b'a+b\n')


async def test_crappy_query(swindon, get_request, static_request_method,
        debug_routing, TESTS_DIR):
    resp, data = await get_request(
        str(swindon.url / 'versioned-fallback' / 'a+b.txt')
        + '?just_some_garbage')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/a+b.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
    data_check(data, static_request_method, b'a+b\n')


async def test_no_version_forbidden(swindon, get_request, debug_routing):
    resp, data = await get_request(swindon.url /
        'versioned/test.html')
    assert resp.status == 404
    assert resp.headers['Content-Type'] == 'text/html'
    assert 'Cache-Control' not in resp.headers['Content-Type']
    # if debug_routing:
    #     assert resp.headers['X-Swindon-Deny'] == "no-version"
    # else:
    #     assert 'X-Swindon-Deny' not in resp.headers


async def test_bad_version_forbidden(swindon, get_request, debug_routing):
    resp, data = await get_request((swindon.url / 'versioned/test.html')
        .with_query(r='xxx'))
    assert resp.status == 404
    assert resp.headers['Content-Type'] == 'text/html'
    assert 'Cache-Control' not in resp.headers['Content-Type']
    # if debug_routing:
    #     assert resp.headers['X-Swindon-Deny'] == "bad-version"
    # else:
    #     assert 'X-Swindon-Deny' not in resp.headers
