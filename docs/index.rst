Welcome to Swindon's documentation!
===================================


Swindon is a web server that eventually should develop all the features needed
for a frontend server. But the most powerful feature is a
:ref:`protocol <swindon-lattice>` for handling websockets.

Github_ | Crate_


.. figure:: messages.png
   :alt: a screenshot of a dashboard showing a 122k simultaneous connections
         and 22.6M messages in a day

   *While swindon is quite recent project it handles about 120k simultaneous
   connections and ~20 million messages per day in our setup. The screenshot
   above shows just a random day from our dashboard.*


Contents:

.. toctree::
   :maxdepth: 2

   installation
   config/index
   internals/index
   swindon-lattice/index
   changelog

.. _github: https://github.com/swindon-rs/swindon
.. _crate: https://crates.io/crates/swindon

Indices and tables
==================

* :ref:`genindex`
* :ref:`search`

