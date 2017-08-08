.. highlight:: yaml

Main Configuration
==================


`Configuration format basics <http://rust-quire.readthedocs.io/en/latest/user.html>`_.


Sections
--------

.. sect:: routing

   Describes routing table for all input requests. See :ref:`routing`

   Example::

     routing:
        localhost/empty.gif: empty-gif
        localhost/admin: admin-handler
        localhost/js: static-files
        localhost/: proxy-handler

.. sect:: handlers

   A mapping of handler name to the handler itself. See :ref:`handlers`

   Example::

      handlers:
        empty-gif: !EmptyGif
        proxy-handler: !Proxy
          destination: host1/

.. sect:: http-destinations

   Describes upstream servers. See ref:`http_destinations`

   Example::

      http-destinations:
         host1:
            addresses:
            - 127.0.0.1:8080


.. sect:: session-pools

   Describes session pools for chat protocol. See :ref:`sessions`

   Example::

      session-pools:
         example-chat-session:
            listen:
            - 127.0.0.1
            inactivity-handlers:
            - some-desctination/chat/route

.. sect:: disk-pools

   TBD

Options
-------

.. opt:: listen

   List of sockets to listen to and accept connections

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

.. opt:: first-byte-timeout

   (default ``5s``) Timeout receiving very first byte over connection

.. opt:: keep-alive-timeout

   (default ``90s``) Timeout of idle connection (when no request has been sent
   yet)

.. opt:: headers-timeout

   (default ``10s``) Timeout of receiving whole request headers

   This timeout starts when first byte of headers is received

.. opt:: input-body-byte-timeout

   (default ``15s``) Maximum delay between any two bytes of
   input request received

.. opt:: input-body-whole-timeout

   (default ``1 hour``) Timeout of whole request body received

.. opt:: output-body-byte-timeout

   (default ``15s``)

.. opt:: output-body-whole-timeout

   (default ``1 hour``) Timeout for the whole response body to be send to the
   client

   This timeout is taken literally for any response, so it must be
   as large as needed for slowest client fetching slowest file. I.e.
   it might be as big as a hour or day for some applications, but consider
   short timeouts if you don't serve large files to prevent DoS attacks.



.. opt:: debug-routing

   Enable ``X-Swindon-*`` headers in responses to debug routes chosen for
   this request.

   Note this option has performance and security implications.

   Currently we have the following headers:

   * ``X-Swindon-Route`` -- displays a handler chosen for serving a request
     (basically a value from the :ref:`routing table<routing>`).
   * ``X-Swindon-File-Path`` -- full path of the file that was served (or
     could be served if exists) for this request
   * ``X-Swindon-Request-Id`` -- identifier of the request if the one was
     generated for the request
   * ``X-Swindon-Authorizer`` -- denotes current authorizer used for request
     if any
   * ``X-Swindon-Allow`` -- denotes why the request was allowed to pass
   * ``X-Swindon-Deny`` -- denotes why the request was denied

   Note that headers are subject to change at any time.

.. opt:: debug-logging

   Enables logging of requests into stderr. This is usually useful for local
   debugging. Better logging functionality will be available in next release.

   .. versionadded:: v0.6.3

.. opt:: server-name

   Server name that will be sent in ``Server`` header. By default it's
   ``swindon/VERSION``, but it might also be ``null`` (don't send ``Server``
   header) or any other value.

.. opt:: set-user
.. opt:: set-group

   (no default) The name and group of the user to setuid into. This is useful
   if you run swindon as root. If only user is specified, we're doing
   `set-group` to user's primary group.

   The trick here is: first time we bind addresses before setting user, but
   when configuration is reloaded we might fail to bind to privileged ports.
