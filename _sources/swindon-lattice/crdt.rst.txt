.. _crdt-types:

==========
CRDT Types
==========

Data types that are called CRDT which is either Conflict-free Replicated Data Type or Commutative Replicated Data Types are used in
:ref:`lattices <lattice-definition>`.

Available types:

.. crdt:: counter

   ever-increasing counter

   .. code-block:: javascript

      // some initial value
      {"user_visits_counter": 1}
      // next value
      {"user_visits_counter": 2}

      // on update this value will be ignored
      {"user_visits_counter": 1}

.. crdt:: set

    set of some elements, the set can only grow (new elements added) and
    elements can never be deleted from the set

   .. code-block:: javascript

      {"last_message_set": [1, 12, 13465]}

      // on update -- will have no effect
      {"last_message_set": [1, 12]}


.. _register-crdt:

.. crdt:: register

    a value with a timestamp or a version, which is updated with
    last-write-wins (LWW) semantics. Any valid json might be used as a value.

    .. code-block:: javascript

       {"status_register": [1, {"icon": "busy", "message": "working"}]}
       // next value
       {"status_register": [2, {"icon": "ready_for_chat", "message": "???"}]}

       // on update -- will have no effect
       {"status_register": [1, {"icon": "offline", "message": "disconnected"}]}

       {"another_register": [1503339804.859186, "hello_string"]}

    .. note:: Only non-negative values might be used as a version. Both
       integers and floating point number are okay, but value will be treated
