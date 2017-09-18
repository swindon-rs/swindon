"""
Tests old swindon protocol even despite that there is no ``SwindonChat``
setting
"""
import pytest
import aiohttp
import json
import asyncio
import re

from unittest import mock
from async_timeout import timeout
from aiohttp import WSMsgType


def assert_auth(req):
    assert req.path == '/tangle/authorize_connection'
    assert req.headers["Host"] == "swindon.internal"
    assert req.headers['Content-Type'] == 'application/json'
    assert re.match('^swindon/(\d+\.){2}\d+$', req.headers['User-Agent'])
    assert 'Authorization' not in req.headers


def assert_headers(req):
    assert req.headers["Host"] == "swindon.internal"
    assert req.headers['Content-Type'] == 'application/json'
    assert re.match('^swindon/(\d+\.){2}\d+$', req.headers['User-Agent'])


async def test_simple_userinfo(proxy_server, swindon, user_id):
    url = swindon.url / 'swindon-chat'
    async with proxy_server() as proxy:
        handler = proxy.swindon_chat(url, timeout=1)
        req = await handler.request()
        assert_auth(req)
        expected = [
            {'connection_id': mock.ANY},
            [],
            {'http_cookie': None,
             'http_authorization': None,
             'url_querystring': '',
             }]
        body = await req.json()
        assert body == expected
        assert isinstance(body[0]['connection_id'], str)
        assert len(body[0]['connection_id']) > 0

        ws = await handler.json_response({
            "user_id": user_id, "username": "John"})
        msg = await ws.receive_json()
        assert msg == ['hello', {}, {'user_id': user_id, 'username': 'John'}]


@pytest.mark.parametrize('resp,meta,data', [
    ({'status': 400, 'text': '[invalid json'},
     {'error_kind': 'http_error', 'http_error': 400},
     None),
    ({'status': 400, 'text': '{"fields_missing": ["args"]}'},
     {'error_kind': 'http_error', 'http_error': 400},
     {"fields_missing": ['args']}),
    ({'status': 200, 'text': '[not a valid json'},
     {'error_kind': 'data_error'},
     'expected ident at line 1 column 3'),
], ids=[
    'http_error; invalid json',
    'http_error; valid json ',
    'data_error; invalid json',
])
async def test_backend_errors(proxy_server, swindon, user_id,
                              resp, meta, data):
    url = swindon.url / 'swindon-chat'
    async with proxy_server() as proxy:
        handler = proxy.swindon_chat(url, timeout=1)
        req = await handler.request()
        assert_auth(req)
        expected = [
            {'connection_id': mock.ANY},
            [],
            {'http_cookie': None,
             'http_authorization': None,
             'url_querystring': '',
             }]
        assert await req.json() == expected

        ws = await handler.json_response(
            {"user_id": user_id, "username": "John"})
        msg = await ws.receive_json()
        assert msg == ['hello', {}, {'user_id': user_id, 'username': 'John'}]

        await ws.send_json(['test.bad_call', {'request_id': '1'}, [], {}])
        req = await handler.request()
        assert req.path == '/test/bad_call'
        assert_headers(req)
        assert await req.json() == [
            {'request_id': '1', 'connection_id': mock.ANY}, [], {},
        ]

        await handler.response(**resp)
        msg = await ws.receive_json()
        meta.update(request_id='1')
        assert msg == ["error", meta, data]


@pytest.mark.xfail(reason="shutdown is not implemented yet")
async def test_ws_close_timeout(proxy_server, swindon, user_id, loop):
    url = swindon.url / 'swindon-chat'
    with timeout(1, loop=loop):
        async with proxy_server() as proxy:
            handler = proxy.swindon_chat(url)
            req = await handler.request()
            assert_auth(req)
            ws = await handler.json_response({"user_id": user_id})
            msg = await ws.receive_json()
            assert msg == ['hello', {}, {'user_id': user_id}]
            await ws.close()


@pytest.mark.parametrize('status_code', [
    400, 401, 403, 404, 410, 500, 503])
