.. _handlers:

Handlers
========


Proxy Handler
=============

Proxy handler looks like::

  example-chat-http: !Proxy
    ip-header: X-Remote-Ip
    destination: somedest/some/path

Settings:

.. opt:: destination

   (required) The name of the destination and *subpath* where to forward
   request to.

.. opt:: ip-header

   (default is null) Name of the HTTP header where to put source IP address to.

.. opt:: max-payload-size

   (default ``10MiB``) Maximum payload size that might be send to this
   destination

