# winnow-token-stream

A [`winnow`](https://crates.io/crates/winnow) stream of tokens, lexed on demand from a string by a
tokenizer function, so parsers can be written against tokens rather than characters & whitespace.

```rust,ignore
// Lexes the next token, skipping any whitespace & comments before it, or returns `None` at the end.
fn tokenize<'i>(input: &mut LocatingSlice<&'i str>) -> Option<Loc<MyToken<'i>>> { ... }

let mut input = TokenStream::new(source, tokenize);
let expr = expression.parse_next(&mut input)?;
```

See the [crate docs](https://docs.rs/winnow-token-stream) for a complete example.

Malformed input should be lexed into an error token of the language rather than failing, so the
parser can report it where it's encountered & recover past it.

`.with_location()` wraps a parser's output in a [`token_location`](https://crates.io/crates/token_location)
`Loc`, spanning the tokens it consumed. `token_location` & `winnow` are re-exported as `winnow_token_stream::token_location` & `winnow_token_stream::winnow`,
so there's no need to depend on matching versions of them separately. A few combinators cover common token-level patterns, e.g.
delimited lists with error recovery (`list_of`, `try_unlocated_list_of`) & skipping ahead (`skip_to`,
`yank_to`).

## License

winnow-token-stream is both MIT and Apache License, Version 2.0 licensed, as found in the
[LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE) files.
