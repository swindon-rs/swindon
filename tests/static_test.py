import os.path
import pytest
import aiohttp

def data_check(data, method, expected):
    if method == "HEAD":
        assert data == b''
    else:
        assert data == expected


async def test_no_index(swindon, get_request, debug_routing):
    # XXX: on resp.read() connection gets closed
    resp, data = await get_request(swindon.url / 'static')
    assert resp.status == 403
    assert resp.headers['Content-Type'] == 'text/html'

async def test_autoindex(swindon, get_request,
        static_request_method, debug_routing):
    # XXX: on resp.read() connection gets closed
    resp, data = await get_request(swindon.url / 'static-autoindex')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/html; charset=utf-8'
    if static_request_method == 'HEAD':
        assert data == b''
    else:
        assert b'static_file.txt' in data
        assert b'index/' in data
        assert b'localhost/' in data
        assert b'</a>' in data
        assert b'href=' in data


async def test_index(swindon, get_request, static_request_method,
        debug_routing):
    # XXX: on resp.read() connection gets closed
    resp, data = await get_request(swindon.url / 'static-w-index')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/html'
    data_check(data, static_request_method,
        b'<!DOCTYPE html>\n<title>Hello</title>\n')


async def test_disabled_index(swindon, get_request, debug_routing):
    # XXX: on resp.read() connection gets closed
    resp, data = await get_request(swindon.url / 'static-wo-index')
    assert resp.status == 403
    assert resp.headers['Content-Type'] == 'text/html'


async def test_ok(swindon, get_request, static_request_method,
        debug_routing, TESTS_DIR):
    url = swindon.url / 'static' / 'static_file.txt'
    resp, data = await get_request(url)
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert resp.headers['Content-Length'] == '17'
    data_check(data, static_request_method, b'Static file test\n')
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/static_file.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers


async def test_url_decoding(swindon, get_request, static_request_method,
        debug_routing, TESTS_DIR):

    # Stringified url, because YARL normalizes some percent-encoded things
    url = str(swindon.url) + '/static/a%2bb.txt'

    resp, data = await get_request(url)
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert resp.headers['Content-Length'] == '4'
    data_check(data, static_request_method, b'a+b\n')
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/a+b.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers


async def test_permission(swindon, get_request, static_request_method,
        debug_routing, TESTS_DIR):
    msg = open(os.path.dirname(__file__) + '/403.html', 'rb').read()
    url = swindon.url / 'static-no-permission/no-permission.txt'
    resp, data = await get_request(url)
    assert resp.status == 403
    data_check(data, static_request_method, msg)
    assert resp.headers['Content-Type'] != 'text/no/permission'
    assert resp.headers['Content-Length'] == str(len(msg))
    if debug_routing:
        assert resp.headers.getall('X-Swindon-File-Path', []) == [
            '"/tmp/no-permission.txt"'.format(TESTS_DIR)]
    else:
        assert 'X-Swindon-File-Path' not in resp.headers

async def test_extra_headers(swindon, get_request, static_request_method,
        debug_routing, TESTS_DIR):
    url = swindon.url / 'static-w-headers' / 'static_file.html'
    resp, data = await get_request(url)
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/html'
    assert resp.headers['Content-Length'] == '17'
    assert resp.headers['X-Some-Header'] == 'some value'
    data_check(data, static_request_method, b'Static file test\n')
    if debug_routing:
        assert resp.headers.getall('X-Swindon-File-Path', []) == [
            '"{}/assets/static_file.html"'.format(TESTS_DIR)]
    else:
        assert 'X-Swindon-File-Path' not in resp.headers


async def test_headers_override(static_request_method,
        swindon, http_version, debug_routing, loop, TESTS_DIR):
    url = swindon.url / 'static-w-ctype' / 'static_file.txt'
    async with aiohttp.ClientSession(version=http_version, loop=loop) as s:
        async with s.request(static_request_method, url) as resp:
            assert resp.status == 200
            assert resp.version == http_version
            assert resp.headers['Content-Length'] == '17'
            data = await resp.read()
            data_check(data, static_request_method,
                b'Static file test\n')
            ctype = [val for key, val in resp.raw_headers
                     if key == b'CONTENT-TYPE']
            assert len(ctype) == 1
            assert ctype[0] == b'something/other'
            if debug_routing:
                assert resp.headers.getall('X-Swindon-File-Path', []) == [
                    '"{}/assets/static_file.txt"'.format(TESTS_DIR)]
            else:
                assert 'X-Swindon-File-Path' not in resp.headers


async def test_hostname(swindon, get_request, static_request_method,
        debug_routing, TESTS_DIR):
    url = swindon.url / 'static-w-hostname' / 'test.txt'
    resp, data = await get_request(url)
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert resp.headers['Content-Length'] == '17'
    data_check(data, static_request_method, b'localhost+static\n')
    if debug_routing:
        assert resp.headers.getall('X-Swindon-File-Path', []) == [
            '"{}/assets/localhost/static-w-hostname/test.txt"'
            .format(TESTS_DIR)]
    else:
        assert 'X-Swindon-File-Path' not in resp.headers


@pytest.mark.parametrize('url_with', [
    lambda u: u.with_query(foo='bar'),
    lambda u: u.with_query(foo='bar').with_fragment('frag'),
    lambda u: u.with_fragment('frag'),
    ], ids='?foo=bar,?foo=bar#frag,#frag'.split(','))
async def test_url_with_query(static_request_method,
        swindon, get_request, debug_routing, url_with, TESTS_DIR):
    url = swindon.url / 'static' / 'static_file.txt'
    url = url_with(url)
    resp, data = await get_request(url)
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert resp.headers['Content-Length'] == '17'
    data_check(data, static_request_method, b'Static file test\n')
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/static_file.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-File-Path' not in resp.headers
