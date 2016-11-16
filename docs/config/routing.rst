.. _routing:

Routing Table
=============

Describes routing table for all input requests. Basically it's a mapping of
prefix to the name of the :ref:`handler <handler>`.

Example of routing table::

    routing:
        localhost/empty.gif: empty-gif
        localhost/admin: admin-handler
        localhost/js: static-files
        localhost/: proxy-handler
