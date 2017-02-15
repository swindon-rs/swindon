import pytest
import asyncio
import aiohttp


async def test_echo_chat(proxy_server, swindon):
    url = swindon.url / 'websocket-echo'
    async with aiohttp.ClientSession() as s:
        async with s.ws_connect(url) as ws:
            ws.send_str('Hello')
            assert await ws.receive_str() == 'Hello'

            ws.send_bytes(b'How are you?')
            assert await ws.receive_bytes() == b'How are you?'

            ws.send_json(["I'm", "fine", "thanks!"])
            assert await ws.receive_json() == ["I'm", "fine", "thanks!"]

            with pytest.raises(asyncio.TimeoutError):
                assert await ws.receive_str(timeout=.1) is None

            ws.ping()
            with pytest.raises(asyncio.TimeoutError):
                assert not await ws.receive(timeout=.1)
