import aiohttp
import asyncio
import json
import pytest
import re

from aiohttp import ClientSession
from async_timeout import timeout
from unittest import mock


def assert_auth(req):
    assert req.path == '/swindon/authorize_connection'
    assert req.headers["Host"] == "swindon.internal"
    assert req.headers['Content-Type'] == 'application/json'
    assert re.match('^swindon/(\d+\.){2}\d+$', req.headers['User-Agent'])
    assert 'Authorization' not in req.headers


async def auth(handler, auth_data):
    req = await handler.request()
    assert req.path == '/swindon/authorize_connection'
    meta, *tail = await req.json()
    assert 'connection_id' in meta
    cid = meta['connection_id']
    ws = await handler.json_response(auth_data)
    hello = await ws.receive_json()
    assert hello == ['hello', {}, auth_data]
    return cid, ws


async def put(url, loop):
    async with ClientSession(loop=loop) as s:
        async with s.put(url,
            headers={"Content-Type": "application/json"}) as resp:
            assert resp.status == 204


async def delete(url, loop):
    async with ClientSession(loop=loop) as s:
        async with s.delete(url) as resp:
            assert resp.status == 204


async def post(url, data, loop):
    async with ClientSession(loop=loop) as s:
        async with s.post(url,
            headers={"Content-Type": "application/json"},
            data=data) as resp:
            assert resp.status == 204


async def test_simple_replication(swindon_two, proxy_server, loop):
    peerA, peerB = swindon_two
    # await asyncio.sleep(1.5, loop=loop)  # wait reconnect

    urlA = peerA.url / 'swindon-lattice'
    urlB = peerB.url / 'swindon-lattice'
    async with proxy_server(port=peerA.proxy.port) as proxy:
        handlerA = proxy.swindon_lattice(urlA, timeout=1)
        cid1, ws1 = await auth(
            handlerA, {'user_id': 'replicated-user:1', 'username': 'John'})

        url = peerA.api3 / 'v1/connection' / cid1 / 'subscriptions' / 'general'
        await put(url, loop)

        handlerB = proxy.swindon_lattice(urlB, timeout=1)
        cid2, ws2 = await auth(
            handlerB, {'user_id': 'replicated-user:1', 'username': 'John'})

        url = peerB.api3 / 'v1/connection' / cid2 / 'subscriptions' / 'general'
        await put(url, loop)

        # subscribe both to some topic and publish into one peer.

        data = b'{"test": "message"}'
        await post(peerB.api3 / 'v1/publish/general', data, loop)

        msg1 = await ws1.receive_json()
        msg2 = await ws2.receive_json()
        assert msg1 == [
            'message', {'topic': 'general'}, {'test': 'message'}]
        assert msg2 == [
            'message', {'topic': 'general'}, {'test': 'message'}]


@pytest.mark.parametrize("through", ["peerA", "peerB"])
async def test_non_local_connections(swindon_two, proxy_server, loop, through):
    peerA, peerB = swindon_two
    if through == 'peerA':
        subscribe_peer = peerA
    else:
        subscribe_peer = peerB

    # await asyncio.sleep(1.5, loop=loop)

    # The use case:
    # * client A connect to peer A; receive peer-local cid(0);
    # * client B connect to peer B; receive peer-lcoal cid(0);
    # * backend sends request to peer A to subscribe client A to some topic
    # * action replicated to peer B;
    # Expected result:
    # client B not subscribed to that topic
    urlA = peerA.url / 'swindon-lattice'
    urlB = peerB.url / 'swindon-lattice'
    async with proxy_server(port=peerA.proxy.port) as proxy:
        handlerA = proxy.swindon_lattice(urlA, timeout=1)
        cid1, ws1 = await auth(
            handlerA, {'user_id': 'replicated-user:1', 'username': 'John'})

        handlerB = proxy.swindon_lattice(urlB, timeout=1)
        cid2, ws2 = await auth(
            handlerB, {'user_id': 'replicated-user:2', 'username': 'John'})

        # Subscribe only first user;
        url = subscribe_peer.api3 / 'v1/connection'
        url = url / cid1 / 'subscriptions/general'
        await put(url, loop)
        await asyncio.sleep(0.1)  # wait for replication
        # publish some data
        data = b'{"hello": "world"}'
        await post(peerA.api3 / 'v1/publish/general', data, loop)

        with timeout(1):
            msg1 = await ws1.receive_json()
            assert msg1 == [
                "message", {"topic": "general"}, {"hello": "world"}]

        with pytest.raises(asyncio.TimeoutError):
            with timeout(1, loop=loop):
                assert await ws2.receive_json() is None


