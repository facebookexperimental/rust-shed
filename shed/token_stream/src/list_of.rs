/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use token_location::Loc;
use token_location::SourceLocation;
use winnow::Parser;
use winnow::combinator::delimited;
use winnow::combinator::trace;
use winnow::error::ErrMode;
use winnow::error::ParserError;
use winnow::stream::Accumulate;
use winnow::stream::ContainsToken;
use winnow::stream::Stream;

use crate::SourceLocationParser;
use crate::TokenStream;
use crate::skip_to;

/// Parses a delimited list of items with an optional trailing separator, e.g. `[1, 2, 3,]`.
///
/// Returns the items alongside whether the list ended with a separator, located over the whole
/// list including its delimiters. To locate each item as well, pass `item.with_location()`.
///
/// Parsing stops at the first item or separator that backtracks; wrap the parsers in
/// [`cut_err`](winnow::combinator::cut_err) to fail the whole list instead.
#[inline]
pub fn list_of<
    'i,
    Token,
    Output,
    Accumulator,
    Error,
    ItemParser,
    StartDelimParser,
    StartDelimOutput,
    SepParser,
    SepOutput,
    EndDelimParser,
    EndDelimOutput,
>(
    start_delim: StartDelimParser,
    mut item: ItemParser,
    mut sep: SepParser,
    end_delim: EndDelimParser,
) -> impl Parser<TokenStream<'i, Token>, Loc<(Accumulator, bool)>, ErrMode<Error>>
where
    Token: std::fmt::Debug,
    Error: ParserError<TokenStream<'i, Token>> + 'i,
    Accumulator: Accumulate<Output> + 'i,
    ItemParser: Parser<TokenStream<'i, Token>, Output, ErrMode<Error>>,
    StartDelimParser: Parser<TokenStream<'i, Token>, StartDelimOutput, ErrMode<Error>>,
    SepParser: Parser<TokenStream<'i, Token>, SepOutput, ErrMode<Error>>,
    EndDelimParser: Parser<TokenStream<'i, Token>, EndDelimOutput, ErrMode<Error>>,
{
    let body = move |input: &mut TokenStream<'i, Token>| list_body(&mut item, &mut sep, input);
    trace(
        "list_of",
        delimited(start_delim, body, end_delim).with_location(),
    )
}

/// Like [`list_of`], but recovers from items that fail to parse with a cut error.
///
/// The recovery skips ahead to the next separator or closing delimiter, then stands in for the
/// broken item with `make_invalid(error, location)`, where `location` spans the skipped tokens.
/// This keeps malformed items from losing the rest of the list, e.g. for partial ASTs.
#[inline]
pub fn try_list_of<
    'i,
    Token,
    Output,
    Accumulator,
    Error,
    ItemParser,
    StartDelimParser,
    StartDelimOutput,
    SepToken,
    SepOutput,
    EndDelimToken,
    EndDelimOutput,
    MakeInvalid,
>(
    start_delim: StartDelimParser,
    mut item: ItemParser,
    mut sep: SepToken,
    end_delim: EndDelimToken,
    make_invalid: MakeInvalid,
) -> impl Parser<TokenStream<'i, Token>, Loc<(Accumulator, bool)>, ErrMode<Error>>
where
    Token: std::fmt::Debug + Clone,
    Error: ParserError<TokenStream<'i, Token>> + 'i,
    Accumulator: Accumulate<Output> + 'i,
    ItemParser: Parser<TokenStream<'i, Token>, Output, ErrMode<Error>>,
    StartDelimParser: Parser<TokenStream<'i, Token>, StartDelimOutput, ErrMode<Error>>,
    SepToken:
        ContainsToken<Token> + Parser<TokenStream<'i, Token>, SepOutput, ErrMode<Error>> + Copy,
    EndDelimToken: ContainsToken<Token>
        + Parser<TokenStream<'i, Token>, EndDelimOutput, ErrMode<Error>>
        + Copy,
    MakeInvalid: Fn(Error, SourceLocation) -> Output,
{
    let body = move |input: &mut TokenStream<'i, Token>| {
        try_list_body(&mut item, &mut sep, end_delim, &make_invalid, input)
    };
    trace(
        "try_list_of",
        delimited(start_delim, body, end_delim).with_location(),
    )
}

/// The items of a [`try_list_of`] list, without its delimiters, for parsers that consume the
/// delimiters themselves.
///
/// The closing delimiter is only peeked at, to know where the list ends.
#[inline]
pub fn try_list_body<
    'i,
    Token,
    Output,
    Accumulator,
    SepOutput,
    Error,
    ItemParser,
    SepToken,
    EndDelimToken,
    EndDelimOutput,
    MakeInvalid,