async def test_error_codes(proxy_server, swindon, loop, status_code):
    url = swindon.url / 'swindon-chat'
    async with proxy_server() as proxy:
        handler = proxy.swindon_chat(url, timeout=1)
        req = await handler.request()
        assert_auth(req)
        ws = await handler.response(b'["Custom Error"]', status=status_code,
            content_type='application/json')
        msg = await ws.receive()
        assert msg.type == WSMsgType.TEXT
        assert json.loads(msg.data) == ["fatal_error",
            {"error_kind": "http_error", 'http_error': status_code},
            ["Custom Error"]]
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
@pytest.mark.parametrize('body', [b'no body', b'{"user_id": "user:1"}'])
async def test_unexpected_responses(
        proxy_server, swindon, loop, status_code, body):
    url = swindon.url / 'swindon-chat'
    async with proxy_server() as proxy:
        handler = proxy.swindon_chat(url, timeout=1)
        req = await handler.request()
        assert_auth(req)
        ws = await handler.response(body, status=status_code)
        msg = await ws.receive()
        assert msg.type == WSMsgType.TEXT
        assert json.loads(msg.data) == ["fatal_error",
            {"error_kind": "http_error", 'http_error': 500},
            None]
        msg = await ws.receive()
        assert msg.type == WSMsgType.CLOSE
        assert msg.data == 4500
        assert msg.extra == 'backend_error'
        assert ws.closed
        assert ws.close_code == 4500


@pytest.mark.parametrize('auth_resp', [
    'invalid json',
    '["user_id", "user:1"]',  # list instead of dict
    '"user:123"',
    '{}',
    '{"user_id": null}',
    '{"user_id": 123.1}',
    '{"user_id": "123",}',  # trailing comma
    ])
async def test_invalid_auth_response(proxy_server, swindon, auth_resp):
    url = swindon.url / 'swindon-chat'
    async with proxy_server() as proxy:
        handler = proxy.swindon_chat(url, timeout=1)
        req = await handler.request()
        assert_auth(req)

        ws = await handler.response(auth_resp, content_type='application/json')
        msg = await ws.receive()
        assert msg.type == WSMsgType.TEXT
        assert json.loads(msg.data) == ["fatal_error",
            {"error_kind": "http_error", 'http_error': 500},
            None]
        msg = await ws.receive()
        assert msg.type == WSMsgType.CLOSE
        assert msg.data == 4500
        assert msg.extra == 'backend_error'
        assert ws.closed
        assert ws.close_code == 4500


async def test_auth_request__cookies(proxy_server, swindon, user_id):
    url = swindon.url / 'swindon-chat'
    h = {"Cookie": "valid=cookie; next=value"}
    async with proxy_server() as proxy:
        handler = proxy.swindon_chat(url, headers=h, timeout=1)
        req = await handler.request()
        assert_auth(req)
        expected = [
            {'connection_id': mock.ANY},
            [],
            {'http_cookie': "valid=cookie; next=value",
             'http_authorization': None,
             'url_querystring': '',
             }]
        assert await req.json() == expected

        ws = await handler.json_response(
            {"user_id": user_id, "username": "John"})
        msg = await ws.receive_json()
        assert msg == ['hello', {}, {'user_id': user_id, 'username': 'John'}]


async def test_auth_request__querystring(proxy_server, swindon, user_id):
    url = (swindon.url / 'swindon-chat').with_query(
        'query=param1&query=param2')
    async with proxy_server() as proxy:
        handler = proxy.swindon_chat(url, timeout=1)
        req = await handler.request()
        assert_auth(req)
        expected = [
            {'connection_id': mock.ANY},
            [],
            {'http_cookie': None,
             'http_authorization': None,
             'url_querystring': 'query=param1&query=param2',
             }]
        assert await req.json() == expected

        ws = await handler.json_response(
            {"user_id": user_id, "username": "John"})
        msg = await ws.receive_json()
        assert msg == ['hello', {}, {'user_id': user_id, 'username': 'John'}]


async def test_auth_request__authorization(proxy_server, swindon, user_id):
    url = swindon.url / 'swindon-chat'
    h = {"Authorization": "digest abcdef"}
    async with proxy_server() as proxy:
        handler = proxy.swindon_chat(url, headers=h, timeout=1)
        req = await handler.request()
        assert_auth(req)
        expected = [
            {'connection_id': mock.ANY},
            [],
            {'http_cookie': None,
             'http_authorization': "digest abcdef",
             'url_querystring': '',
             }]
        assert await req.json() == expected

        ws = await handler.json_response(
            {"user_id": user_id, "username": "John"})
        msg = await ws.receive_json()
        assert msg == ['hello', {}, {'user_id': user_id, 'username': 'John'}]