@pytest.mark.parametrize("through", ["peerA", "peerB"])
async def test_topic_unsubscribe(swindon_two, proxy_server, loop,
                                 user_id, through):
    peerA, peerB = swindon_two
    if through == 'peerA':
        control = peerA
    else:
        control = peerB
    urlA = peerA.url / 'swindon-lattice'
    # urlB = peerB.url / 'swindon-lattice'
    async with proxy_server(port=peerA.proxy.port) as proxy:
        handlerA = proxy.swindon_lattice(urlA, timeout=1)
        cid, ws = await auth(handlerA, {"user_id": user_id})

        topic_url = control.api3 / 'v1/connection'
        topic_url = topic_url / cid / 'subscriptions/xxxx'
        await put(topic_url, loop)

        # publish some data
        data = b'["hello", "from", "peerA"]'
        await post(peerA.api3 / 'v1/publish/xxxx', data, loop)
        msg = await ws.receive_json()
        assert msg == [
            "message", {"topic": "xxxx"}, ["hello", "from", "peerA"]]

        data = b'["hello", "from", "peerB"]'
        await post(peerB.api3 / 'v1/publish/xxxx', data, loop)
        msg = await ws.receive_json()
        assert msg == [
            "message", {"topic": "xxxx"}, ["hello", "from", "peerB"]]

        topic_url = control.api3 / 'v1/connection'
        topic_url = topic_url / cid / 'subscriptions/xxxx'
        await delete(topic_url, loop)
        # XXX: publish can be received earlier than unsubscribe "replicated"
        await asyncio.sleep(.05, loop=loop)

        # publish some data
        data = b'["hello", "world"]'
        await post(peerA.api3 / 'v1/publish/xxxx', data, loop)
        await post(peerB.api3 / 'v1/publish/xxxx', data, loop)
        with pytest.raises(asyncio.TimeoutError):
            with timeout(1, loop=loop):
                assert await ws.receive_json() is None


@pytest.mark.parametrize('by_user_id', [True, False])
async def test_swindon_user(proxy_server, swindon_two, loop,
        user_id, user_id2, by_user_id):

    peerA, peerB = swindon_two

    urlA = peerA.url / 'swindon-lattice'
    urlB = peerB.url / 'swindon-lattice'
    async with proxy_server(port=peerA.proxy.port) as proxy:
        handlerA = proxy.swindon_lattice(urlA, timeout=1)
        req = await handlerA.request()
        assert_auth(req)
        meta, args, kwargs = await req.json()
        cid1 = meta['connection_id']

        async with aiohttp.ClientSession(loop=loop) as s:
            subscr_url = peerA.api3 / 'v1/connection' / cid1 / 'users'
            data = json.dumps([user_id]).encode('utf-8')
            async with s.put(subscr_url,
                    headers={'Content-Type': 'application/json'},
                    data=data) as resp:
                assert resp.status == 204

        ws1 = await handlerA.json_response({
            "user_id": user_id, "username": "Jack"})
        hello = await ws1.receive_json()
        assert hello == [
            'hello', {}, {'user_id': user_id, 'username': 'Jack'}]
        msg = await ws1.receive_json()
        assert msg == ['lattice',
            {'namespace': 'swindon.user'},
            {user_id: {'status_register': [mock.ANY, 'active']}}]

        # Log in second user but subscribe him later

        handlerB = proxy.swindon_lattice(urlB, timeout=1)
        req = await handlerB.request()
        assert_auth(req)
        meta, args, kwargs = await req.json()
        cid2 = meta['connection_id']

        ws2 = await handlerB.json_response({
            "user_id": user_id2, "username": "John"})
        hello = await ws2.receive_json()
        assert hello == [
            'hello', {}, {'user_id': user_id2, 'username': 'John'}]

        # Now subscribe both users to each other
        async with aiohttp.ClientSession(loop=loop) as s:
            data = json.dumps([user_id, user_id2]).encode('utf-8')

            subscr_url = peerB.api3 / 'v1/connection' / cid2 / 'users'
            async with s.put(subscr_url,
                    headers={'Content-Type': 'application/json'},
                    data=data) as resp:
                assert resp.status == 204

            if by_user_id:
                subscr_url = peerB.api3 / 'v1/user' / user_id / 'users'
                async with s.put(subscr_url,
                    headers={'Content-Type': 'application/json'},
                    data=data) as resp:
                    assert resp.status == 204
            else:
                subscr_url = peerB.api3 / 'v1/connection' / cid1 / 'users'
                async with s.put(subscr_url,
                        headers={'Content-Type': 'application/json'},
                        data=data) as resp:
                    assert resp.status == 204

        with timeout(1):
            msg = await ws1.receive_json()
        # TODO(tailhook) this user is fully sent on resubscription
        #                we can optimize it to send only new users
        assert msg == ['lattice',
            {'namespace': 'swindon.user'},
            {user_id: {'status_register': [mock.ANY, 'active']},
             user_id2: {'status_register': [mock.ANY, 'active']}}]

        with timeout(1):
            msg = await ws2.receive_json()
        assert msg == ['lattice',
            {'namespace': 'swindon.user'},
            {user_id: {'status_register': [mock.ANY, 'active']},
             user_id2: {'status_register': [mock.ANY, 'active']}}]
