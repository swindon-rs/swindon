

async def test_ok(swindon, http_request, debug_routing, TESTS_DIR):
    resp, data = await http_request(swindon.url / 'static-file')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert resp.headers['Content-Length'] == '17'
    assert data == b'Static file test\n'
    if debug_routing:
        assert resp.headers['X-Swindon-Route'] == 'single_file'
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/static_file.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-Route' not in resp.headers
        assert 'X-Swindon-File-Path' not in resp.headers


async def test_query_args(swindon, http_request, debug_routing, TESTS_DIR):
    url = swindon.url / 'static-file'
    url = url.with_query(foo='bar')
    resp, data = await http_request(url)
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert resp.headers['Content-Length'] == '17'
    assert data == b'Static file test\n'
    if debug_routing:
        assert resp.headers['X-Swindon-Route'] == 'single_file'
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/static_file.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-Route' not in resp.headers
        assert 'X-Swindon-File-Path' not in resp.headers


async def test_request_method(swindon, http_request):
    resp, data = await http_request(swindon.url / 'static-file')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert resp.headers['Content-Length'] == '17'
    assert data == b'Static file test\n'


async def test_missing_file(swindon, http_request, debug_routing, TESTS_DIR):
    msg = (b'<!DOCTYPE html><html><head>'
           b'<title>404 Not Found</title></head>'
           b'<body><h1>404 Not Found</h1><hr>'
           b'<p>Yours faithfully,<br>swindon web server</p></body></html>')
    resp, data = await http_request(swindon.url / 'missing-file')
    assert resp.status == 404
    assert data == msg
    assert resp.headers['Content-Type'] != 'text/is/missing'
    assert resp.headers['Content-Length'] == str(len(msg))
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/missing_file.txt"'.format(TESTS_DIR)


async def test_permission(swindon, http_request):
    # XXX: PermissionDenied error is not exposed and returned as 500
    msg = (b'<!DOCTYPE html><html><head>'
           b'<title>500 Internal Server Error</title></head>'
           b'<body><h1>500 Internal Server Error</h1><hr>'
           b'<p>Yours faithfully,<br>swindon web server</p></body></html>')
    resp, data = await http_request(swindon.url / 'no-permission')
    assert resp.status == 500
    assert data == msg
    assert resp.headers['Content-Type'] == 'text/html'
    assert resp.headers['Content-Length'] == str(len(msg))


async def test_extra_headers(swindon, http_request):
    resp, data = await http_request(swindon.url / 'static-file-headers')
    assert resp.status == 200
    assert resp.headers.getall('X-Extra-Header') == ['extra value']
    assert 'X-Bad-Header' not in resp.headers


async def test_symlink(swindon, http_request, debug_routing, TESTS_DIR):
    resp, data = await http_request(swindon.url / 'symlink')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert resp.headers['Content-Length'] == '17'
    assert data == b'Static file test\n'
    if debug_routing:
        assert resp.headers['X-Swindon-Route'] == 'single_symlink'
        assert resp.headers['X-Swindon-File-Path'] == \
            '"{}/assets/link.txt"'.format(TESTS_DIR)
    else:
        assert 'X-Swindon-Route' not in resp.headers
        assert 'X-Swindon-File-Path' not in resp.headers


async def test_non_file(swindon, http_request, debug_routing):
    msg = (b'<!DOCTYPE html><html><head>'
           b'<title>500 Internal Server Error</title></head>'
           b'<body><h1>500 Internal Server Error</h1><hr>'
           b'<p>Yours faithfully,<br>swindon web server</p></body></html>')
    resp, data = await http_request(swindon.url / 'dev-null')
    assert resp.status == 500
    assert resp.headers['Content-Type'] == 'text/html'
    assert resp.headers['Content-Length'] == str(len(msg))
    assert data == msg
    if debug_routing:
        assert resp.headers['X-Swindon-Route'] == 'dev_null'
        assert resp.headers['X-Swindon-File-Path'] == \
            '"/dev/null"'
    else:
        assert 'X-Swindon-Route' not in resp.headers
        assert 'X-Swindon-File-Path' not in resp.headers
