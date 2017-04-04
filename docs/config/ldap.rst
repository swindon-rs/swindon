LDAP Support
============

**LDAP doesn't work yet, this is provisional documentation**

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

Next thing is to configure an authorizer. The authorizer is a thing that
picks specific rules for accessing the website.

Here is an example of authorizer configuration:

.. code-block:: yaml

   authorizers:
     ldap: !Ldap
       destination: local-ldap
       search-base: dc=users,dc=example,dc=org
       login-attribute: uid
       password-attibute: userPassword
       login-header: X-User-Uid
       additional-queries:
         X-User-Groups:
           search-base: cn=Group,dc=uaprom,dc=org
           fetch-attribute: dn
           filter: "member=${dn}"
           dn-attribute-strip-base: cn=Group,dc=uaprom,dc=org

Options:

.. opt:: destination

   Destination LDAP connection pool name (see :ref:`LDAP Destinations`)

.. opt:: search-base

   Base DN for searching for user

.. opt:: login-attribute

   The attribute that will be matched against when user is logging in.

.. opt:: password-attribute

   The password attribute name for authentication.

.. opt:: login-header

   A header where valid login will be passed when proxying request to a HTTP
   destination (when authentication succeeds).

.. opt:: additional-queries

   Each of this query will be executed for already logged in user and result
   of the query will be passed as the header value to the a HTTP destination.

