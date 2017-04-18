import asyncio
from aiohttp import ClientSession as S
from aiohttp.web import json_response


async def test_simple_replication(swindon_two, proxy_server, loop):
    peerA, peerB = swindon_two
    await asyncio.sleep(1.5, loop=loop)  # wait reconnect
    await proxy_server.stop_server(shutdown=False)
    await proxy_server.start_server(peerA.proxy.port)

    urlA = peerA.url / 'swindon-chat'
    urlB = peerB.url / 'swindon-chat'
    async with proxy_server.swindon_chat(urlA, timeout=1) as call:
        req, fut = await call.request()
        assert req.path == '/tangle/authorize_connection'
        meta, *tail = await req.json()
        assert 'connection_id' in meta
        cid1 = meta['connection_id']

        fut.set_result(json_response({
            'user_id': 'replicated-user:1', 'username': 'John'}))

        ws1 = await call.websocket
        hello = await ws1.receive_json()
        assert hello == [
            'hello', {}, {'user_id': 'replicated-user:1', 'username': 'John'}]

        url = peerA.api / 'v1/connection' / cid1 / 'subscriptions' / 'general'
        async with S(loop=loop).put(url) as resp:
            assert resp.status == 204

        async with proxy_server.swindon_chat(urlB, timeout=1) as call:
            req, fut = await call.request()
            assert req.path == '/tangle/authorize_connection'
            meta, *tail = await req.json()
            assert 'connection_id' in meta
            cid2 = meta['connection_id']

            fut.set_result(json_response({
                'user_id': 'replicated-user:1', 'username': 'John'}))

            ws2 = await call.websocket
            hello = await ws2.receive_json()
            assert hello == [
                'hello', {}, {'user_id': 'replicated-user:1',
                              'username': 'John'}]

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
