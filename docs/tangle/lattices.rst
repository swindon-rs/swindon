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
CRDT: Commutative Replicated Data Types, and are described in
`their own section <crdt-types>`_.


What Does Backend Send?
=======================

Backend has a little bit more complex representation of lattices. Data consists
of "private" and "shared".

1. Shared data is the set of keys visible for many (potentially all)
   users. But not all users have access to all the keys.
2. Private keys are only visible to specific user.

Example
-------

`Example chat <lattice-chat-example>`_ from the backend point of view looks
like:

.. code-block:: json

    {"shared": {
        "room1": {"last_message_counter": 123},
        "room2": {"last_message_counter": 245}},
     "private": {
       "7777": {
        "room1": {"last_seen_counter": 120},
        "room2": {"last_seen_counter": 245}},
       "8734": {
        "room1": {"last_seen_counter": 123},
        "room2": {"last_seen_counter": 24}}
    }}

As you might see every user in the "private" section has it's own key. In the
example above you have seen how these dicts are merged for a specific user
"7777".

You can send only changed keys from the backend. For example if user "7777"
sent a message to a "room2", a backend can send the following message:

.. code-block:: json

    {"shared": {
        "room2": {"last_message_counter": 246}},
     "private": {
       "7777": {
        "room2": {"last_seen_counter": 246}},
    }}

Note the following:

1. The message text and metadata is delivered in other way. Usually pub-sub
   topic is used. But client can also request the message on demand.
2. We marked message as read in the same transaction

In this case javascript received the following JSON for user "7777":

.. code-block:: json

    ["lattice", {"namespace": "chat.public"},
     {
        "room2": {"last_message_counter": 246,
                  "last_seen_counter": 246}
     }]

And devices working on behalf of "8734" receive something like this:

.. code-block:: json

    ["lattice", {"namespace": "chat.public"},
     {"room2": {"last_message_counter": 246}}]


Why is it so Complex?
=====================

We aim to provide reliable information for users despite that something might
fail during user's session. Here is the list of some issues that we try to
avoid with lattices:

1. Messages from backend can be delayed for arbitrary time, so the order
   backend messages reach swindon (and client) is not guaranteed
2. Websocket can be disconnected at any time. Any single message can be
   lost on disconnect.
3. Messages to or from backend can be lost (backend is down, connection between
   datacenters is lost, ...)
4. And swindon itself (which is an edge/gateway server for users) can die
   so all client will reconnect again.

Lattices try to avoid all these issues and always provide reliable data.
