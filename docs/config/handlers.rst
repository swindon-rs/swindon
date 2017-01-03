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

.. opt:: stream-requests

   (default ``false``) If ``true`` requests bodies might be forwarded to the
   backend before whole body has been read.

   You shouldn't turn this option to ``true`` unless your backend is
   asynchronous too or can start processing request before receiving full body.

.. opt:: response-buffer-size

   (default ``10MiB``) A high water mark of buffering responses in swindon. If
   this number of bytes is reached swindon will stop reading response from a
   backend until client receives some data.

   Here are few tips for tweaking this value:

   1. The size of most of your pages (or other content served through this
      proxy) should be strictly less than this value, to have good performance.
      This holds true even for async backends written in scripting languages.
   2. For non-async backends even if just one page of your site doesn't fit
      the buffer it might make DoS attack super-easy unless this page is
      protected by some rate limit.
   3. Making this limit lower makes sense when you can generate data
      continuously, like fetching data from the database by chunks, or
      decompress data on the fly.
   4. Consider making use cases (1-2) and (3) separate routes with different
      limits.

