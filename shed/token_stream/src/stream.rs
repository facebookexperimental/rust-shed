/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::fmt::Debug;

use token_location::Loc;
use winnow::LocatingSlice;
use winnow::error::Needed;
use winnow::stream::Location as _;
use winnow::stream::Offset;
use winnow::stream::Stream;
use winnow::stream::StreamIsPartial;

/// Lexes the next token from `input`, skipping any whitespace & comments before it, or returns
/// `None` once only those remain.
///
/// The token's location must be the byte range it was lexed from, as reported by `input`'s
/// [`Location`](winnow::stream::Location): within the input consumed by this call. [`TokenStream`]
/// relies on it to report positions & to slice the source text between tokens.
///
/// Lexing shouldn't fail: input that doesn't form a valid token should become an error token of
/// the language instead, so the parser can report it where it's encountered & recover past it.
///
/// Tokenizers are plain functions, so that [`TokenStream`]'s type doesn't depend on its tokenizer,
/// but that means they can't capture state. Lexing that depends on runtime configuration, e.g. a
/// set of keywords, should leave that distinction to the parser or read the configuration from a
/// `static`.
pub type Tokenizer<'i, Token> = fn(&mut LocatingSlice<&'i str>) -> Option<Loc<Token>>;

/// A [`winnow::stream::Stream`] of the tokens a [`Tokenizer`] lexes from a string on demand.
pub struct TokenStream<'i, Token> {
    input: LocatingSlice<&'i str>,
    tokenize: Tokenizer<'i, Token>,
}

impl<Token> Clone for TokenStream<'_, Token> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Token> Copy for TokenStream<'_, Token> {}

impl<Token> Debug for TokenStream<'_, Token> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenStream")
            .field("input", &self.input)
            .finish()
    }
}

impl<'i, Token> TokenStream<'i, Token> {
    /// Creates a stream of the tokens `tokenize` lexes from `input`.
    pub fn new(input: &'i str, tokenize: Tokenizer<'i, Token>) -> Self {
        Self {
            input: LocatingSlice::new(input),
            tokenize,
        }
    }

    /// Returns the next token without consuming it
    #[inline(always)]
    pub fn peek(&self) -> Option<Loc<Token>> {
        let mut stream = *self;
        stream.next()
    }

    /// Returns true once there are no more tokens
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.peek().is_none()
    }

    /// The offset just past the last consumed token
    #[inline(always)]
    pub fn prev_token_end(&self) -> usize {
        self.input.current_token_start()
    }

    /// The offset of the next token, or [`Self::prev_token_end`] if there are no more tokens
    #[inline(always)]
    pub fn next_token_start(&self) -> usize {
        self.peek()
            .map_or_else(|| self.prev_token_end(), |token| token.location.start)
    }

    /// The source text following the last consumed token
    #[inline(always)]
    pub fn remaining(&self) -> &'i str {
        *self.input
    }

    /// The source text between the last consumed token & the next one, e.g. whitespace & comments
    pub fn trivia_before_next(&self) -> &'i str {
        &self.remaining()[..self.next_token_start() - self.prev_token_end()]
    }
}

impl<Token> Iterator for TokenStream<'_, Token> {
    type Item = Loc<Token>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let mut input = self.input;
        let token = (self.tokenize)(&mut input)?;
        debug_assert!(
            self.prev_token_end() <= token.location.start
                && token.location.start <= token.location.end
                && token.location.end <= input.current_token_start(),
            "the tokenizer located {} outside of the input it consumed",
            token.location,
        );
        self.input = input;
        Some(token)
    }
}

/// An iterator over the tokens of a [`TokenStream`], alongside their offset from where it started
pub struct TokenOffsets<'i, Token> {
    stream: TokenStream<'i, Token>,
    start: usize,
}

impl<Token> Iterator for TokenOffsets<'_, Token> {
    type Item = (usize, Token);

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.stream.next()?;
        Some((token.location.start - self.start, token.node))
    }
}

type InputCheckpoint<'i> = <LocatingSlice<&'i str> as Stream>::Checkpoint;

impl<'i, Token> Offset<InputCheckpoint<'i>> for TokenStream<'i, Token> {
    fn offset_from(&self, start: &InputCheckpoint<'i>) -> usize {
        self.input.offset_from(start)
    }
}

/// Offsets & slices are measured in bytes of the underlying input, like the checkpoints
impl<'i, Token: Debug> Stream for TokenStream<'i, Token> {
    type Token = Token;

    // TODO: a lazy lookahead buffer (or lexing everything up front) would avoid allocating slices
    type Slice = Vec<Token>;

    type IterOffsets = TokenOffsets<'i, Token>;

    type Checkpoint = InputCheckpoint<'i>;

    #[inline(always)]
    fn iter_offsets(&self) -> Self::IterOffsets {
        TokenOffsets {
            stream: *self,
            start: self.prev_token_end(),
        }
    }

    #[inline(always)]
    fn eof_offset(&self) -> usize {
        self.input.eof_offset()
    }

    #[inline(always)]
    fn next_token(&mut self) -> Option<Self::Token> {
        self.next().map(|token| token.node)
    }

    #[inline(always)]
    fn peek_token(&self) -> Option<Self::Token> {
        self.peek().map(|token| token.node)
    }

