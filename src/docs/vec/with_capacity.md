Constructs a new empty vector with at least the specified capacity
with the provided `BumpScope`.

The vector will be able to hold `capacity` elements without
reallocating. If `capacity` is 0, the vector will not allocate.

It is important to note that although the returned vector has the
minimum *capacity* specified, the vector will have a zero *length*. For
an explanation of the difference between length and capacity, see
*[Capacity and reallocation]*.

When `T` is a zero-sized type, there will be no allocation
and the capacity will always be `usize::MAX`.

[Capacity and reallocation]: alloc::vec::Vec#capacity-and-reallocation