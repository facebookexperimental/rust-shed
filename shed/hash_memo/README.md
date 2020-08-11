# hash\_memo

`hash_memo` contains implementations of `std::hash::BuildHasher`, `std::hash::Hash`, and `std::hash::Hasher`
that can memoize the `Hasher::finish()` values to save recomputing them.

`hash_memo::EagerHashMemoizer` can wrap your type `T` to eagerly memoize your hash value.  This is ideal
when you know you are immediately going to use it with somthing that will call `Hash::hash()`, for example
you are going to use is as a key in a `HashMap`.

`hash_memo::LazyHashMemoizer` can do similar, but is useful if you are not sure if `Hash::hash()` will be called
and you want to defer the cost.

`hash_memo::BuildMemoHasher` provides a way to construct a wrapper of a `std::hash::Hasher`s so that the memoized
`Hasher::finish()` values are identical to the non-memoized values.  This is useful if you are going to look up
a map by both the wrapped memoized value and via `std::borrow::Borrow::borrow()` to `&T`.

`hash_memo` is part of
[rust-shed](https://github.com/facebookexperimental/rust-shed).  See the rust-shed
repository for more documentation, including the contributing guide.

## License

hash\_memo is both MIT and Apache License, Version 2.0 licensed, as
found in the
[LICENSE-MIT](https://github.com/facebookexperimental/rust-shed/blob/master/LICENSE-MIT)
and
[LICENSE-APACHE](https://github.com/facebookexperimental/rust-shed/blob/master/LICENSE-APACHE)
files.