>(
    item: &mut ItemParser,
    sep: &mut SepToken,
    mut end_delim: EndDelimToken,
    make_invalid: &MakeInvalid,
    input: &mut TokenStream<'i, Token>,
) -> Result<(Accumulator, bool), ErrMode<Error>>
where
    Error: ParserError<TokenStream<'i, Token>>,
    Token: std::fmt::Debug + Clone,
    Accumulator: Accumulate<Output> + 'i,
    ItemParser: Parser<TokenStream<'i, Token>, Output, ErrMode<Error>>,
    SepToken:
        ContainsToken<Token> + Parser<TokenStream<'i, Token>, SepOutput, ErrMode<Error>> + Copy,
    EndDelimToken: ContainsToken<Token>
        + Parser<TokenStream<'i, Token>, EndDelimOutput, ErrMode<Error>>
        + Copy,
    MakeInvalid: Fn(Error, SourceLocation) -> Output,
{
    let mut acc = Accumulator::initial(None);
    let mut has_trailing_sep = false;

    loop {
        let checkpoint = input.checkpoint();

        // An empty list, or one with a trailing separator
        if end_delim.parse_next(input).is_ok() {
            input.reset(&checkpoint);
            break;
        }
        input.reset(&checkpoint);

        let item_start = input.next_token_start();
        let len = input.eof_offset();

        match item.parse_next(input) {
            Ok(output) => {
                if input.eof_offset() == len {
                    return Err(ParserError::assert(
                        input,
                        "`try_list_body` parser must always consume",
                    ));
                }
                acc.accumulate(output);
                has_trailing_sep = false;

                let sep_checkpoint = input.checkpoint();
                match sep.parse_next(input) {
                    Err(e) if e.is_backtrack() => {
                        input.reset(&sep_checkpoint);
                        break;
                    }
                    Err(e) => return Err(e),
                    Ok(_) => {
                        has_trailing_sep = true;
                    }
                }
            }
            Err(ErrMode::Backtrack(_)) => {
                input.reset(&checkpoint);
                break;
            }
            Err(ErrMode::Cut(err)) => {
                skip_to(input, (*sep, end_delim));
                let item_end = input.prev_token_end().max(item_start);
                acc.accumulate(make_invalid(err, SourceLocation::new(item_start, item_end)));

                let sep_checkpoint = input.checkpoint();
                has_trailing_sep = sep.parse_next(input).is_ok();
                if !has_trailing_sep {
                    // Hit the closing delimiter or the end of the input
                    input.reset(&sep_checkpoint);
                    break;
                }
            }
            Err(ErrMode::Incomplete(_)) => {
                break;
            }
        }
    }

    Ok((acc, has_trailing_sep))
}

/// Like [`winnow::combinator::separated`], but allowing a trailing separator & reporting whether
/// there was one.
fn list_body<'i, Token, Output, Accumulator, SepOutput, Error, ItemParser, SepParser>(
    item: &mut ItemParser,
    sep: &mut SepParser,
    input: &mut TokenStream<'i, Token>,
) -> Result<(Accumulator, bool), ErrMode<Error>>
where
    Error: ParserError<TokenStream<'i, Token>>,
    Token: std::fmt::Debug,
    Accumulator: Accumulate<Output> + 'i,
    ItemParser: Parser<TokenStream<'i, Token>, Output, ErrMode<Error>>,
    SepParser: Parser<TokenStream<'i, Token>, SepOutput, ErrMode<Error>>,
{
    let mut acc = Accumulator::initial(None);
    let mut has_trailing_sep = false;

    loop {
        let start = input.checkpoint();
        let len = input.eof_offset();
        let output = match item.parse_next(input) {
            Err(e) if e.is_backtrack() => {
                input.reset(&start);
                break;
            }
            Err(e) => return Err(e),
            Ok(output) => {
                if input.eof_offset() == len {
                    return Err(ParserError::assert(
                        input,
                        "`list_of` parser must always consume",
                    ));
                }
                output
            }
        };

        acc.accumulate(output);
        has_trailing_sep = false;

        let start = input.checkpoint();
        match sep.parse_next(input) {
            Err(e) if e.is_backtrack() => {
                input.reset(&start);
                break;
            }
            Err(e) => return Err(e),
            Ok(_) => {
                has_trailing_sep = true;
            }
        };
    }

    Ok((acc, has_trailing_sep))
}

#[cfg(test)]
mod tests {
    use token_location::IntoLoc;
    use token_location::SourceLocation;
    use winnow::LocatingSlice;
    use winnow::ascii::multispace0;
    use winnow::combinator::alt;
    use winnow::stream::Location as _;

