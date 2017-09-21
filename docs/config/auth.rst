Auth & Auth
===========

Swindon currently supports authorization based on source ip address.
Ldap authorization is in the works.


Authorization Table
===================

.. versionchanged:: v0.7.0

Authorizer are flagged in ``routing`` table as ``@authorizer``.

.. code-block:: yaml

    routing:
      corporate.example.com: site @corporate-network
      corporate.example.com/admin: admin @super-admins

Everything is allowed by default (because it's a web server, it's here
to publish things). But you can override it by setting ``default`` authorizer,
which applies implicitly.

Note that unlike handlers, authorizers are inherited across paths and
subdomains unless overrriden:

.. code-block:: yaml

    routing:
        *.example.org: site @auth1
        example.org: main-site
        example.org/admin: admin @admin
        example.org/admin/something: something
        example.org/otherthing: otherthing

Is equivalent to:

.. code-block:: yaml

    routing:
        *.example.org: site @auth1
        example.org: main-site @auth1
        example.org/admin: admin @admin
        example.org/admin/something: something @admin
        example.org/otherthing: otherthing @auth1

Also note that exact domain is more specific star domain:

    routing:
        *.example.org: handler
        example.org: handler @auth

Here the ``auth`` is not applied to ``somethign.example.org``, but in this
case:

    routing:
        *.example.org: handler @auth

The authorization (as well as handler) is applied both for the main site
``example.org`` and all the subdomains.


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

Note by default there is a hidden ``default`` authorizer:


.. code-block:: yaml

    authorizers:
      default: !AllowAll

You can override it and it will be used for anything having no authorizer:

.. code-block:: yaml

    authorizers:
      default: !SourceIp
        allowed-network: localhost


AllowAll Authorizer
====================

This authorizer allows everybody access the page. It's here to be used
as default one, but maybe specified explicitly if default is overriden or
just for convenience.

.. index:: pair: !AllowAll; Authorizers

.. code-block:: yaml

     public-data: !AllowAll


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
