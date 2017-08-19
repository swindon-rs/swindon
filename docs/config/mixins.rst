.. _mixins:

======
Mixins
======

Mixin is another file that contains a number of sections from the configuration
file, but only has items prefixed with specified prefix. For example:

.. code-block:: yaml

   # main.yaml
   routing:
     app1.example.com: app1-main
     app1.example.com/static: app1-static
     app1.example.com/app2: app2-main  # app2 is mounted in a folder too
     app2.example.com: app2-main
     app2.example.com/static: app2-empty

   mixins:
     app1-: app1.yaml
     app2-: app2.yaml

   # app1.yaml

   handlers:
     app1-main: !Proxy
       destination: app1-service
     app1-static: !Static

   http-destination:
     app1-service:
       addresses:
       - localhost:8000

   # app2.yaml

   handlers:
     app2-main: !Static
     app2-empty: !EmptyGif

Note the following things:

1. Swindon ensures that all handlers, http-destinations, and other things
   are prefixed in a file, to avoid mistakes
2. ``routing`` section is not mixed in. You can split it via
    includes_ and merge-tags_ if you want.
3. You can mix and match different handlers in ``routing`` table as well
   as refer to the items accross files. There is no limitation on referencing,
   only on definition of items.

This allows nice splitting and incapsulation in config. Still keeping routing
table short an clean.

Sections supported for mixins:

* :sect:`handlers`
* :sect:`authorizers`
* :sect:`session-pools`
* :sect:`http-destinations`
* :sect:`ldap-destinations`
* :sect:`networks`
* :sect:`log-formats`
* :sect:`disk-pools`

.. _includes: http://rust-quire.readthedocs.io/en/latest/user.html#includes
.. _merge-tags: http://rust-quire.readthedocs.io/en/latest/user.html#merging-mappings
