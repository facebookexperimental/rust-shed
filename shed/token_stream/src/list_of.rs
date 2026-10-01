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
use crate::with_location;

/// Parses a list of items, separated by [`sep`] & delimited by [`start_delim`] & [`end_delim`],
/// with an optional trailing separator. This is a common pattern for list-like constructs, e.g. list/map literals
/// ```ignore
/// [1, 2, 3]
/// ```
/// Returns the list of items, as well as a boolean indicating whether there was a trailing separator.
/// As the trailing separator is optional, in some scenarios it may influence the behavior of the list,
#[inline(always)]
pub fn list_of<
    'i,
    Token,
    Output,
    Accumulator,
    Error,
    ParseNext,
    StartDelimParser,
    StartDelimParserOutput,
    SepParser,
    SepOutput,
    EndDelimParser,
    EndDelimParserOutput,
>(
    start_delim: StartDelimParser,
    mut parser: ParseNext,
    mut sep: SepParser,
    end_delim: EndDelimParser,
) -> impl Parser<TokenStream<'i, Token>, Loc<(Accumulator, bool)>, ErrMode<Error>>
where
    Token: std::fmt::Debug,
    Error: ParserError<TokenStream<'i, Token>> + 'i,
    Accumulator: Accumulate<Loc<Output>> + 'i,
    ParseNext: Parser<TokenStream<'i, Token>, Output, ErrMode<Error>>,
    StartDelimParser: Parser<TokenStream<'i, Token>, StartDelimParserOutput, ErrMode<Error>>,
    SepParser: Parser<TokenStream<'i, Token>, SepOutput, ErrMode<Error>>,
    EndDelimParser: Parser<TokenStream<'i, Token>, EndDelimParserOutput, ErrMode<Error>>,
{
    let body = move |input: &mut TokenStream<'i, Token>| {
        list_body(&mut parser, &mut sep, with_location, input)
    };
    trace(
        "list_of",
        delimited(start_delim, body, end_delim).with_location(),
    )
}

/// Parses a list of items, separated by [`sep`] & delimited by [`start_delim`] & [`end_delim`],
/// with an optional trailing separator. This is a common pattern for list-like constructs, e.g. list/map literals
/// ```ignore
/// [1, 2, 3]
/// ```
/// Returns the list of items, as well as a boolean indicating whether there was a trailing separator.
/// As the trailing separator is optional, in some scenarios it may influence the behavior of the list,
#[inline(always)]
pub fn unlocated_list_of<
    'i,
    Token,
    Output,
    Accumulator,
    Error,
    ParseNext,
    StartDelimParser,
    StartDelimParserOutput,
    SepParser,
    SepOutput,
    EndDelimParser,
    EndDelimParserOutput,
>(
    start_delim: StartDelimParser,
    mut parser: ParseNext,
    mut sep: SepParser,
    end_delim: EndDelimParser,
) -> impl Parser<TokenStream<'i, Token>, Loc<(Accumulator, bool)>, ErrMode<Error>>
where
    Token: std::fmt::Debug,
    Error: ParserError<TokenStream<'i, Token>> + 'i,
    Accumulator: Accumulate<Output> + 'i,
    ParseNext: Parser<TokenStream<'i, Token>, Output, ErrMode<Error>>,
    StartDelimParser: Parser<TokenStream<'i, Token>, StartDelimParserOutput, ErrMode<Error>>,
    SepParser: Parser<TokenStream<'i, Token>, SepOutput, ErrMode<Error>>,
    EndDelimParser: Parser<TokenStream<'i, Token>, EndDelimParserOutput, ErrMode<Error>>,
{
    let body = move |input: &mut TokenStream<'i, Token>| {
        list_body(&mut parser, &mut sep, passthrough, input)
    };
    trace(
        "list_of",
        delimited(start_delim, body, end_delim).with_location(),
    )
}

fn passthrough<P, I, O, E>(parser: &mut P, input: &mut I) -> Result<O, ErrMode<E>>
where
    P: Parser<I, O, ErrMode<E>>,
{
    parser.parse_next(input)
}

