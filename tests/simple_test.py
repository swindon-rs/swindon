import aiohttp


async def test_empty_gif(swindon, loop, request, debug_routing):
    async with aiohttp.ClientSession(loop=loop) as sess:
        async with sess.get(swindon.url / 'empty.gif') as resp:
            assert resp.status == 200
            assert resp.headers['Content-Type'] == 'image/gif'
            assert resp.headers['Content-Length'] == '26'
            assert resp.headers['Server'] == 'swindon/func-tests'
            if debug_routing:
                assert resp.headers['X-Swindon-Route'] == 'empty_gif'
            else:
                assert 'X-Swindon-Route' not in resp.headers
            data = await resp.content.read()
            assert len(data) == 26
