Swindon-lattice Protocol
========================

The protocol is higher level-protocol on top of websockets that allows routing
messages between backends, push from any backend to any user, publish-subscribe
and few other useful things.

The protocol is designed to cover large number of cases including different
applications covered by single websocket connection for efficiency.

The protocol is `registered by IANA <iana_ws>`_ as ``v1.swindon-lattice+json``
and this value needs to be passed in ``Sec-WebSocket-Protocol`` field in
handshake.

.. toctree::

    lattices
    crdt
    frontend
    upstream
    backend
    websocket_shutdown_codes

.. _iana_ws: https://www.iana.org/assignments/websocket/websocket.xhtml
