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
   Names starting with ``tangle.*`` are reserved and cannot be called from
   client.

``request_meta``
   A dictionary (technically a Javascript/JSON object) that contains metadata
   of the request. All fields that are passed in this object are returned
   in ``response_meta``.

   Currently there are the following well-known fields:

      ``request_id``
         This field contains request identified. Swindon itself uses the
         field barely for logging purposes. But it's inteded to be used to
         match responses on the frontend. It should be either string or
         a non-negative integer.
      ``active``
         Time for which session should be considered active. This should
         be positive integer.

``args_array``
   Positional arguments for the remote method. They are passed to the backend
   as is (must be a valid JSON though).

``kwargs_object``
   Named arguments for the remote method.

.. note:: All four arguments are **required** even if some of them are empty.
