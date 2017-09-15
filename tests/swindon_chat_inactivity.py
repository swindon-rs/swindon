import re

from unittest import mock


def assert_auth(req):
    assert req.path == '/swindon/authorize_connection'
    assert req.headers["Host"] == "swindon.internal"
    assert req.headers['Content-Type'] == 'application/json'
    assert re.match('^swindon/(\d+\.){2}\d+$', req.headers['User-Agent'])
    assert 'Authorization' not in req.headers


def assert_headers(req):
    assert req.headers["Host"] == "swindon.internal"
    assert req.headers['Content-Type'] == 'application/json'
    assert re.match('^swindon/(\d+\.){2}\d+$', req.headers['User-Agent'])


async def test_inactivity(proxy_server, swindon, loop):
    chat_url = swindon.url / 'swindon-lattice-w-timeouts'
    async with proxy_server() as proxy:
        handler = proxy.swindon_lattice(chat_url, timeout=1)
        req = await handler.request()
        assert_auth(req)
        ws = await handler.json_response({
            "user_id": 'user:1', "username": "Jim"})

        hello = await ws.receive_json()
        assert hello == [
            'hello', {}, {'user_id': 'user:1', 'username': 'Jim'}]

        req = await handler.request(timeout=1.2)
        assert req.path == '/swindon/session_inactive'
        assert_headers(req)
        assert req.headers.getall('Authorization') == [
            'Tangle eyJ1c2VyX2lkIjoidXNlcjoxIn0='
            ]
        assert await req.json() == [{}, [], {}]
        await handler.response(status=204)

        await ws.send_json([
            'whatever', {'request_id': '1', 'active': 2}, [], {}])
        req = await handler.request(timeout=5)
        assert req.path == '/whatever'
        assert_headers(req)
        assert await req.json() == [
            {'request_id': '1', 'active': 2, 'connection_id': mock.ANY},
            [], {}]
        await handler.response(status=200)

        req = await handler.request(timeout=3.2)
        assert req.path == '/swindon/session_inactive'
        assert_headers(req)
        assert req.headers.getall('Authorization') == [
            'Tangle eyJ1c2VyX2lkIjoidXNlcjoxIn0='
            ]
        assert await req.json() == [{}, [], {}]
        await handler.response(status=200)