    fn offset_for<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Token) -> bool,
    {
        self.iter_offsets()
            .find_map(|(offset, token)| predicate(token).then_some(offset))
    }

    fn offset_at(&self, tokens: usize) -> Result<usize, Needed> {
        let mut stream = *self;
        for _ in 0..tokens {
            stream.next().ok_or(Needed::Unknown)?;
        }
        Ok(stream.input.offset_from(&self.input))
    }

    /// Consumes the tokens ending within `offset` bytes
    fn next_slice(&mut self, offset: usize) -> Self::Slice {
        let start = self.input;
        let mut slice = Vec::new();
        loop {
            let mut peek = *self;
            match peek.next() {
                Some(token) if peek.input.offset_from(&start) <= offset => {
                    *self = peek;
                    slice.push(token.node);
                }
                _ => return slice,
            }
        }
    }

    fn peek_slice(&self, offset: usize) -> Self::Slice {
        let mut stream = *self;
        stream.next_slice(offset)
    }

    #[inline(always)]
    fn checkpoint(&self) -> Self::Checkpoint {
        self.input.checkpoint()
    }

    #[inline(always)]
    fn reset(&mut self, checkpoint: &Self::Checkpoint) {
        self.input.reset(checkpoint);
    }

    fn raw(&self) -> &dyn Debug {
        self
    }
}

impl<'i, Token> StreamIsPartial for TokenStream<'i, Token> {
    type PartialState = <&'i str as StreamIsPartial>::PartialState;

    #[inline(always)]
    fn complete(&mut self) -> Self::PartialState {
        self.input.complete()
    }

    #[inline(always)]
    fn restore_partial(&mut self, state: Self::PartialState) {
        self.input.restore_partial(state);
    }

    #[inline(always)]
    fn is_partial_supported() -> bool {
        LocatingSlice::<&str>::is_partial_supported()
    }
}

#[cfg(test)]
mod tests {
    use token_location::IntoLoc;
    use winnow::Parser;
    use winnow::ascii::multispace0;
    use winnow::combinator::alt;
    use winnow::token::any;
    use winnow::token::take;
    use winnow::token::take_till;
    use winnow::token::take_while;

    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Token {
        A,
        B,
        Invalid,
    }

    /// Lexes `a`s & `b`s, skipping whitespace & `-` comments
    fn tokenize(input: &mut LocatingSlice<&str>) -> Option<Loc<Token>> {
        loop {
            let _ = multispace0::<_, ()>.parse_next(input);
            if input.is_empty() {
                return None;
            }
            let start = input.current_token_start();
            let token = alt((
                'a'.value(Some(Token::A)),
                'b'.value(Some(Token::B)),
                ('-', take_till(0.., '\n')).value(None),
            ))
            .parse_next(input)
            .unwrap_or_else(|()| {
                let _ = any::<_, ()>.parse_next(input);
                Some(Token::Invalid)
            });
            if let Some(token) = token {
                return Some(token.at(start..input.current_token_start()));
            }
        }
    }

    #[test]
    fn skips_trivia() {
        let stream = TokenStream::new(" a -comment\n  b ", tokenize);
        assert_eq!(
            stream.collect::<Vec<_>>(),
            vec![Token::A.at(1..2), Token::B.at(14..15)]
        );
    }

    #[test]
    fn keeps_lexing_past_invalid_input() {
        let stream = TokenStream::new("a?b", tokenize);
        assert_eq!(
            stream.collect::<Vec<_>>(),
            vec![
                Token::A.at(0..1),
                Token::Invalid.at(1..2),
                Token::B.at(2..3)
            ]
        );
    }

    #[test]
    fn positions_between_tokens() {
        let mut stream = TokenStream::new("a -comment\n b  ", tokenize);
        assert_eq!(stream.next_token_start(), 0);
        stream.next();
        assert_eq!(stream.prev_token_end(), 1);
        assert_eq!(stream.next_token_start(), 12);
        assert_eq!(stream.trivia_before_next(), " -comment\n ");
        stream.next();
        assert!(stream.is_empty());
        assert_eq!(
            stream.next_token_start(),
            13,
            "trailing trivia isn't part of any token"
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "outside of the input it consumed")]
    fn tokens_must_be_located_where_they_were_lexed() {
        fn misplaced(input: &mut LocatingSlice<&str>) -> Option<Loc<Token>> {
            let _ = any::<_, ()>.parse_next(input).ok()?;
            Some(Token::A.at(5..6))
        }
        let _ = TokenStream::new("ab", misplaced).next();
    }

    #[test]
    fn winnow_slicing_takes_whole_tokens() {
        let mut stream = TokenStream::new("a -comment\n a b a", tokenize);
        let run: Vec<Token> = take_while::<_, _, ()>(0.., |token| token == Token::A)
            .parse_next(&mut stream)
            .unwrap();
        assert_eq!(run, [Token::A, Token::A]);
        let pair: Vec<Token> = take::<_, _, ()>(2usize).parse_next(&mut stream).unwrap();
        assert_eq!(pair, [Token::B, Token::A]);
        assert!(stream.is_empty());
    }

    #[test]
    fn checkpoints_restore_position() {
        let mut stream = TokenStream::new("a b", tokenize);
        let checkpoint = stream.checkpoint();
        stream.next();
        stream.reset(&checkpoint);
        assert_eq!(stream.peek(), Some(Token::A.at(0..1)));
    }
}
