import pytest
from unittest import mock
from async_timeout import timeout
from aiohttp import web
from aiohttp import WSMsgType


async def test_simple_userinfo(proxy_server, swindon):
    url = swindon.url / 'swindon-chat'
    async with proxy_server.swindon_chat(url, timeout=1) as inflight:
        req, fut = await inflight.req.get()
        assert req.path == '/tangle/authorize_connection'
        assert req.headers['Content-Type'] == 'application/json'
        assert 'Authorization' not in req.headers
        expected = [
            {'connection_id': '0'},
            [],
            {'http_cookie': None,
             'http_authorization': None,
             'url_querystring': '',
             }]
        assert await req.json() == expected

        fut.set_result(
            web.Response(text='{"user_id": "user:1", "username": "John"}'))
        ws = await inflight.client_resp
        msg = await ws.receive_json()
        assert msg == ['hello', {}, {'user_id': 'user:1', 'username': 'John'}]


async def test_ws_close_timeout(proxy_server, swindon):
    url = swindon.url / 'swindon-chat'
    with timeout(1):
        async with proxy_server.swindon_chat(url) as inflight:
            req, fut = await inflight.req.get()
            assert req.path == '/tangle/authorize_connection'
            fut.set_result(
                web.Response(text='{"user_id": "user:1"}'))
            ws = await inflight.client_resp
            msg = await ws.receive_json()
            assert msg == [
                'hello', {}, {'user_id': 'user:1'}]


@pytest.mark.parametrize('status_code', [
    400, 401, 404, 410, 500, 503])
async def test_error_codes(proxy_server, swindon, loop, status_code):
    url = swindon.url / 'swindon-chat'
    async with proxy_server.swindon_chat(url, timeout=1) as inflight:
        req, fut = await inflight.req.get()
        assert req.path == '/tangle/authorize_connection'
        fut.set_result(
            web.Response(status=status_code, body=b'Custom Error'))
        ws = await inflight.client_resp
        msg = await ws.receive()
        assert msg.type == WSMsgType.CLOSE
        assert msg.data == 4000 + status_code
        assert msg.extra == 'backend_error'
        assert ws.closed
        assert ws.close_code == 4000 + status_code


@pytest.mark.parametrize('status_code', [
    100, 101,
    201, 204,
    300, 301, 302, 304,
    402, 405,
    501, 502, 504,  # these codes are not exposed to end-user.
    ])
async def test_unexpected_responses(proxy_server, swindon, loop, status_code):
    url = swindon.url / 'swindon-chat'
    async with proxy_server.swindon_chat(url, timeout=1) as inflight:
        req, fut = await inflight.req.get()
        assert req.path == '/tangle/authorize_connection'
        fut.set_result(
            web.Response(status=status_code, body=b'no body'))
        ws = await inflight.client_resp
        msg = await ws.receive()
        assert msg.type == WSMsgType.CLOSE
        assert msg.data == 4500
        assert msg.extra == 'backend_error'
        assert ws.closed
        assert ws.close_code == 4500


async def test_auth_request__cookies(proxy_server, swindon):
    url = swindon.url / 'swindon-chat'
    h = {"Cookie": "valid=cookie; next=value"}
    call = proxy_server.swindon_chat
    async with call(url, headers=h, timeout=1) as inflight:
        req, fut = await inflight.req.get()
        assert req.path == '/tangle/authorize_connection'
        assert req.headers['Content-Type'] == 'application/json'
        assert 'Authorization' not in req.headers
        expected = [
            {'connection_id': mock.ANY},
            [],
            {'http_cookie': "valid=cookie; next=value",
             'http_authorization': None,
             'url_querystring': '',
             }]
        assert await req.json() == expected

        fut.set_result(
            web.Response(text='{"user_id": "user:1", "username": "John"}'))
        ws = await inflight.client_resp
        msg = await ws.receive_json()
        assert msg == ['hello', {}, {'user_id': 'user:1', 'username': 'John'}]


async def test_auth_request__querystring(proxy_server, swindon):
    url = (swindon.url / 'swindon-chat').with_query(
        'query=param1&query=param2')
    print(url)
    call = proxy_server.swindon_chat
    async with call(url, timeout=1) as inflight:
        req, fut = await inflight.req.get()
        assert req.path == '/tangle/authorize_connection'
        assert req.headers['Content-Type'] == 'application/json'
        assert 'Authorization' not in req.headers
        expected = [
            {'connection_id': mock.ANY},
            [],
            {'http_cookie': None,
             'http_authorization': None,
             'url_querystring': 'query=param1&query=param2',
             }]
        assert await req.json() == expected

        fut.set_result(
            web.Response(text='{"user_id": "user:1", "username": "John"}'))
        ws = await inflight.client_resp
        msg = await ws.receive_json()
        assert msg == ['hello', {}, {'user_id': 'user:1', 'username': 'John'}]


async def test_auth_request__authorization(proxy_server, swindon):
    url = swindon.url / 'swindon-chat'
    h = {"Authorization": "digest abcdef"}
    call = proxy_server.swindon_chat
    async with call(url, headers=h, timeout=1) as inflight:
        req, fut = await inflight.req.get()
        assert req.path == '/tangle/authorize_connection'
        assert req.headers['Content-Type'] == 'application/json'
        assert 'Authorization' not in req.headers
        expected = [
            {'connection_id': mock.ANY},
            [],
            {'http_cookie': None,
             'http_authorization': "digest abcdef",
             'url_querystring': '',
             }]
        assert await req.json() == expected

        fut.set_result(
            web.Response(text='{"user_id": "user:1", "username": "John"}'))
        ws = await inflight.client_resp
        msg = await ws.receive_json()
        assert msg == ['hello', {}, {'user_id': 'user:1', 'username': 'John'}]


async def test_auth_request__all(proxy_server, swindon):
    url = swindon.url / 'swindon-chat'
    url = url.with_query("foo=bar")
    h = {"Cookie": "valid=cookie", "Authorization": "digest abcdef"}
    call = proxy_server.swindon_chat
    async with call(url, headers=h, timeout=1) as inflight:
        req, fut = await inflight.req.get()
        assert req.path == '/tangle/authorize_connection'
        assert req.headers['Content-Type'] == 'application/json'
        assert 'Authorization' not in req.headers
        expected = [
            {'connection_id': mock.ANY},
            [],
            {'http_cookie': "valid=cookie",
             'http_authorization': "digest abcdef",
             'url_querystring': 'foo=bar',
             }]
        assert await req.json() == expected

        fut.set_result(
            web.Response(text='{"user_id": "user:1", "username": "John"}'))
        ws = await inflight.client_resp
        msg = await ws.receive_json()
        assert msg == ['hello', {}, {'user_id': 'user:1', 'username': 'John'}]
