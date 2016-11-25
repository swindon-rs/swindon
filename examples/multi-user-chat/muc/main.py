import re
import logging
import uvloop
from http.cookies import SimpleCookie
from sanic import Sanic
from sanic.response import json as response

from .convention import swindon_convention
from .swindon import connect
from . import chat


NON_ALPHA = re.compile('[^a-z0-9_]')


def main():
    logging.basicConfig(level=logging.DEBUG)
    loop = uvloop.new_event_loop()
    app = Sanic('multi-user-chat')
    swindon = connect(('localhost', 8081), loop=loop)

    @app.route("/tangle/authorize_connection", methods=['POST'])
    @swindon_convention
    async def auth(req, http_authorization, http_cookie):
        name = SimpleCookie(http_cookie)['swindon_muc_login'].value
        uid = NON_ALPHA.sub('_', name.lower())
        user = chat.ensure_user(uid, username=name)
        await swindon.lattice(req.connection, 'muc', user.initial_lattice())
        return {
            'user_id': uid,
            'username': name,
        }

    @app.route("/message", methods=['POST'])
    @swindon_convention
    async def message(req, text):
        await swindon.publish('message-board', {
            'author': req.user.username,
            'text': text,
            })
        return True


    app.run(host="0.0.0.0", port=8082, loop=loop)
