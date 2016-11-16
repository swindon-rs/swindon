

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
