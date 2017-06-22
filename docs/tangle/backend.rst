Backend API Reference
=====================

Pub/Sub subscriptions
---------------------

.. http:PUT:: /v1/connection/(str:conn_id)/subscriptions/(path:topic)

   Subscribe client with ``conn_id`` to specific topic ``topic``.

   Request body is not used.

   Client will receive ``topic`` with ``/`` substituted with ``.``,
   for instance the ``channel/general`` topic will become
   ``channel.general`` in client message.

   Example:

   .. sourcecode:: http

      PUT /v1/connection/nb9NC-HpR/subscriptions/channel/general HTTP/1.1
      Host: example.com

   .. sourcecode:: http

      HTTP/1.1 204 No Content
      Content-Length: 0


.. http:DELETE:: /v1/connection/(str:conn_id)/subscriptions/(path:topic)

   Unsubscribe client with ``conn_id`` from specific ``topic``.

   Request body is not used.

   Example:

   .. sourcecode:: http

      DELETE /v1/connection/nb9NC-HpR/subscriptions/channel/general HTTP/1.1
      Host: example.com

   .. sourcecode:: http

      HTTP/1.1 204 No Content
      Content-Length: 0

.. _topic-publish:

.. http:POST:: /v1/publish/(path:topic)

   Publish JSON message to all subscribed clients.
   See also :ref:`front-messsage`.

   Request body **must** be a valid JSON.

   Example:

   .. sourcecode:: http

      POST /v1/publish/channel/general HTTP/1.1
      Host: example.com
      Content-Type: application/json
      Content-Length: 26

      {"message": "Hello World"}

   .. sourcecode:: http

      HTTP/1.1 204 No Content
      Content-Length: 0


Lattice subscriptions
---------------------

.. http:PUT:: /v1/connection/(str:conn_id)/lattices/(path:namespace)

   Subscribe client with ``conn_id`` to lattice namespace ``namespace``.

   Example:

   .. sourcecode:: http

      PUT /v1/connection/nb9NC-HpR/lattices/test-chat/rooms HTTP/1.1
      Host: example.com
      Content-Type: application/json
      Content-Length: 223

      {"shared": {
          "room1": {"last_message_counter": 123},
          "room2": {"last_message_counter": 245}},
       "private": {
         "132565": {
              "room1": {"last_seen_counter": 120},
              "room2": {"last_seen_counter": 245}}
      }}

   .. sourcecode:: http

      HTTP/1.1 204 No Content
      Content-Length: 0

.. http:DELETE:: /v1/connection/(str:conn_id)/lattices/(path:namespace)

   Unsubscribe client with ``conn_id`` from lattice ``namespace``.

   Example:

   .. sourcecode:: http

      DELETE /v1/connection/nb9NC-HpR/lattices/test-chat/rooms HTTP/1.1
      Host: example.com
      Content-Length: 0

   .. sourcecode:: http

      HTTP/1.1 204 No Content
      Content-Length: 0

.. http:POST:: /v1/lattice/(path:namespace)

   Publish an update to lattice namespace.

   Example:

   .. sourcecode:: http

      POST /v1/lattice/test-chat/rooms HTTP/1.1
      Host: example.com
      Content-Type: application/json
      Content-Length: 308

      {"shared": {
          "room1": {"last_message_counter": 123},
          "room2": {"last_message_counter": 245}},
       "private": {
         "7777": {
          "room1": {"last_seen_counter": 120},
          "room2": {"last_seen_counter": 245}},
         "8734": {
          "room1": {"last_seen_counter": 123},
          "room2": {"last_seen_counter": 24}}
      }}

   .. sourcecode:: http

      HTTP/1.1 204 No Content
      Content-Length: 0


Parameters specification
------------------------


The following table shows allowed character sets for corresponding URL
componets.

   +---------------+-------------------+
   | Parameter     | Valid Characters  |
   +===============+===================+
   | Topic         | ``a-zA-Z0-9:_/-`` |
   +---------------+-------------------+
   | Namespace     | ``a-zA-Z0-9_/-``  |
   +---------------+-------------------+
   | Connection ID | ``a-zA-Z0-9_-``   |
   +---------------+-------------------+

Valid topics/namespaces example::

   valid/topic/value
   valid-topic-value
   valid_topic_value
   still-valid-topic/value:1
   1/2/3
   valid/namespace

Invalid topics/namespaces::

   not,a,valid,topic
   topic.with.dots


API response status codes
-------------------------

In case of valid request (correct URL and valid data) endpoints
respond with ``204 No Content`` status code.
This must be considered successful call.

Any of above endpoints call also respond with following codes signaling error:

* ``400 Bad Request``

   May be returned in the following cases:

   * Request with body sent to endpoint that does not expect body.

   * Request without body sent to endpoint that expects body.

   * Request body is not valid JSON.

   * Request JSON body has invalid format (for lattice endpoints).

* ``404 Not Found``

   May be caused by either invalid URL path (or one of its components) or
   unsupported request method.
