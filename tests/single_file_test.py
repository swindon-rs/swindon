import os.path


def data_check(data, method, expected):
    if method == "HEAD":
        assert data == b''
    else:
        assert data == expected


async def test_ok(swindon, get_request, static_request_method,
        debug_routing, TESTS_DIR):
    resp, data = await get_request(swindon.url / 'static-file')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert resp.headers['Content-Length'] == '17'
    data_check(data, static_request_method, b'Static file test\n')
    if debug_routing:
        assert resp.headers['X-Swindon-Route'] == 'single_file'
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/static_file.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-Route' not in resp.headers
        assert 'X-Swindon-File-Path' not in resp.headers


async def test_query_args(swindon, get_request, static_request_method,
        debug_routing, TESTS_DIR):
    url = swindon.url / 'static-file'
    url = url.with_query(foo='bar')
    resp, data = await get_request(url)
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert resp.headers['Content-Length'] == '17'
    data_check(data, static_request_method, b'Static file test\n')
    if debug_routing:
        assert resp.headers['X-Swindon-Route'] == 'single_file'
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/static_file.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-Route' not in resp.headers
        assert 'X-Swindon-File-Path' not in resp.headers


async def test_request_method(swindon, get_request, static_request_method):
    resp, data = await get_request(swindon.url / 'static-file')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert resp.headers['Content-Length'] == '17'
    data_check(data, static_request_method, b'Static file test\n')


async def test_missing_file(swindon, get_request, static_request_method,
        debug_routing, TESTS_DIR):
    msg = open(os.path.dirname(__file__) + '/404.html', 'rb').read()
    resp, data = await get_request(swindon.url / 'missing-file')
    assert resp.status == 404
    data_check(data, static_request_method, msg)
    assert resp.headers['Content-Type'] != 'text/is/missing'
    assert resp.headers['Content-Length'] == str(len(msg))
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/missing_file.txt"'.format(TESTS_DIR)


async def test_permission(swindon, get_request, static_request_method):
    msg = open(os.path.dirname(__file__) + '/403.html', 'rb').read()
    resp, data = await get_request(swindon.url / 'no-permission')
    assert resp.status == 403
    data_check(data, static_request_method, msg)
    assert resp.headers['Content-Type'] == 'text/html'
    assert resp.headers['Content-Length'] == str(len(msg))


async def test_extra_headers(swindon, get_request, static_request_method):
    resp, data = await get_request(swindon.url / 'static-file-headers')
    assert resp.status == 200
    assert resp.headers.getall('X-Extra-Header') == ['extra value']
    assert 'X-Bad-Header' not in resp.headers


async def test_symlink(swindon, get_request, static_request_method,
        debug_routing, TESTS_DIR):
    resp, data = await get_request(swindon.url / 'symlink')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert resp.headers['Content-Length'] == '17'
    data_check(data, static_request_method, b'Static file test\n')
    if debug_routing:
        assert resp.headers['X-Swindon-Route'] == 'single_symlink'
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/link.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-Route' not in resp.headers
        assert 'X-Swindon-File-Path' not in resp.headers


async def test_non_file(swindon, get_request, static_request_method,
        debug_routing):
    msg = open(os.path.dirname(__file__) + '/403.html', 'rb').read()
    resp, data = await get_request(swindon.url / 'dev-null')
    assert resp.status == 403
    assert resp.headers['Content-Type'] == 'text/html'
    assert resp.headers['Content-Length'] == str(len(msg))
    data_check(data, static_request_method, msg)
    if debug_routing:
        assert resp.headers['X-Swindon-Route'] == 'dev_null'
        assert resp.headers['X-Swindon-File-Path'] == \
            '"/dev/null"'
    else:
        assert 'X-Swindon-Route' not in resp.headers
        assert 'X-Swindon-File-Path' not in resp.headers
