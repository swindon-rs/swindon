

Main Configuration
==================


`Configuration format basics <http://rust-quire.readthedocs.io/en/latest/user.html>`_.


Sections
--------

.. sect:: routing

   Describes routing table for all input requests. :ref:`routing`

   Example::

     routing:
        localhost/empty.gif: empty-gif
        localhost/admin: admin-handler
        localhost/js: static-files
        localhost/: proxy-handler

.. sect:: handlers

    A mapping of handler name to the handler itself. :ref:`handlers`

Options
-------

.. opt:: listen

   Listen of sockets to listen and accept connections

   Example::

    listen:
    - 127.0.0.1:80
    - 127.0.0.1:8080

.. opt:: max-connections

   (default ``1000``) Maximum number of client connections to accept. Note
   you should bump up a file descriptor limit to something larger than this
   value + number of potential proxy/backend connections.

   Note: currently max-connections is obeyed per each listening address
   separately. We're considering to change this behavior in future.

.. opt:: pipeline-depth

   (default ``2``) Accept maximum N in-flight requests for each HTTP
   connection. Pipelined requests improve performance of your service but also
   expose it to DoS attacks.

   The possible DoS attack vector is: client can send multiple requests and
   they will be forwarded to backends regardless of whether previous ones are
   read from client. Which effectively means that active requests can be up
   to ``max-connections × pipeline-depth``.

.. opt:: listen-error-timeout

   (default ``100ms``) Time to sleep when we caught error accepting connection,
   mostly error is some resource shortage (usually EMFILE or ENFILE), so
   repeating after some short timeout makes sense (chances that some connection
   freed some resources).

.. opt:: debug-routing

   Enable ``X-Swindon-*`` headers in responses to debug routes chosen for
   this request.

   Note this option has performance and security implications.

   Currently we have the following headers:

   * ``X-Swindon-Route`` -- displays a handler chosen for serving a request
     (basically a value from the :ref:`routing table<routing>`).
   * ``X-Swindon-File-Path`` -- full path of the file that was served (or
     could be served if exists) for this request

   Note that headers are subject to change at any time.
