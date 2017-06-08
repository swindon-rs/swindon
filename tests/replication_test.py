import asyncio
import pytest
from async_timeout import timeout
from aiohttp import ClientSession as S
from aiohttp.web import json_response


async def auth(auth_call, auth_data):
    req, fut = await auth_call.request()
    assert req.path == '/tangle/authorize_connection'
    meta, *tail = await req.json()
    assert 'connection_id' in meta
    cid = meta['connection_id']
    fut.set_result(json_response(auth_data))
    ws = await auth_call.websocket
    hello = await ws.receive_json()
    assert hello == ['hello', {}, auth_data]
    return cid, ws


async def test_simple_replication(swindon_two, proxy_server, loop):
    peerA, peerB = swindon_two
    # await asyncio.sleep(1.5, loop=loop)  # wait reconnect
    await proxy_server.stop_server(shutdown=False)
    await proxy_server.start_server(peerA.proxy.port)

    urlA = peerA.url / 'swindon-chat'
    urlB = peerB.url / 'swindon-chat'
    async with proxy_server.swindon_chat(urlA, timeout=1) as call:
        cid1, ws1 = await auth(
            call, {'user_id': 'replicated-user:1', 'username': 'John'})

        url = peerA.api / 'v1/connection' / cid1 / 'subscriptions' / 'general'
        async with S(loop=loop).put(url) as resp:
            assert resp.status == 204

        async with proxy_server.swindon_chat(urlB, timeout=1) as call:
            cid2, ws2 = await auth(
                call, {'user_id': 'replicated-user:1', 'username': 'John'})

            url = peerB.api / 'v1/connection' / cid2
            url = url / 'subscriptions' / 'general'
            async with S(loop=loop).put(url) as resp:
                assert resp.status == 204

            # subscribe both to some topic and publish into one peer.

            data = b'{"test": "message"}'
            async with S(loop=loop).post(
                    peerB.api / 'v1/publish/general', data=data) as resp:
                assert resp.status == 204

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
    await proxy_server.stop_server(shutdown=False)
    await proxy_server.start_server(peerA.proxy.port)

    # The use case:
    # * client A connect to peer A; receive peer-local cid(0);
    # * client B connect to peer B; receive peer-lcoal cid(0);
    # * backend sends request to peer A to subscribe client A to some topic
    # * action replicated to peer B;
    # Expected result:
    # client B not subscribed to that topic
    urlA = peerA.url / 'swindon-chat'
    urlB = peerB.url / 'swindon-chat'
    async with proxy_server.swindon_chat(urlA, timeout=1) as call:
        cid1, ws1 = await auth(
            call, {'user_id': 'replicated-user:1', 'username': 'John'})

        async with proxy_server.swindon_chat(urlB, timeout=1) as call:
            cid2, ws2 = await auth(
                call, {'user_id': 'replicated-user:2', 'username': 'John'})

            # Subscribe only first user;
            url = subscribe_peer.api / 'v1/connection'
            url = url / cid1 / 'subscriptions/general'
            async with S(loop=loop).put(url) as resp:
                assert resp.status == 204
            # publish some data
            url = peerA.api / 'v1/publish/general'
            data = b'{"hello": "world"}'
            async with S(loop=loop).post(url, data=data) as resp:
                assert resp.status == 204

            msg1 = await ws1.receive_json()
            assert msg1 == [
                "message", {"topic": "general"}, {"hello": "world"}]

            with pytest.raises(asyncio.TimeoutError):
                with timeout(1, loop=loop):
                    assert await ws2.receive_json() is None
