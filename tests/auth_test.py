import aiohttp

def assert_gif(resp, data, debug_routing):
    assert resp.status == 200
    assert resp.headers['Content-Type'] == 'image/gif'
    assert resp.headers['Content-Length'] == '26'
    assert resp.headers['Server'] == 'swindon/func-tests'
    if debug_routing:
        assert resp.headers['X-Swindon-Route'] == 'empty_gif'
    assert len(data) == 26

def assert_403(resp, data, debug_routing):
    assert resp.status == 403


async def test_local_ok(swindon, http_request, debug_routing):
    resp, data = await http_request(swindon.url / 'auth/local')
    assert_gif(resp, data, debug_routing)
    if debug_routing:
        assert resp.headers['X-Swindon-Authorizer'] == 'only-127-0-0-1'
        assert resp.headers['X-Swindon-Allow'] == 'source-ip 127.0.0.1/24'


async def test_forwarded_ok(swindon, http_request, debug_routing):
    resp, data = await http_request(swindon.url / 'auth/by-header',
        headers={"X-Real-Ip": "8.8.8.8"})
    if debug_routing:
        assert resp.headers['X-Swindon-Authorizer'] == 'by-header'
        assert resp.headers['X-Swindon-Allow'] == \
            'forwarded-from 127.0.0.1/24, source-ip 8.0.0.0/8'


async def test_forwarded_bad(swindon, http_request, debug_routing):
    resp, data = await http_request(swindon.url / 'auth/by-header',
        headers={"X-Real-Ip": "4.4.4.4"})
    assert_403(resp, data, debug_routing)
    if debug_routing:
        assert resp.headers['X-Swindon-Authorizer'] == 'by-header'
        assert resp.headers['X-Swindon-Deny'] == 'source-ip 4.4.4.4'
