import re
import json
import aiohttp

TOPIC_RE = re.compile("^[a-zA-Z0-9_-].")


class Swindon(object):

    def __init__(self, addr):
        self.addr = addr
        self.prefix = 'http://{}:{}/v1/'.format(*self.addr)
        self.session = aiohttp.ClientSession()

    async def subscribe(self, conn, topic):
        assert TOPIC_RE.match(topic)
        async with self.session.put(self.prefix +
                'connection/{}/subscriptions/{}'.format(
                    conn.connection_id,
                    topic),
                data='') as req:
            await req.read()


    async def publish(self, topic, data):
        assert TOPIC_RE.match(topic)
        async with self.session.post(self.prefix + 'publish/' + topic,
                data=json.dumps(data)) as req:
            await req.read()


def connect(addr):
    return Swindon(addr)

