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


