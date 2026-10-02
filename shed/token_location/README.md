# token_location

Source locations for parsed nodes, as half-open byte ranges into the source text.

* `SourceLocation` is a `[start, end)` byte range, e.g. for slicing the source with `&source[loc.range()]`.
* `Loc<T>` pairs a node with its location, and `IntoLoc::at` attaches one: `"foo".at(0..3)`.
* `Located` exposes the location of any node that carries one.

Enable the `serde` feature for `Serialize` & `Deserialize` implementations.

Line & column numbers aren't tracked; derive them from the source text when needed, e.g. with a line index.