/// Parses a list of items with error recovery support.
///
/// Similar to [`unlocated_list_of`], but when a Cut error occurs during item parsing,
/// instead of propagating the error, this function:
/// 1. Skips to the next separator or closing delimiter
/// 2. Uses the provided `make_invalid` function to create an "invalid" placeholder item
/// 3. Continues parsing subsequent items
///
/// This enables partial AST construction even when some list elements are malformed.
///
/// # Arguments
/// * `start_delim` - Parser for the opening delimiter (e.g., `[`)
/// * `parser` - Parser for individual items
/// * `sep` - Separator token (must implement both `Parser` and `ContainsToken`)
/// * `end_delim` - End delimiter token (must implement both `Parser` and `ContainsToken`)
/// * `make_invalid` - Function to create an invalid item from a Cut error
#[inline(always)]
pub fn try_unlocated_list_of<
    'i,
    Token,
    Output,
    Accumulator,
    Error,
    ParseNext,
    StartDelimParser,
    StartDelimParserOutput,
    SepToken,
    SepOutput,
    EndDelimToken,
    EndDelimOutput,
    MakeInvalid,
>(
    start_delim: StartDelimParser,
    mut parser: ParseNext,
    mut sep: SepToken,
    end_delim: EndDelimToken,
    make_invalid: MakeInvalid,
) -> impl Parser<TokenStream<'i, Token>, Loc<(Accumulator, bool)>, ErrMode<Error>>
where
    Token: std::fmt::Debug + Clone,
    Error: ParserError<TokenStream<'i, Token>> + 'i,
    Accumulator: Accumulate<Output> + 'i,
    ParseNext: Parser<TokenStream<'i, Token>, Output, ErrMode<Error>>,
    StartDelimParser: Parser<TokenStream<'i, Token>, StartDelimParserOutput, ErrMode<Error>>,
    SepToken:
        ContainsToken<Token> + Parser<TokenStream<'i, Token>, SepOutput, ErrMode<Error>> + Copy,
    EndDelimToken: ContainsToken<Token>
        + Parser<TokenStream<'i, Token>, EndDelimOutput, ErrMode<Error>>
        + Copy,
    MakeInvalid: Fn(Error, usize, usize) -> Output,
{
    let body = move |input: &mut TokenStream<'i, Token>| {
        try_list_body(&mut parser, &mut sep, end_delim, &make_invalid, input)
    };
    trace(
        "try_list_of",
        delimited(start_delim, body, end_delim).with_location(),
    )
}

/// Parses a comma-separated list body with error recovery support.
///
/// This is the body-only version of [`try_unlocated_list_of`] - it does not
/// handle opening/closing delimiters. Use this when you need to manually handle
/// delimiters or when working with list parsers that have already consumed the
/// opening delimiter.
///
/// When a Cut error occurs during item parsing, this function:
/// 1. Skips to the next separator or closing delimiter
/// 2. Uses the provided `make_invalid` function to create an "invalid" placeholder item
/// 3. Continues parsing subsequent items
///
/// # Arguments
/// * `parser` - Parser for individual items
/// * `sep` - Separator token (must implement both `Parser` and `ContainsToken`)
/// * `end_delim` - End delimiter token (must implement both `Parser` and `ContainsToken`)
/// * `make_invalid` - Function to create an invalid item from a Cut error (err, start_pos, end_pos)
#[inline(always)]
pub fn try_list_body<
    'i,
    Token,
    Output,
    Acc,
    SepOutput,
    Error,
    ParseNext,
    SepToken,
    EndDelimToken,
    EndDelimOutput,
    MakeInvalid,
