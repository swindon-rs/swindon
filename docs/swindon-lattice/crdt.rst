.. _crdt-types:

==========
CRDT Types
==========

Data types that are called CRDT which is either Conflict-free Replicated Data Type or Commutative Replicated Data Types are used in
:ref:`lattices <lattice-definition>`.

Available types:

counter
   ever-increasing counter

   .. code-block:: javascript

      // some initial value
      {"user_visits_counter": 1}
      // next value
      {"user_visits_counter": 2}

      // on update this value will be ignored
      {"user_visits_counter": 1}

set
    set of some elements, the set can only grow (new elements added) and
    elements can never be deleted from the set

   .. code-block:: javascript

      {"last_message_set": [1, 12, 13465]}

      // on update -- will have no effect
      {"last_message_set": [1, 12]}
