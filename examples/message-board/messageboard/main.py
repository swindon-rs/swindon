from censusname import generate as make_name
from sanic import Sanic
from sanic.response import json as response


def main():
    app = Sanic('messageboard')

    @app.route("/tangle/authorize_connection", methods=['POST'])
    async def swindon(request):
        name = make_name()
        id = name.lower().replace(' ', '_')
        return response({
            'user_id': id,
            'username': name,
        })

    app.run(host="0.0.0.0", port=8082)
