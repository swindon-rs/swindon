.. highlight:: yaml

.. _routing:

Routing Table
=============

Describes routing table for all input requests.
It is a mapping of either exact host and path prefix,
or host suffix and path path prefix
to the name of the :ref:`handler <handlers>`.

Example of routing table::

   routing:
     localhost/empty.gif: empty-gif
     localhost/admin: admin-handler
     localhost/js: static-files
     localhost/: proxy-handler
     "*.example.com/": all-subdomains-handler
     www.example.com/favicon.ico: favicon-handler

Route rosolution is done in two steps:

   1. The host is matched first:

      a) exact match is tested first (``www.example.com``),

      b) then match by suffix is checked (``*.example.com``).

   2. The path prefix within that host is matched.

Here is the example for route matching:

Assume we requested ``www.example.com/hello`` URL,
at first step the host ``www.example.com`` will be matched
with last entry in table above, next, path ``/hello`` will
be tested against all pathes for that host -- only one in our case --
and ``/favicon.ico`` path doesn't match ``/hello``.
So the request for ``www.example.com/hello`` will end up with ``404 Not Found``.
