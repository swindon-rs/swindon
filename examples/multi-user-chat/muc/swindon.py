import re
import json
import aiohttp


TOPIC_RE = re.compile("^[a-zA-Z0-9_-].")


class Swindon(object):

    def __init__(self, addr, loop):
        self.addr = addr
        self.prefix = 'http://{}:{}/v1/'.format(*self.addr)
        self.session = aiohttp.ClientSession(loop=loop)

    async def lattice(self, conn, namespace, initial_data):
        assert TOPIC_RE.match(namespace)
        async with self.session.put(self.prefix +
                'connection/{}/lattices/{}'.format(
                    conn.connection_id,
                    namespace),
                data=json.dumps(initial_data)) as req:
            assert req.status == 204, req.status
            await req.read()

    async def subscribe(self, conn, topic):
        assert TOPIC_RE.match(topic)
        async with self.session.put(self.prefix +
                'connection/{}/subscriptions/{}'.format(
                    conn.connection_id,
                    topic),
                data='') as req:
            assert req.status == 204, req.status
            await req.read()

    async def publish(self, topic, data):
        assert TOPIC_RE.match(topic)
        async with self.session.post(self.prefix + 'publish/' + topic,
                data=json.dumps(data)) as req:
            assert req.status == 204, req.status
            await req.read()


def connect(addr, loop):
    return Swindon(addr, loop=loop)

