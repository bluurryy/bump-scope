Removes and returns the element at position `index` within the vector,
shifting all elements after it to the left.

Note: Because this shifts over the remaining elements, it has a
worst-case performance of *O*(*n*). If you don't need the order of elements
to be preserved, use [`swap_remove`] instead.

# Panics
Panics if `index` is out of bounds.

[`swap_remove`]: Self::swap_remove