async def test_auth_request__all(proxy_server, swindon, user_id):
    url = swindon.url / 'swindon-chat'
    url = url.with_query("foo=bar")
    h = {"Cookie": "valid=cookie", "Authorization": "digest abcdef"}
    async with proxy_server() as proxy:
        handler = proxy.swindon_chat(url, headers=h, timeout=1)
        req = await handler.request()
        assert_auth(req)
        expected = [
            {'connection_id': mock.ANY},
            [],
            {'http_cookie': "valid=cookie",
             'http_authorization': "digest abcdef",
             'url_querystring': 'foo=bar',
             }]
        assert await req.json() == expected

        ws = await handler.json_response(
            {"user_id": user_id, "username": "John"})
        msg = await ws.receive_json()
        assert msg == ['hello', {}, {'user_id': user_id, 'username': 'John'}]


async def test_echo_messages(proxy_server, swindon):
    url = swindon.url / 'swindon-chat'
    async with proxy_server() as proxy:
        handler = proxy.swindon_chat(url, timeout=1)
        req = await handler.request()
        assert_auth(req)
        ws = await handler.json_response(
            {"user_id": 'user:2', "username": "Jack"})
        hello = await ws.receive_json()
        assert hello == [
            'hello', {}, {'user_id': 'user:2', 'username': 'Jack'}]

        await ws.send_json(['chat.echo_message', {'request_id': '1'},
                            ['some message'], {}])
        req = await handler.request()
        assert req.path == '/chat/echo_message'
        assert_headers(req)
        assert await req.json() == [
            {'request_id': '1', 'connection_id': mock.ANY},
            ['some message'],
            {},
        ]
        auth_data = 'Tangle eyJ1c2VyX2lkIjoidXNlcjoyIn0='
        assert req.headers['Authorization'] == auth_data

        await handler.json_response({'echo': "some message"})

        echo = await ws.receive_json()
        assert echo == [
            'result', {'request_id': '1'},
            {'echo': "some message"},
            ]


async def test_prefix_routes(proxy_server, swindon, user_id):
    url = swindon.url / 'swindon-chat'
    async with proxy_server() as proxy:
        handler = proxy.swindon_chat(url, timeout=1)
        req = await handler.request()
        assert_auth(req)
        ws = await handler.json_response(
            {"user_id": user_id, "username": "Jack"})
        hello = await ws.receive_json()
        assert hello == [
            'hello', {}, {'user_id': user_id, 'username': 'Jack'}]

        await ws.send_json(['prefixed.echo_message', {'request_id': '1'},
                            ['some message'], {}])
        req = await handler.request()
        assert req.path == '/with-prefix/prefixed/echo_message'
        assert_headers(req)
        assert await req.json() == [
            {'request_id': '1', 'connection_id': mock.ANY},
            ['some message'],
            {},
        ]
        await handler.json_response({
            'echo': "some message",
            })

        echo = await ws.receive_json()
        assert echo == [
            'result', {'request_id': '1'},
            {'echo': "some message"},
            ]


async def test_topic_subscribe_publish(proxy_server, swindon, loop, user_id):
    url = swindon.url / 'swindon-chat'
    async with proxy_server() as proxy:
        handler = proxy.swindon_chat(url, timeout=1)
        req = await handler.request()
        assert_auth(req)
        meta, args, kwargs = await req.json()
        assert 'connection_id' in meta
        assert not args
        assert kwargs

        cid = meta['connection_id']

        async with aiohttp.ClientSession(loop=loop) as s:
            sub_url = swindon.api / 'v1/connection' / cid / 'subscriptions'
            sub_url = sub_url / 'some/topic'
            async with s.put(sub_url) as resp:
                assert resp.status == 204

            publish_url = swindon.api / 'v1/publish' / 'some/topic'
            data = b'{"Test": "message"}'
            async with s.post(publish_url, data=data) as resp:
                assert resp.status == 204

        ws = await handler.json_response({
            "user_id": user_id, "username": "Jack"})
        hello = await ws.receive_json()
        assert hello == [
            'hello', {}, {'user_id': user_id, 'username': 'Jack'}]
        msg = await ws.receive_json()
        assert msg == ['message', {'topic': 'some.topic'}, {'Test': 'message'}]

        async with aiohttp.ClientSession(loop=loop) as s:
            publish_url = swindon.api / 'v1/publish' / 'some/topic'
            data = b'"other message"'
            async with s.post(publish_url, data=data) as resp:
                assert resp.status == 204
        msg = await ws.receive_json()
        assert msg == ['message', {'topic': 'some.topic'}, 'other message']


