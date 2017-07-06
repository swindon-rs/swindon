============
Installation
============

.. contents::


Using Cargo
===========

You can install *swindon* using cargo::

    cargo install swindon

But we also provide binaries for ubuntu and vagga_ configs.


Example Config
==============

To run swindon you need some config here is the minimal one serving static
from ``public`` folder at port 8080:


.. literalinclude:: ./minimal.yaml
   :language: yaml


Ubuntu Installation
===================

Installation for ubuntu xenial::

    echo 'deb [trusted=yes] http://repo.mglawica.org/ubuntu/ xenial swindon' | sudo tee /etc/apt/sources.list.d/swindon.list
    apt-get update
    apt-get install swindon


More repositories::

    deb [trusted=yes] http://repo.mglawica.org/ubuntu/ xenial swindon
    deb [trusted=yes] http://repo.mglawica.org/ubuntu/ xenial swindon-testing
    deb [trusted=yes] http://repo.mglawica.org/ubuntu/ precise swindon
    deb [trusted=yes] http://repo.mglawica.org/ubuntu/ precise swindon-testing
    deb [trusted=yes] http://repo.mglawica.org/ubuntu/ trusty swindon
    deb [trusted=yes] http://repo.mglawica.org/ubuntu/ trusty swindon-testing


Vagga Installation
==================

Same as above, but in form of vagga config::

    containers:
      serve:
        setup:
        - !Ubuntu xenial
        - !UbuntuRepo
          url: https://repo.mglawica.org/ubuntu/
          suite: xenial
          components: [swindon]
          trusted: true
        - !Install [swindon@0.5.6]

    commands:
      swindon: !Command
        container: swindon
        run:
        - swindon
        - --verbose
        - --config=config/swindon-local.yaml

.. _vagga: http://vagga.readthedocs.org
