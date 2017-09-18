==========================
Swindon Changes By Version
==========================

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
* User `online status tracking`_ is implemented in Swindon-lattice_ Protocol
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