>(
    parser: &mut ParseNext,
    sep: &mut SepToken,
    mut end_delim: EndDelimToken,
    make_invalid: &MakeInvalid,
    input: &mut TokenStream<'i, Token>,
) -> Result<(Acc, bool), ErrMode<Error>>
where
    Error: ParserError<TokenStream<'i, Token>>,
    Token: std::fmt::Debug + Clone,
    Acc: Accumulate<Output> + 'i,
    ParseNext: Parser<TokenStream<'i, Token>, Output, ErrMode<Error>>,
    SepToken:
        ContainsToken<Token> + Parser<TokenStream<'i, Token>, SepOutput, ErrMode<Error>> + Copy,
    EndDelimToken: ContainsToken<Token>
        + Parser<TokenStream<'i, Token>, EndDelimOutput, ErrMode<Error>>
        + Copy,
    MakeInvalid: Fn(Error, usize, usize) -> Output,
{
    use crate::skip_to;

    let mut acc: Acc = Acc::initial(None);
    let mut has_trailing_sep = false;

    loop {
        let checkpoint = input.checkpoint();

        // Check for closing delimiter (empty list or trailing comma case)
        if end_delim.parse_next(input).is_ok() {
            input.reset(&checkpoint);
            break;
        }
        input.reset(&checkpoint);

        let start_char = input.next_token_start();
        let len: usize = input.eof_offset();

        match parser.parse_next(input) {
            Ok(item) => {
                if input.eof_offset() == len {
                    return Err(ParserError::assert(
                        input,
                        "`try_list_body` parser must always consume",
                    ));
                }
                acc.accumulate(item);
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
                // Recovery: skip to next separator or closing delimiter
                skip_to(input, (*sep, end_delim));
                let end_char = input.prev_token_end().max(start_char);
                let invalid_item = make_invalid(err, start_char, end_char);
                acc.accumulate(invalid_item);

                // If we hit a separator, consume it and continue
                let sep_checkpoint = input.checkpoint();
                if sep.parse_next(input).is_ok() {
                    has_trailing_sep = true;
                    continue;
                } else {
                    input.reset(&sep_checkpoint);
                    // Hit closing delimiter or end of input
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

/// Similar to the [`separated`](winnow::combinator::separated) combinator, but allows
/// for an optional trailing separator & and allows for comments between elements, e.g.
/// ```ignore
///     1, // Some comment
///     2, // Some other comment
///     /* some leading comment*/ 3, // Some trailing comment
/// ```
/// Note: This stops when either parser returns [`ErrMode::Backtrack`][winnow::error::ErrMode::Backtrack]. To instead chain an error up, see
/// [`cut_err`][winnow::combinator::cut_err].
fn list_body<'i, Token, Output, Acc, Sep, Error, ParseNext, SepParser, WrappedItem, ItemWrapper>(
    parser: &mut ParseNext,
    sep: &mut SepParser,
    item_wrapper: ItemWrapper,
    input: &mut TokenStream<'i, Token>,
) -> Result<(Acc, bool), ErrMode<Error>>
where
    Error: ParserError<TokenStream<'i, Token>>,
    Token: std::fmt::Debug,
    Acc: Accumulate<WrappedItem> + 'i,
    ParseNext: Parser<TokenStream<'i, Token>, Output, ErrMode<Error>>,
    SepParser: Parser<TokenStream<'i, Token>, Sep, ErrMode<Error>>,
    ItemWrapper:
        Fn(&mut ParseNext, &mut TokenStream<'i, Token>) -> Result<WrappedItem, ErrMode<Error>>,
{
    let mut acc: Acc = Acc::initial(None);
    let mut has_trailing_sep = false;

    // Tries to parse sequences of [misc] <element> [misc] [separator] [misc]
    loop {
        let start = input.checkpoint();
        let len: usize = input.eof_offset();
        let item = match item_wrapper(parser, input) {
            Err(e) if e.is_backtrack() => {
                input.reset(&start);
                break;
            }
            Err(e) => return Err(e),
            Ok(o) => {
                if input.eof_offset() == len {
                    return Err(ParserError::assert(
                        input,
                        "`list_of` parser must always consume",
                    ));
                }
                o
            }
        };

        acc.accumulate(item);
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

    fn name<'i>(input: &mut TestInput<'i>) -> Result<Token, ErrMode<()>> {
        alt((Token::Foo, Token::Bar)).parse_next(input)
    }

    #[test]
    fn csv() {
        let raw_input = " [ foo, bar, foo, bar ] ";
        let mut input = TestInput::new(raw_input, tokenize);
        let loc = list_of(Token::LSquare, name, Token::Comma, Token::RSquare)
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
}
