from aiohttp import web


async def test_simple_userinfo(proxy_server, swindon):
    url = swindon.url / 'swindon-chat'
    async with proxy_server.swindon_chat(url) as inflight:
        req, fut = await inflight.req.get()
        fut.set_result(web.Response(text='{"user_id": "user:1"}'))
        ws = await inflight.client_resp
        msg = await ws.receive_json()
        assert msg == ['hello', {}, {'user_id': 'user:1'}]
