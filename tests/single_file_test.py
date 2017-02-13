import pytest


async def test_ok(swindon, http_request, debug_routing):
    resp, data = await http_request(swindon.url / 'static-file')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert resp.headers['Content-Length'] == '17'
    assert data == b'Static file test\n'
    if debug_routing:
        assert resp.headers['X-Swindon-Route'] == 'single_file'
        assert resp.headers['X-Swindon-File-Path'] == \
            '"/work/tests/assets/static_file.txt"'
    else:
        assert 'X-Swindon-Route' not in resp.headers
        assert 'X-Swindon-File-Path' not in resp.headers


async def test_request_method(swindon, http_request):
    resp, data = await http_request(swindon.url / 'static-file')
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'text/plain'
    assert resp.headers['Content-Length'] == '17'
    assert data == b'Static file test\n'


@pytest.mark.xfail(reason="Server name is static; expected one from config")
async def test_missing_file(swindon, http_request, debug_routing):
    msg = (b'<!DOCTYPE html><html><head>'
           b'<title>404 Not Found</title></head>'
           b'<body><h1>404 Not Found</h1><hr>'
           b'<p>Yours faithfully,<br>swindon/func-tests</p></body></html>')
    resp, data = await http_request(swindon.url / 'missing-file')
    assert resp.status == 404
    assert data == msg
    assert resp.headers['Content-Type'] != 'text/is/missing'
    assert resp.headers['Content-Length'] == str(len(msg))
    if debug_routing:
        assert resp.headers['X-Swindon-File-Path'] == \
            '"/work/tests/assets/missing_file.txt"'


@pytest.mark.xfail(reason="Server name is static; expected one from config")
async def test_permission(swindon, http_request):
    msg = (b'<!DOCTYPE html><html><head>'
           b'<title>404 Not Found</title></head>'
           b'<body><h1>404 Not Found</h1><hr>'
           b'<p>Yours faithfully,<br>swindon/func-tests</p></body></html>')
    resp, data = await http_request(swindon.url / 'no-permission')
    assert resp.status == 404
    assert data == msg
    assert resp.headers['Content-Type'] != 'text/no/permission'
    assert resp.headers['Content-Length'] == str(len(msg))
