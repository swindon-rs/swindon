.. _http_destinations:

Http Destinations
=================


Each HTTP destination defines a connection pool to uniformly distributed
service. For example:

.. code-block:: yaml

    http-destinations:

      myapp:

        load-balancing: queue
        queue-size-for-503: 100k
        backend-connections-per-ip-port: 1
        in-flight-requests-per-backend-connection: 1

        addresses:
        - example.com:5000

      httpbin:

        load-balancing: queue
        queue-size-for-503: 50
        backend-connections-per-ip-port: 5
        in-flight-requests-per-backend-connection: 2

        addresses:
        - httpbin.org:80

Here we define two connection pools with different pipelining settings (
``inflight-requests-per-backend-connection`` and different queue size.

You can use single http destination in multiple handlers. Handlers that support
http destination are ``!Proxy`` and ``!SwindonChat``, it's also used
as a ``inactivity-handler`` in ``session-pools``.

Options
-------

.. opt:: addresses

   A list of addresses to connect to. Currently you must specify also a port,
   but we consider using ``SRV`` records in the future.

   Each address may be resolved to a multiple IPs and each API participate in
   round-robin on it's own (not the whole hostname).

.. opt:: load-balancing

   (default ``queue``) Load-balancing kind, only ``queue`` is supported for now.

.. opt:: queue-size-for-503

   (default ``100000``) After this number of requests queued all subsequent
   requests that match this destination will be rejected immediately with
   the code 503 (service unavailable).

.. opt:: backend-connections-per-ip-port

   (default ``100``) This number of connections will be created to each backend
   specified in ``addresses`` or resolved by hostname.

   Optimal value for this setting depends on whether backends are written
   in asynchronous style or synchronous style and how many threads or processes
   are running on each machine.

.. opt:: keep-alive-timeout

   (default ``4 sec``) Time connection can be kept idle when no requests being
   sent. Note the default time is very conservative. You should upgrade it
   to as large as server on the other side is willing to keep connection open
   sans roundtrip time.

.. opt:: max-request-timeout

   (default ``30 secs``) Maximum time request is waiting for response. This
   time is accounted from the first byte sent to the last byte received.

   There are two important issues to consider:

   1. In the current implementation we don't cancel requests earlier than
      a timeout. I.e. if timeout in `!Handler` is smaller, request on backend
      will wait.
   2. Time request waits in the queue is not accounted in this timeout.

.. opt:: safe-pipeline-timeout

   (default ``300 ms``) Maximum time a single request is being handled by
   backend until we stop pipelining more requests into this connection. This
   timeout stems from two principles:

   1. Pipelining is only useful for quick request, on slower ones effect of
      pipelining is negligible.
   2. If backend starts to be slow, we should stop sending more requests as
      fast as possible.

   Note: we track this timeout on each individual connection, so it isn't
   suited very well for avoiding failed nodes, but rather to make effect of
   head of line blocking smaller. But with carefully set up
   ``backend-connections-per-ip-port`` it might help loosing smaller number
   of requests.

.. opt:: override-host-header

   (optional) Replace host header for the original request into this one.
   This is kind of rewrite of a request url if your backend accepts different
   domain name than frontend shows.

   .. note::

      This setting is currently required for handlers used for
      :opt:`message-handlers` for ``SwindonChat`` protocol. We're seeking for
      a way to provide sane default `Host` header for such handlers.

.. opt:: request-id-header

   (default is null) Creates a request id. Request id value depends on
   handler type:

   * for ``!Proxy`` -- see :doc:`/internals/request_id`;

   * for ``!SwindonChat`` handler -- a combination of server id, connection id
     and request id is used.