async def test_lattice_subscribe_update(proxy_server, swindon, loop, user_id):
    url = swindon.url / 'swindon-chat'
    async with proxy_server() as proxy:
        handler = proxy.swindon_chat(url, timeout=1)
        req = await handler.request()
        assert_auth(req)
        meta, args, kwargs = await req.json()
        assert 'connection_id' in meta
        assert not args
        assert kwargs
        cid = meta['connection_id']

        async with aiohttp.ClientSession(loop=loop) as s:
            u = swindon.api / 'v1/connection' / cid / 'lattices'
            u = u / 'lattice/namespace'
            room_id = 'room:{}'.format(user_id)
            data = json.dumps({
                'shared': {
                    room_id: {'last_message_counter': 123},
                },
                'private': {
                    user_id: {
                        room_id: {'last_seen_counter': 120},
                    }
                },
            })
            async with s.put(u, data=data) as resp:
                assert resp.status == 204

        ws = await handler.json_response({
            "user_id": user_id, "username": "Jim"})
        hello = await ws.receive_json()
        assert hello == [
            'hello', {}, {'user_id': user_id, 'username': 'Jim'}]
        up = await ws.receive_json()
        assert up == [
            'lattice',
            {'namespace': 'lattice.namespace'},
            {room_id: {
                'last_message_counter': 123,
                'last_seen_counter': 120,
                }},
        ]

@pytest.mark.parametrize('pub_update, priv_update, result', [
    (101, 102, {'pub_counter': 101, 'priv_counter': 102}),
    (99, 107, {'priv_counter': 107}),
    (115, 55, {'pub_counter': 115}),
], ids=[
    'all-updated',
    'private-updated',
    'public-updated',
])
async def test_lattice_counter(proxy_server, swindon, loop, user_id,
        pub_update, priv_update, result):
    url = swindon.url / 'swindon-chat'
    async with proxy_server() as proxy:
        handler = proxy.swindon_chat(url, timeout=1)
        req = await handler.request()
        assert_auth(req)
        meta, args, kwargs = await req.json()
        assert 'connection_id' in meta
        assert not args
        assert kwargs
        cid = meta['connection_id']

        async with aiohttp.ClientSession(loop=loop) as s:
            u = swindon.api / 'v1/connection' / cid / 'lattices'
            u = u / 'lattice/namespace'
            room_id = 'room:{}'.format(user_id)
            data = json.dumps({
                'shared': {
                    room_id: {'pub_counter': 100},
                },
                'private': {
                    user_id: { room_id: { 'priv_counter': 100 }},
                },
            })
            async with s.put(u, data=data) as resp:
                assert resp.status == 204

        ws = await handler.json_response({
            "user_id": user_id, "username": "Jim"})
        hello = await ws.receive_json()
        assert hello == [
            'hello', {}, {'user_id': user_id, 'username': 'Jim'}]
        up = await ws.receive_json()
        assert up == [
            'lattice',
            {'namespace': 'lattice.namespace'},
            {room_id: {
                'pub_counter': 100.0,
                'priv_counter': 100.0,
                }},
        ]

        async with aiohttp.ClientSession(loop=loop) as s:
            u = swindon.api / 'v1/connection' / cid / 'lattices'
            u = u / 'lattice/namespace'
            room_id = 'room:{}'.format(user_id)
            data = json.dumps({
                'shared': {
                    room_id: {'pub_counter': pub_update},
                },
                'private': {
                    user_id: {
                        room_id: {'priv_counter': priv_update},
                    }
                },
            })
            async with s.put(u, data=data) as resp:
                assert resp.status == 204

        up = await ws.receive_json()
        assert up == [
            'lattice',
            {'namespace': 'lattice.namespace'},
            {room_id: result},
        ]


