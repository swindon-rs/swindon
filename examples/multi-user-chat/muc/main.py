import os
import re
import logging
import uvloop
import argparse
from http.cookies import SimpleCookie
from sanic import Sanic
from sanic.response import json as response

from .convention import swindon_convention
from .swindon import connect
from . import chat


NON_ALPHA = re.compile('[^a-z0-9_]')


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--port', default=8082, help="Listen port")
    ap.add_argument('--swindon-port', default=8081,
        help="Connect to swindon at port")
    options = ap.parse_args()

    logging.basicConfig(level=logging.DEBUG)
    loop = uvloop.new_event_loop()
    app = Sanic('multi-user-chat')
    swindon = connect(('localhost', options.swindon_port), loop=loop)

    @app.route("/tangle/authorize_connection", methods=['POST'])
    @swindon_convention
    async def auth(req, http_authorization, http_cookie):
        name = SimpleCookie(http_cookie)['swindon_muc_login'].value
        uid = NON_ALPHA.sub('_', name.lower())
        user = chat.ensure_user(uid, username=name)
        await swindon.attach(req.connection, 'muc', user.initial_lattice())
        return {
            'user_id': uid,
            'username': name,
        }

    @app.route("/muc/enter_room", methods=['POST'])
    @swindon_convention
    async def enter_room(req, room_name):
        user  = chat.get_user(req.user_id)
        await swindon.subscribe(req.connection, 'muc.' + room_name)
        await swindon.lattice('muc', user.add_room(room_name))
        return True

    @app.route("/muc/leave_room", methods=['POST'])
    @swindon_convention
    async def enter_room(req, room_name):
        await swindon.unsubscribe(req.connection, 'muc.' + room_name)
        return True

    @app.route("/muc/switch_room", methods=['POST'])
    @swindon_convention
    async def enter_room(req, old_room, new_room):
        user = chat.get_user(req.user_id)
        await swindon.unsubscribe(req.connection, 'muc.' + old_room)
        await swindon.subscribe(req.connection, 'muc.' + new_room)
        await swindon.lattice('muc', user.add_room(new_room))
        return True

    @app.route("/muc/message", methods=['POST'])
    @swindon_convention
    async def message(req, room, text):
        msg = chat.get_room(room).add(
            chat.get_user(req.user_id).username,
            text)
        await swindon.publish('muc.' + room, msg)
        await swindon.lattice('muc', {
            'shared': { room: { 'last_message_counter': msg['id'] }},
            'private': {
                # you have always seen your own message
                req.user_id:
                    {room: { 'last_seen_counter': msg['id']}},
            },
        })
        return msg['id']

    @app.route("/muc/get_history", methods=['POST'])
    @swindon_convention
    async def get_history(req, room, start=0):
        return chat.get_room(room).get_history(start)

    if os.environ.get("LISTEN_FDS") == '1':
        # Systemd socket activation protocol
        import socket
        sock = socket.fromfd(3, socket.AF_INET, socket.SOCK_STREAM)
        app.run(host=None, port=None, sock=sock, loop=loop)
    else:
        app.run(host="0.0.0.0", port=options.port, loop=loop)
