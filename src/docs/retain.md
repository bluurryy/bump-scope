Retains only the elements specified by the predicate, passing a mutable reference to it.

In other words, remove all elements `e` such that `f(&mut e)` returns `false`.
This method operates in place, visiting each element exactly once in the
original order, and preserves the order of the retained elements.