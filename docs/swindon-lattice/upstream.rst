Upstream backend requirements
=============================

Swindon will transform frontend WebSocket calls into HTTP requests.
This page describes requests format.

General requests format
-----------------------

All upstream requests are ``POST`` requests.

``Authorization`` header will hold base64 encoded JSON object
received on authorization step.

All requests contain valid JSON with the following structure:

.. code-block:: javascript

   [request_meta, args_array, kwargs_object]

``request_meta``
   A JSON object that contains metadata of the request.
   See :ref:`Request Meta <request-meta>` for details.

   Swindon will add ``connection_id`` field identifing current websocket
   connection, this ID must be used in API calls.

``args_array``
   Positional arguments for the remote method (JSON Array).

``kwargs_object``
   Named arguments for the remote method (JSON Object).

All responses are transformed into websocket message —
:ref:`method call result <call-result>`.

All endpoints can be prefixed with http destination prefix, for instance:

.. code-block:: yaml

   handlers:
      chat1: !SwindonChat
         message-handlers:
           "*": chat-host/
           tangle.*: auth-host/chat1
      chat2: !SwindonChat
         message-handlers:
           "*": chat-host/
           tangle.*: auth-host/chat2
   http-destinations:
      chat-host:
         addresses: [chat.example.com]
      auth-host:
         addresses: [auth.example.com]


Final request will be made against following URL:

.. sourcecode:: http

   POST /chat1/tangle/authorize_connection HTTP/1.1
   Host: auth.example.com
   Content-Type: application/json
   Content-Length: 11

   [{}, [], {}]

.. sourcecode:: http

   POST /chat/room/send_message HTTP/1.1
   Host: chat.example.com
   Content-Type: application/json
   Content-Length: 11

   [{}, [], {}]


Authorization request
---------------------

.. http:POST:: /tangle/authorize_connection

   Connection authorization request. If upstream server replies with
   invalid response websocket connection will be closed.

   Anything **except** HTTP status ``200 OK`` is considered invalid response.

   The upstream also **must** provide valid JSON object containing
   ``"user_id"`` string field. This JSON object will be send to websocket as
   ``hello`` message.

   .. seealso::
      :ref:`hello-message`.

      :doc:`websocket_shutdown_codes`.

   Example:

   .. sourcecode:: http

      POST /tangle/authorize_connection HTTP/1.1
      Content-Type: application/json
      Host: chat.example.com

      [{"connection_id": "W0XeqRFiPpdHXHU0"},
       [],
       {"http_cookie": "xxx=yyy", "http_authorization": "Token abc-etc",
        "url_querystring": "key=value&key=value&key=value"}]

   .. sourcecode:: http

      HTTP/1.1 200 OK
      Content-Type: application/json

      {"user_id": "user:1234", "username": "John"}

   Websocket message:

   .. sourcecode:: json

      ["hello", {}, {"user_id": "user:1234", "username": "John"}]


Inactive session notification
-----------------------------

.. http:POST:: /tangle/session_inactive

   Notifies upstream that user's session is inactive,
   see :ref:`Request Meta <request-meta>`.

   Example:

   .. sourcecode:: http

      POST /tangle/session_inactive HTTP/1.1
      Host: example.com
      Authorization: Token some/base64encoded/data=
      Content-Type: application/json
      Content-Length: 11

      [{}, [], {}]

Websocket calls
---------------

Method field of websocket call is transformed into request path
(``.`` are replaced with ``/`` and http destination prefix is added).

.. code-block:: json

   ["chat.send_message",
    {"request_id": 123},
    [{"text": "Arbitrary message"}],
    {"room": "room1"}
    ]

.. sourcecode:: http

   POST /chat/send_message HTTP/1.1
   Host: example.com
   Authorization: Token some/base64encoded/data=
   Content-Type: application/json
   Content-Length: 114

   [{"request_id": 123, "connection_id": "W0XeqRFiPpdHXHU0"},
    [{"text": "Arbitrary message"}],
    {"room": "room1"}
    ]
