.. _lattice-definition:

========
Lattices
========

Lattice [1]_ has single one function, that is: **subscribe to a family of
values, receiving updates from server**. For example:

    We subscribe for the rooms of user X. If new message arrives in any
    of subscribed chat room the update is propagated to the user. If user
    joins another room the update is delivered to every device connected on
    behalf of the user.


.. [1] We invented this term to describe this specific protocol. Any clashes
   with the term in other contexts are coincidental.


What Does Client See?
=====================

.. _lattice-chat-example:

Every lattice is a dictionary of dictionaries. For example this is how
chat rooms might be delivered to the javascript client:

.. code-block:: json

    ["lattice", {"namespace": "chat.rooms"}, {
        "room1": {
            "last_message_counter": 123,
            "last_seen_counter": 120},
        "room2": {
            "last_message_counter": 245,
            "last_seen_counter": 245}
    }]

Here we see that there is single message unread by user in ``room1`` (that is
the difference between ``last_message_counter`` and ``last_seen_counter``), and
there are no unread messages in ``room2``.

Every update can contain arbitrary number or keys on both levels, and client
must be able to aggregate them correctly. Aggregation of the messages works
only two levels deep and depends on type of the value. Value type is marked
by the key suffix.

Suffix ``_counter`` is used for type that can only grow. This means if update
that was just received contains smaller value we drop it and use old value.
This works on **key by key basis**.

This kind of structures and how to summarize them correctly are called
CRDT: Commutative Replicated Data Structures, and are described in
theri own section.
