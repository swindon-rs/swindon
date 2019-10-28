.. _sessions:

.. highlight:: yaml

============
Session Pool
============


Session pool is a fully-isolated namespace of swindon chat service with
it's own address for backend connections. Each client websocket can connect
to exactly one session pool.

Note: while it's tempting to use a session pool per application, it may or
may not make sense for your specific case. You may combine multiple
"applications" under umbrella of a single session pool to connect all of them
using a single websocket. Each session pool contains multiple namespaces of
"lattices" and you can arbitrarily nest pub-sub topics, so there are plenty
room for isolating and integrating multiple applications in the session
pool.


Example
=======

.. code-block:: yaml

    session-pools:

      example-chat-session:
        listen:
        - 127.0.0.1:2007
        inactivity-handlers:
        - some-destination/chat/route


Options
=======

.. opt:: listen

   List of sockets to listen to and accept connections

   Example::

    listen:
    - 127.0.0.1:2222
    - 127.0.0.1:3333

.. opt:: max-connections

   (default ``1000``) Maximum number of backend connections to accept. Note
   you should bump up a file descriptor limit to something larger than this
   value + max client connections.

.. opt:: max-payload-size

   (default ``10Mib``) Maximum size of the payload (json data) from backend
   to swindon.

.. opt:: pipeline-depth

   (default ``2``) Accept maximum N in-flight requests for each HTTP
   connection. Pipelined requests improve performance of your service but also
   expose it to DoS attacks.

.. opt:: listen-error-timeout

   (default ``100ms``) Time to sleep when we caught error accepting connection,
   mostly error is some resource shortage (usually EMFILE or ENFILE), so
   repeating after some short timeout makes sense (chances that some connection
   freed some resources).


.. opt:: inactivity-handlers
   TBD

