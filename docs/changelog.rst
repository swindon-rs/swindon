==========================
Swindon Changes By Version
==========================


.. _changelog-v0.7.5:

v0.7.5
======

* All ``listen`` settings are now updatable, i.e. swindon will rebind if
  the name resolves to different addresses or when config changes.
* When swindon fails to listen on some address it will retry every second
  instead of waiting for configuration change
* Fixes broken ``swindon-dev`` when proxying is used


.. _changelog-v0.7.4:

v0.7.4
======

* Bugfix: due to a bug in abstract-ns domain names with numbers were not
  resolved properly (in client protocols: replication, http destination...)
* Added a log message on start (to find restarts in log easier)

.. _changelog-v0.7.3:

v0.7.3
======

* Bugfix: TTL on private lattice data introduced in v0.7.2 were sometimes not
  handled properly on auth, effectively leaving users unsubscribed right after
  authorization
* Bugfix: while we started to sync users activity properly in v0.7.2, the
  information from a replica wasn't properly propagated to clients


.. _changelog-v0.7.2:

v0.7.2
======

* ``SwindonLattice`` protocol:
    * Added :ref:`expires_in <expires_in>` experimental
      API
    * Private lattice data is now cleaned in 1 minute if unused, this removes
      a memory leak, but may be a problem if connection authorization works for
      longer that a minute
    * ``active`` user online status is now replicated better
* Previously when swindon couldn't resolve name on startup it could crash
  connectin pool (http-destination) and never fix it again. Now it will retry
  name resolution indefinitely


.. _changelog-v0.7.0:

v0.7.0
======

* **[breaking]** changes to ``SwindonLattice`` protocol:
    * uses ``/swindon/authorize_connection`` and ``/swindon/session_inactive``
      API routes instead of ones prefixed by ``/tangle/`` previously
    * deprecated handler type ``SwindonChat`` is removed, also dropped
      ``allow-empty-subprotocol`` setting (the behavior can still be restored
      by using :opt:`compatibility`)
    * added :opt:`compatibility` setting
    * The lattices ``swindon.*`` are reserved, and can't be subscribed to and
      updated using normal backend lattice API (one ``swindon.user`` is already
      used as desribed above)
    * The ``Authorization`` header has now prefix of ``Swindon+json`` rather
      than ``Tangle``
    * Responses to ``authorize_connection`` and method calls must contain
      ``Content-Type: application/json`` header, as well as requests to
      backend API endpoints that contain JSON data.
* Added :ref:`register CRDT <register-crdt>` type (basically last-write-wins)
* User `online status tracking`_ is implemented in :ref:`Swindon-lattice`
* Added :ref:`mixins` support
* **[breaking]** no ``authorization`` section any more. You can add
  authorizer by adding ``@authorizer-name`` in the normal routing table.
* **[breaking]** there is now ``default`` handler implicitly defined. You
  can override it and it will work everywhere where plain 404 were previosly
  returned
* **[breaking]** there is now ``default`` authorizer. You can override it
  and it will apply for every route whenever not overridden by other
  authorizer
* Added ``AllowAll`` authorizer (just to be explicit)
* [bugfix] Updating only public part of lattice now delivers the changes to
  users
* Swindon now compiles correctly on Windows
* Many enhancements into file serving, in particular:
      * Range requests are supported
      * If-Modified-Since requests are supported
      * If-None-Match requests are supported
      * Only GET (and HEAD) requests serve file now, other methods are rejected
      * Gzip (``.gz``) and Brotli (``.br``) files are now served by default if
        file exists and user agent supports the encoding (including for
        ``!SingleFile`` handler)
      * Directory indexes are now rendered (format of the directory index will
        probably change in future)
      * :ref:`VersionedStatic <versioned-static>` now sets cache control
        headers
      * ``content-type`` is not required for ``!SingleFile`` anymore,
         it's guessed by extension as for ``!Static``
      * **[breaking]** ``text-charset`` is now ``utf-8`` by default
      * **[breaking]** ``charset=`` is now added to ``application/javascript``
        too (in addition to all ``text/*`` as before)
      * Serving devices (special files like ``/dev/null``) returns 403, while
        previously might work
* The dot ``.`` character is allowed in ``user_id``
* Upgraded quire_ configuration library to the one based on the ``serde``
  crate, this should not change anything user-visible, except some tweaks of
  error messages in configs. But can also have some edge cases.

Upgrading:

1. Replace ``SwindonChat`` to ``SwindonLattice``
2. Set :opt:`compatibility` field to desired level
3. Upgrade application to support both versions of APIs (there are no things
   that conflict with each other)
4. Bump :opt:`compatibility`
5. Remove support of the old API

Note: ``/swindon/`` prefix was reserved (so you couldn't call such methods
from frontend) in swindon since ``0.6.0``.

.. _online status tracking: https://github.com/swindon-rs/swindon/issues/51
.. _quire: http://rust-quire.readthedocs.io/en/latest/
