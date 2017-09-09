==========================
Swindon Changes By Version
==========================

.. _changelog-v0.7.0:

v0.7.0
======

* **[breaking]** by default ``SwindonLattice`` now uses
  ``/swindon/authorize_connection`` and ``/swindon/session_inactive`` API
  routes instead of ones prefixed by ``/tangle/`` previously (see upgrade
  options below)
* Swindon now compiles correctly on Windows
* Added :ref:`register CRDT <register-crdt>` type (basically last-write-wins)
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
* User `online status tracking`_ is implemented in Swindon-lattice_ Protocol
* The dot ``.`` character is allowed in ``user_id``
* [bugfix] Updating only public part of lattice now delivers the changes to
  users
* Upgraded quire_ configuration library to the one based on the ``serde``
  crate, this should not change anything user-visible, except some tweaks of
  error messages in configs. But can also have some edge cases.

There are few uptrade paths for swindon-lattice users:

1. Before upgrading swindon start serving same at both ``/swindon/`` and
   ``/tangle/`` prefixes. Then upgrade swindon. *(This options is preferred)*

2. Set ``use-tangle-prefix: true`` in the ``!SwindonLattice`` handler, but be
   aware that flag wasn't present in previous versions, so you will not be
   able to downgrade swindon. Then fix the code and update the flag at will.

3. Downgrade ``!SwindonLattice`` to ``!SwindonChat``. This will have same
   effect as ``use-tangle-prefix: true`` but config stays compatible with
   older swindon.

Note: ``/swindon/`` prefix was reserved (so you couldn't call such methods
from frontend) in swindon since ``0.6.0``.

.. _online status tracking: https://github.com/swindon-rs/swindon/issues/51
.. _quire: http://rust-quire.readthedocs.io/en/latest/