@pytest.mark.parametrize('pub_update, priv_update, result', [
    ([101, 'hello world'], [1503328587.024564, {'icon': 'busy'}],
     {'topic_register': [101, 'hello world'],
      'status_register': [1503328587.024564, {'icon': 'busy'}]}),
    ([ 95, 'hello world'], [1503328587.024564, {'icon': 'busy'}],
     {'status_register': [1503328587.024564, {'icon': 'busy'}]}),
    ([107, 'hello world'], [1503328500.024564, {'icon': 'busy'}],
     {'topic_register': [107, 'hello world']}),
], ids=[
    'all-updated',
    'private-updated',
    'public-updated',
])
async def test_lattice_register(proxy_server, swindon, loop, user_id,
        pub_update, priv_update, result):
    url = swindon.url / 'swindon-chat'
    async with proxy_server() as proxy:
        handler = proxy.swindon_chat(url, timeout=1)
        req = await handler.request()
        assert_auth(req)
        meta, args, kwargs = await req.json()
        assert 'connection_id' in meta
        assert not args
        assert kwargs
        cid = meta['connection_id']

        async with aiohttp.ClientSession(loop=loop) as s:
            u = swindon.api / 'v1/connection' / cid / 'lattices'
            u = u / 'lattice/namespace'
            room_id = 'room:{}'.format(user_id)
            data = json.dumps({
                'shared': {
                    room_id: {'topic_register': [100, "hello"]},
                },
                'private': {
                    user_id: {
                        room_id: {'status_register':
                            [1503328586.024564, {"icon": "free_for_chat"}]},
                    }
                },
            })
            async with s.put(u, data=data) as resp:
                assert resp.status == 204

        ws = await handler.json_response({
            "user_id": user_id, "username": "Jim"})
        hello = await ws.receive_json()
        assert hello == [
            'hello', {}, {'user_id': user_id, 'username': 'Jim'}]
        up = await ws.receive_json()
        assert up == [
            'lattice',
            {'namespace': 'lattice.namespace'},
            {room_id: {
                'topic_register': [100.0, "hello"],
                'status_register': [1503328586.024564,
                                    {"icon": "free_for_chat"}],
                }},
        ]

        async with aiohttp.ClientSession(loop=loop) as s:
            u = swindon.api / 'v1/connection' / cid / 'lattices'
            u = u / 'lattice/namespace'
            room_id = 'room:{}'.format(user_id)
            data = json.dumps({
                'shared': {
                    room_id: {'topic_register': pub_update},
                },
                'private': {
                    user_id: {
                        room_id: {'status_register': priv_update},
                    }
                },
            })
            async with s.put(u, data=data) as resp:
                assert resp.status == 204

        up = await ws.receive_json()
        assert up == [
            'lattice',
            {'namespace': 'lattice.namespace'},
            {room_id: result},
        ]


@pytest.mark.parametrize('path', [
    '', '/vvvv', '/v1/',
    '/v1/connection',
    '/v1/connection/',
    '/v1/connection/1/',
    '/v1/connection/1/invalid-method',
    '/v1/connection/1/subscriptions/',
    '/v1/connection/invalid-cid/subscriptions',
    '/v1/connection/invalid-cid/subscriptions/',
    '/v1/connection/1/subscriptions/some.topic',
    '/v1/connection/1/lattices',
    '/v1/connection/1/lattices/',
    '/v1/connection/1/lattices/invalid.namespace',
    '/v1/connection/invalid-cid/lattices/',
    '/v1/connection/invalid-cid/lattices/invalid.namespace',
    '/v1/publish',
    '/v1/publish/',
    '/v1/publish/invalid.topic',
    '/v1/lattice',
    '/v1/lattice/',
    '/v1/lattice/invalid.namespace',
])
async def test_invalid_api_path(proxy_server, swindon, loop, path):
    async with proxy_server(), aiohttp.ClientSession(loop=loop) as s:
        async with s.put(swindon.api.with_path(path)) as resp:
            assert resp.status == 404
            assert resp.content_length == 0


@pytest.mark.parametrize('method', [
    'GET', 'HEAD', 'POST', 'UPDATE', 'PATCH', 'XXX',
])
@pytest.mark.parametrize('path', [
    'v1/connection/1/subscriptions/topic',
    'v1/connection/1/lattices/namespace',
])
async def test_invalid_api_method_connection(
        proxy_server, swindon, loop, path, method):
    async with proxy_server(), aiohttp.ClientSession(loop=loop) as s:
        async with s.request(method, swindon.api / path) as resp:
            assert resp.status == 404


