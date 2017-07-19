Frontend API Reference
======================

Javascript/browser client connects to the swindon via websocket. We authorize
websocket by headers (``Cookie``, ``Authorization``, ``request-uri``,
or any other header) and then we keep that authorization data for future
requests via same websocket.

On top of the websockets we implement request-reply pattern from client
to server and multiple patterns from server to client.

Request Format
--------------

.. code-block:: javascript

   [method, request_meta, args_array, kwargs_object]


``method``
   Name of the remote method to call (example: ``"chat.send_message"``).
   Names starting with ``tangle.*`` and ``swindon.*`` are reserved and cannot
   be called from client.

.. _request-meta:

``request_meta``
   A dictionary (technically a Javascript/JSON object) that contains metadata
   of the request. All fields that are passed in this object are returned
   in ``response_meta``.

   Currently there are the following well-known fields:

      ``request_id``
         This field contains request identified. Swindon itself uses the
         field barely for logging purposes. But it's inteded to be used to
         match responses on the frontend. It should be either non-negative
         integer or ascii string up to 36 chars long
         (with valid characters ``a-z``, ``A-Z``, ``0-9``, ``-``, ``_``).
      ``active``
         Time (in seconds) for which session should be considered active.
         This should be positive integer.

``args_array``
   Positional arguments for the remote method. They are passed to the backend
   as is (must be a valid JSON array though).

``kwargs_object``
   Named arguments for the remote method (must be valid JSON object).

.. note:: All four arguments are **required** even if some of them are empty.


Example
~~~~~~~

Request:

.. code-block:: json

   ["chat.send_message",
    {"request_id": 123},
    [{"text": "Arbitrary message"}],
    {"room": "room1"}
    ]

Possible responses:

.. code-block:: json

   # Success result
   ["result", {"request_id": 123}, result_json_object]

   # Error results
   ["error", {"request_id": 123, "tangle_code": "http_error", "error_code": 400}, json_body_object]
   ["error", {"request_id": 123, "tangle_code": "validation_error"},
    json_body_object]


Response Format
---------------

.. code-block:: javascript

   [event_type, response_meta, data]

``event_type``
   Event type (see `Event Types`_)

``response_meta``
   A dictionary (a object in terms of Javascript/JSON) that contains
   auxilliary data about the event.

   For responses this dictionary contains fields from ``request_meta``

``data``
   Event data. Type and format of this value depends on ``event_type``

Event Types
-----------

.. contents::
   :local:


.. _call-result:

Method Call Result (``result``/``error``)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

:event_type: ``result``

.. code-block:: javascript

   ["result", {"request_id": 123}, json_result_object]


**Error result**

:event_type: ``error``

.. code-block:: javascript

   ["error",
    {"request_id": 123, "error_kind": "http_error", "http_error": 400},
    json_body_object]

   ["error",
    {"request_id": 123, "error_kind": "validation_error"},
    "invalid method"]

In case of error ``response_meta`` always has ``error_kind`` field.
Other fields may contain error details depending on the type of error.

Possible ``error_kind`` values:

   ``http_error``
      HTTP error from backend server. This error contains additional field
      ``http_error`` which contains *HTTP status code*. The ``data`` field
      may contain error data if response has
      ``Content-Type: application/json`` and valid JSON body.

   ``validation_error``
      Error validating request. ``data`` contains addition information.

   ``data_error``
      Error related to decoding response from a backend.
      ``data`` field contains string describing an error.
      Possible causes:

      * wrong (unsupported) ``Content-Type`` header;

      * not a JSON or malformed JSON response;

   ``internal_error``
      Swindon encountered internal error while processing the request.
      ``data`` field contains string describing an error.

.. :: NOTE: These two were never used, thus dropped;
   ``invalid_content_type``
      Wrong (i.e. unsupported) ``Content-Type`` in response from a backend.
   ``forbidden``
      This call is forbidden to call from frontend. This is used when you
      are trying to call ``tangle.*`` or ``swindon.*`` methods. These names
      are reserved for calls initiated by swindon.


.. _hello-message:

User Information (``hello``)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

   :event_type: ``hello``

   .. code-block:: json

      ["hello", {}, {"username": "John"}]

   Initial event sent just after websocket handshake is complete, which
   in turn means backend has authorized connection.

   Format of the data sent (third item in the tuple above) is defined
   by a backend (i.e. it's JSON data sent from a backend).
   See :ref:`backend-auth` for more info.

.. _front-message:

Message (``message``)
~~~~~~~~~~~~~~~~~~~~~

   :event_type: ``message``

   .. code-block:: json

      ["message",
       {"topic": "test-chat.room1"},
       {"id": 1,
        "message": "...",
        "author": ".."
        }]

   This message type is used to propagate published messages to frontend.
   See :ref:`Pub/Sub subscriptions <topic-publish>` for more info.


Lattice Update (``lattice``)
~~~~~~~~~~~~~~~~~~~~~~~~~~~~

   More about lattice updates in :ref:`lattice-definition`

   :event_type: ``lattice``

   .. code-block:: json

      ["lattice",
       {"namespace": "test-chat.rooms"},
       {
         "room1": {
           "last_message_count": 2,
           "last_seen_count": 3
         },
         "room2": {
           "last_message_count": 123
         },
      }]
