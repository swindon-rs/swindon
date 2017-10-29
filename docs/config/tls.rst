TLS Support
===========

Swindon has experimental support of TLS, curently only TLS client is supported.


Client Configuration
--------------------

To enable TLS in ``http-destination`` you need to set client configuration:

.. code-block:: yaml

    http-destinations:

      myapp:
        tls: default  # << default config
        addresses:
        - example.com:5000

This uses default TLS settings and system-default certificate bundle. Or
you can fine tune settings:

.. code-block:: yaml

    tls-client-settings:
      my-app-certs:
        certificates:
        - /etc/certs/myapp.crt

    http-destinations:

      myapp:
        tls: my-app-certs
        addresses:
        - example.com:5000
