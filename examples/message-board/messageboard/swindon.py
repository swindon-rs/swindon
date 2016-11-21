import aiohttp


class Swindon(object):

    def __init__(self, addr):
        self.addr = addr
        self.session = aiohttp.ClientSession()


def connect(addr):
    return Swindon(addr)

