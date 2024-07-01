Constructs a new `Bump` with a size hint for the first chunk.

If you want to ensure a specific capacity use the <code>[with_capacity](Bump::with_capacity)\([_in](Bump::with_capacity_in)\)</code> constructor.

The actual size that will be requested from the base allocator may be bigger or smaller.
(A small fixed amount will be subtracted to make it friendlier towards its base allocator that may store its own header information along with it.)