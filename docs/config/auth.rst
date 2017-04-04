Auth & Auth
===========

Swindon currently supports authorization based on source ip address.
Ldap authorization is in the works.


Authorization Table
===================

We have another routing table for authorization, which is similar to and
works exactly like ``routing`` section, but it covers only authorization.
It's expected that this table is more coarse-grained in most cases, but
it's not mandatory. Here is an example:

.. code-block:: yaml

    authorization:
      corporate.example.com: corporate-network
      corporate.example.com/admin: super-admins

Note: everything is allowed by default (because it's a web server, it's here
to publish things).

The routing here is the same as in normal routing table, in particular:
``corporate-network`` limit is not obeyed on ``corporate.example.com/admin``.


Authorizers
===========

Authorizers section contains named authorizers. For example:

.. code-block:: yaml

   authorizers:
     corporate-network: !SourceIp
        allowed-network: corporate-ip-group
        forwarded-ip-header: X-Remote-Ip
        accept-forwarded-headers-from: frontend-servers


Source Ip Authorizer
====================

.. index:: pair: !SourceIp; Authorizers

Source IP authorizer looks like this:

.. code-block:: yaml

     corporate-network: !SourceIp
        allowed-network: corporate-ip-group
        forwarded-ip-header: X-Remote-Ip
        accept-forwarded-headers-from: frontend-servers


Settings:

.. opt:: allowed-network

   (required) Name of the network to allow access from. The network is got
   from ``networks`` section.

.. opt:: accept-forwarded-headers-from

   (optional) Sometimes clients do not connect to this instance of swindon
   directly but are proxied from another instance. This means that real IP
   address where swindon receives a connection from is upstream server rather
   than real client. In this case, real client IP address is transferred in
   header specified by ``forwarded-ip-header``.

   To prevent faking the IP address we accept this header only from allowed
   networks specified in this setting.

.. opt:: forwarded-ip-header

   (optional) Name of the header where to read IP address from if the source
   address is within the ``accept-forwarded-headers-from`` network.
