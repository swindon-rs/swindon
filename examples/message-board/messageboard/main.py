import logging
from censusname import generate as make_name
from sanic import Sanic
from sanic.response import json as response

from .convention import swindon_convention
from .swindon import connect

def main():
    logging.basicConfig(level=logging.DEBUG)
    app = Sanic('messageboard')
    swindon = connect(('localhost', 8081))

    @app.route("/tangle/authorize_connection", methods=['POST'])
    async def swindon(request):
        name = make_name()
        id = name.lower().replace(' ', '_')
        return response({
            'user_id': id,
            'username': name,
        })

    @app.route("/message", methods=['POST'])
    @swindon_convention
    async def message(req, text):
        print(req)


    app.run(host="0.0.0.0", port=8082)
