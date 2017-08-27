====================
Example Applications
====================

There are examples in the swindon repository, which you might want to
study to better understand the underlying protocols. There are four
examples:

1. `message-board`_ -- displays a list of messages from every user joined
   the chat, using pub-sub. Does not represent all the details of the protocol
   but it has < 60 lines of raw javascript code without any libraries, so
   it's easy to grok for people with different backgrounds. Backend is in
   python3_ and sanic_ (for some coincidence)
2. `message-board2`_ -- basically the same but uses a `swindon-js`_ helper
   library for the frontend. And `aiohttp.web`_ for backend.
3. `multi-user-chat`_ -- is a more powerful chat application with rooms and
   using both lattices and pubsub for keeping state up to date. Uses
   `create-react-app`_ for bootstrapping the application and sanic sanic_ for
   backend.
4. `multi-user-chat2`_ -- is basically the same thing, but uses `swindon-js`_
   library for communicating with swindon.

.. _message-board: https://github.com/swindon-rs/swindon/tree/master/examples/message-board
.. _message-board2: https://github.com/swindon-rs/swindon/tree/master/examples/message-board2
.. _multi-user-chat: https://github.com/swindon-rs/swindon/tree/master/examples/multi-user-chat
.. _multi-user-chat2: https://github.com/swindon-rs/swindon/tree/master/examples/multi-user-chat2
.. _python3: http://python.org
.. _sanic: https://github.com/channelcat/sanic/
.. _aiohttp.web: http://aiohttp.readthedocs.io/
.. _swindon-js: https://npmjs.com/package/swindon
.. _create-react-app: https://github.com/facebookincubator/create-react-app
