Replication configuration
=========================


Example
-------

.. code-block:: yaml

   replication:
      listen:
      - 0.0.0.0:7878

      peers:
      - peer2:7878
      - peer3:7878

      max-connections: 10
      listen-error-timeout: 100ms
      reconnect-timeout: 5s


Options
-------

.. opt:: listen

   A list of addresses to bind to.

.. opt:: peers

   A list of peer names to connect to.

.. opt:: max-connections

   (default ``10``) Maximum number of client connections to accept.

.. opt:: listen-error-timeout

   (default ``100ms``) Time to sleep when we caught error accepting connection.

.. opt:: reconnect-timeout

   (default ``5s``) Time to sleep between retrying to connect to peer.
