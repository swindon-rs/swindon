LDAP Support
============

Swindon has experimental support of LDAP authorization.

Configuration of LDAP consists of three parts:

1. Configuring LDAP destination. This is where addresses and size of connection
   pool are configured.
2. Actual LDAP search and bind requests are configured in ``authorizers``
   section with ``!Ldap`` authorizer.
3. And the last but least thing is to add authorizer configured at step #2 to
   actually handle parts of the site.

LDAP Destination
----------------

Currently destination has minimum configuration:

.. code-block:: yaml

   ldap-destinations:
     local-ldap:
       addresses:
       - localhost:8398

Options:

.. opt:: addresses

   A list of addresses to connect to. Currently you must specify also a port,
   but we consider using ``SRV`` records in the future.

   Each address may be resolved to a multiple IPs and each API participate in
   round-robin on it's own (not the whole hostname).

LDAP Authorizer
---------------

TBD
