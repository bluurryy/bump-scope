Removes an element from the vector and returns it.

The removed element is replaced by the first element of the vector.

This does not preserve ordering, but is *O*(1).
If you need to preserve the element order, use [`remove`] instead.

# Panics
Panics if `index` is out of bounds.

[`remove`]: Self::remove