import os
import logging
import uvloop
import argparse
from censusname import generate as make_name
from sanic import Sanic
from sanic.response import json as response

from .convention import swindon_convention
from .swindon import connect

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--port', default=8082, help="Listen port")
    ap.add_argument('--swindon-port', default=8081,
        help="Connect to swindon at port")
    options = ap.parse_args()

    logging.basicConfig(level=logging.DEBUG)
    loop = uvloop.new_event_loop()
    app = Sanic('messageboard')
    swindon = connect(('localhost', options.swindon_port), loop=loop)

    @app.route("/tangle/authorize_connection", methods=['POST'])
    @swindon_convention
    async def auth(req, http_authorization, http_cookie, url_querystring):
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


    if os.environ.get("LISTEN_FDS") == '1':
        # Systemd socket activation protocol
        import socket
        sock = socket.fromfd(3, socket.AF_INET, socket.SOCK_STREAM)
        app.run(host=None, port=None, sock=sock, loop=loop)
    else:
        app.run(host="0.0.0.0", port=options.port, loop=loop)
