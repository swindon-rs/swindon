import re
import json
import aiohttp

TOPIC_RE = re.compile("^[a-zA-Z0-9_-].")


class Swindon(object):

    def __init__(self, addr):
        self.addr = addr
        self.prefix = 'http://{}:{}/v1/'.format(*self.addr)
        self.all_users = set()
        self.session = aiohttp.ClientSession()

    async def attach_users(self, conn, namespace):
        assert TOPIC_RE.match(namespace)
        async with self.session.put(self.prefix +
                'connection/{}/users'.format(conn.connection_id),
                data=json.dumps(list(self.all_users))) as req:
            assert req.status == 204, req.status
            res = await req.read()
            print("RES", res)


def connect(addr):
    return Swindon(addr)

