.. highlight:: yaml

.. _routing:

Routing Table
=============

Describes routing table for all input requests.
It is a mapping of either exact host and path prefix,
or host suffix and path path prefix
to the name of the :ref:`handler <handler>`.

Example of routing table::

   routing:
     localhost/empty.gif: empty-gif
     localhost/admin: admin-handler
     localhost/js: static-files
     localhost/: proxy-handler
     *.example.com/: all-subdomains-handler

Priority is given to longest match first, for instance
having the following routing table::

   routing:
      *.example.com: handler-a
      *.example.com/hello: handler-b
      www.example.com: handler-c

So request like ``www.example.com/hello`` will be passed to ``handler-c``.
