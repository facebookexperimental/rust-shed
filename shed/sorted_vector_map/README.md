# sorted_vector_map

`sorted_vector_map` is an implementation of an ordered map and set (like
`std::collections::BTreeMap` and `std::collections::BTreeSet`) using a sorted
vector as the backing store.

Sorted vector maps are appropriate when data is frequently loaded from an
ordered source, queried a small number of times, and infrequently modified
through insertion or removal.

Loading from an ordered sequence is _O(n)_ through an optimization to `insert`
that handles in-order insertions specially. Extension of the sequence is also
optimized, where extending a map or set of size n with m elements in a single
operation is _O(n + m log m)_. Otherwise, loading from an unordered sequence is
_O(n^2)_.

Look-up is _O(log n)_ through binary search. Insertion and removal are both
_O(n)_, as are set operations like intersection, union and difference.

`sorted_vector_map` is part of
[rust-shed](https://github.com/facebookexperimental/rust-shed). See the
rust-shed repository for more documentation, including the contributing guide.

## License

sorted_vector_map is both MIT and Apache License, Version 2.0 licensed, as found
in the
[LICENSE-MIT](https://github.com/facebookexperimental/rust-shed/blob/master/LICENSE-MIT)
and
[LICENSE-APACHE](https://github.com/facebookexperimental/rust-shed/blob/master/LICENSE-APACHE)
files.