@pytest.mark.parametrize('method', [
    'GET', 'HEAD', 'PUT', 'UPDATE', 'PATCH', 'XXX',
])
@pytest.mark.parametrize('path', [
    'v1/publish/topic',
    'v1/lattice/namespace',
])
async def test_invalid_api_method_publish(
        proxy_server, swindon, loop, path, method):
    async with proxy_server(), aiohttp.ClientSession(loop=loop) as s:
        async with s.request(method, swindon.api / path, data='{}') as resp:
            assert resp.status == 404


@pytest.mark.parametrize('request_id', [
    1, "2", "abc_def_xyz", "abc-def-xyz",
])
async def test_request_id_routes__ok(
        proxy_server, swindon, request_id, user_id):
    url = swindon.url / 'swindon-chat'
    async with proxy_server() as proxy:
        handler = proxy.swindon_chat(url, timeout=1)
        req = await handler.request()
        assert_auth(req)
        ws = await handler.json_response({
            "user_id": user_id, "username": "Jack"})
        hello = await ws.receive_json()
        assert hello == [
            'hello', {}, {'user_id': user_id, 'username': 'Jack'}]

        await ws.send_json(
            ['rxid.echo_message', {'request_id': request_id}, [], {}])
        req = await handler.request()
        assert req.path == '/rxid/echo_message'
        assert_headers(req)
        assert "X-Request-Id" in req.headers
        msg = await req.json()
        assert msg == [
            {'request_id': request_id, 'connection_id': mock.ANY}, [], {},
        ]
        conn_id = msg[0]['connection_id']
        rxid = "{}-{}".format(conn_id, request_id)
        assert req.headers["X-Request-Id"] == rxid
        await handler.json_response({})

        echo = await ws.receive_json()
        assert echo == ['result', {'request_id': request_id}, {}, ]


@pytest.mark.parametrize('request_id', [
    1.1, -1, "invalid rxid", "!@#$", "a" * 37,
    {}, None, [],
], ids=str)
async def test_request_id_routes__bad(
        proxy_server, swindon, request_id, user_id):
    url = swindon.url / 'swindon-chat'
    async with proxy_server() as proxy:
        handler = proxy.swindon_chat(url, timeout=1)
        req = await handler.request()
        assert_auth(req)
        ws = await handler.json_response({
            "user_id": user_id, "username": "Jack"})
        hello = await ws.receive_json()
        assert hello == [
            'hello', {}, {'user_id': user_id, 'username': 'Jack'}]

        await ws.send_json(
            ['rxid.echo_message', {'request_id': request_id}, [], {}])
        msg = await ws.receive_json()
        assert msg == [
            'error',
            {'request_id': request_id, 'error_kind': 'validation_error'},
            'invalid request id']


async def test_client_auth_timeout(proxy_server, swindon, loop):
    url = swindon.url / 'swindon-chat-w-client-timeout'
    async with proxy_server() as proxy:
        handler, ws_fut = proxy.swindon_chat(url, timeout=1)
        req = await handler.request()
        assert_auth(req)
        await asyncio.sleep(2, loop=loop)
        assert ws_fut.done()
        # assert handler.resp.done()
        # assert handler.resp.cancelled()

        ws = await ws_fut
        msg = await ws.receive()
        assert msg.type == WSMsgType.TEXT
        assert json.loads(msg.data) == ["fatal_error",
            {"error_kind": "http_error", 'http_error': 500},
            None]
        msg = await ws.receive()
        assert msg.type == WSMsgType.CLOSE
        assert msg.data == 4500
        assert ws.closed
        assert ws.close_code == 4500


async def test_client_call_timeout(proxy_server, swindon, loop, user_id):
    url = swindon.url / 'swindon-chat-w-client-timeout'
    async with proxy_server() as proxy:
        handler = proxy.swindon_chat(url, timeout=1)
        req = await handler.request()
        assert_auth(req)
        ws = await handler.json_response({'user_id': user_id})

        msg = await ws.receive_json()
        assert msg == ['hello', {}, {'user_id': user_id}]

        await ws.send_json(['timeout', {'request_id': 1}, [], {}])
        req = await handler.request()
        assert req.path == '/timeout'
        assert_headers(req)
        await asyncio.sleep(2, loop=loop)
        # ws_fut = handler[1]
        # assert ws_fut.done()

        msg = await ws.receive_json()
        assert msg == [
            "error",
            {'request_id': 1, "error_kind": "http_error", 'http_error': 500},
            None]
        assert not ws.closed
