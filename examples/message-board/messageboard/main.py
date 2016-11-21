import logging
import uvloop
from censusname import generate as make_name
from sanic import Sanic
from sanic.response import json as response

from .convention import swindon_convention
from .swindon import connect

def main():
    logging.basicConfig(level=logging.DEBUG)
    loop = uvloop.new_event_loop()
    app = Sanic('messageboard')
    swindon = connect(('localhost', 8081), loop=loop)

    @app.route("/tangle/authorize_connection", methods=['POST'])
    @swindon_convention
    async def auth(req, http_authorization, http_cookie):
        name = make_name()
        id = name.lower().replace(' ', '_')
        await swindon.subscribe(req.connection, 'message-board')
        return {
            'user_id': id,
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
