Shortens the vector, keeping the first `len` elements and dropping
the rest.

If `len` is greater than the vector's current length, this has no
effect.

The [`drain`] method can emulate `truncate`, but causes the excess
elements to be returned instead of dropped.

Note that this method has no effect on the allocated capacity
of the vector.