    use super::*;

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    enum Token {
        Foo,
        Bar,
        LSquare,
        RSquare,
        Comma,
    }

    fn tokenize(input: &mut LocatingSlice<&str>) -> Option<Loc<Token>> {
        let _ = multispace0::<_, ()>.parse_next(input);
        let start = input.current_token_start();
        let token = alt::<_, _, (), _>((
            ','.value(Token::Comma),
            '['.value(Token::LSquare),
            ']'.value(Token::RSquare),
            "foo".value(Token::Foo),
            "bar".value(Token::Bar),
        ))
        .parse_next(input)
        .ok()?;
        Some(token.at(start..input.current_token_start()))
    }

    type TestInput<'i> = TokenStream<'i, Token>;

    /// Allows parsers matching on [`Token`] during parsing
    impl<'i, E> winnow::Parser<TestInput<'i>, Token, E> for Token
    where
        E: ParserError<TestInput<'i>>,
    {
        fn parse_next(&mut self, input: &mut TestInput<'i>) -> winnow::Result<Token, E> {
            let token = input
                .next()
                .ok_or_else(|| ParserError::from_input(input))?
                .node;
            if *self == token {
                Ok(token)
            } else {
                Err(ParserError::from_input(input))
            }
        }
    }

    impl ContainsToken<Token> for Token {
        fn contains_token(&self, token: Token) -> bool {
            *self == token
        }
    }

    fn name<'i>(input: &mut TestInput<'i>) -> Result<Token, ErrMode<()>> {
        alt((Token::Foo, Token::Bar)).parse_next(input)
    }

    #[derive(Debug, PartialEq)]
    enum Item {
        Foo,
        Invalid(SourceLocation),
    }

    /// Only `foo`s are valid items, & a `bar` is a malformed one
    fn item<'i>(input: &mut TestInput<'i>) -> Result<Item, ErrMode<()>> {
        match input.next().map(|token| token.node) {
            Some(Token::Foo) => Ok(Item::Foo),
            Some(Token::Bar) => Err(ErrMode::Cut(())),
            _ => Err(ErrMode::Backtrack(())),
        }
    }

    fn items(raw_input: &str) -> (Vec<Item>, bool) {
        let mut input = TestInput::new(raw_input, tokenize);
        try_list_of(
            Token::LSquare,
            item,
            Token::Comma,
            Token::RSquare,
            |(), location| Item::Invalid(location),
        )
        .parse_next(&mut input)
        .unwrap()
        .node
    }

    #[test]
    fn csv() {
        let raw_input = " [ foo, bar, foo, bar ] ";
        let mut input = TestInput::new(raw_input, tokenize);
        let loc = list_of(
            Token::LSquare,
            name.with_location(),
            Token::Comma,
            Token::RSquare,
        )
        .parse_next(&mut input)
        .unwrap();

        assert_eq!(loc.location, SourceLocation::new(1, 23));
        assert_eq!(&raw_input[1..2], "[");
        assert_eq!(&raw_input[22..23], "]");

        let (list, has_trailing_sep): (Vec<_>, bool) = loc.node;
        assert_eq!(
            list,
            vec![
                Token::Foo.at(3..6),
                Token::Bar.at(8..11),
                Token::Foo.at(13..16),
                Token::Bar.at(18..21)
            ]
        );
        assert!(!has_trailing_sep);
        assert_eq!(&raw_input[3..6], "foo");
        assert_eq!(&raw_input[8..11], "bar");
        assert_eq!(&raw_input[13..16], "foo");
        assert_eq!(&raw_input[18..21], "bar");
    }

    #[test]
    fn trailing_separator_and_empty_lists() {
        let list = |raw_input| {
            let mut input = TestInput::new(raw_input, tokenize);
            let (names, trailing): (Vec<Token>, bool) =
                list_of(Token::LSquare, name, Token::Comma, Token::RSquare)
                    .parse_next(&mut input)
                    .unwrap()
                    .node;
            (names, trailing)
        };
        assert_eq!(list("[foo, bar,]"), (vec![Token::Foo, Token::Bar], true));
        assert_eq!(list("[]"), (vec![], false));
    }

    #[test]
    fn recovers_from_malformed_items() {
        assert_eq!(
            items("[foo, bar bar, foo]"),
            (
                vec![Item::Foo, Item::Invalid((6..13).into()), Item::Foo],
                false
            ),
            "the invalid item spans everything up to the next separator"
        );
        assert_eq!(
            items("[foo, bar]"),
            (vec![Item::Foo, Item::Invalid((6..9).into())], false),
            "recovery stops at the closing delimiter"
        );
    }
}
