/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Wrapper around ascii::AsciiChar and AsciiString to implement quickcheck::Arbitrary.

#![deny(warnings, missing_docs, clippy::all, broken_intra_doc_links)]

use quickcheck::{Arbitrary, Gen};
use rand::{seq::SliceRandom, Rng};
use std::{
    iter::{self, FromIterator},
    ops::Deref,
};

/// [ascii::AsciiString] wrapper that implements [quickcheck::Arbitrary]
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct AsciiString(pub ascii::AsciiString);

impl Deref for AsciiString {
    type Target = ascii::AsciiString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ascii::AsciiString> for AsciiString {
    fn from(ch: ascii::AsciiString) -> Self {
        AsciiString(ch)
    }
}

impl FromIterator<AsciiChar> for AsciiString {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = AsciiChar>,
    {
        AsciiString(
            iter.into_iter()
                .map(|ch| ch.0)
                .collect::<ascii::AsciiString>(),
        )
    }
}

impl Arbitrary for AsciiString {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let size = {
            let s = g.size();
            g.gen_range(0, s)
        };
        iter::repeat(())
            .map(|()| AsciiChar::arbitrary(g))
            .take(size)
            .collect()
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let chars: Vec<AsciiChar> = self.0.chars().map(AsciiChar).collect();
        Box::new(
            chars
                .shrink()
                .map(|x| x.into_iter().collect::<AsciiString>()),
        )
    }
}

/// [ascii::AsciiChar] wrapper that implements [quickcheck::Arbitrary]
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(transparent)]
pub struct AsciiChar(pub ascii::AsciiChar);

impl Deref for AsciiChar {
    type Target = ascii::AsciiChar;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ascii::AsciiChar> for AsciiChar {
    fn from(ch: ascii::AsciiChar) -> Self {
        AsciiChar(ch)
    }
}

impl Arbitrary for AsciiChar {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let mode = g.gen_range(0, 100);
        let ret = match mode {
            0..=14 => {
                // Control characters
                unsafe { ascii::AsciiChar::from_ascii_unchecked(g.gen_range(0u8, 0x1F)) }
            }
            15..=39 => {
                // Characters often used in programming languages
                use ascii::AsciiChar;
                *[
                    AsciiChar::Space,
                    AsciiChar::Tab,
                    AsciiChar::LineFeed,
                    AsciiChar::Tilde,
                    AsciiChar::Grave,
                    AsciiChar::Exclamation,
                    AsciiChar::At,
                    AsciiChar::Hash,
                    AsciiChar::Dollar,
                    AsciiChar::Percent,
                    AsciiChar::Ampersand,
                    AsciiChar::Asterisk,
                    AsciiChar::ParenOpen,
                    AsciiChar::ParenClose,
                    AsciiChar::UnderScore,
                    AsciiChar::Minus,
                    AsciiChar::Equal,
                    AsciiChar::Plus,
                    AsciiChar::BracketOpen,
                    AsciiChar::BracketClose,
                    AsciiChar::CurlyBraceOpen,
                    AsciiChar::CurlyBraceClose,
                    AsciiChar::Colon,
                    AsciiChar::Semicolon,
                    AsciiChar::Apostrophe,
                    AsciiChar::Quotation,
                    AsciiChar::BackSlash,
                    AsciiChar::VerticalBar,
                    AsciiChar::Caret,
                    AsciiChar::Comma,
                    AsciiChar::LessThan,
                    AsciiChar::GreaterThan,
                    AsciiChar::Dot,
                    AsciiChar::Slash,
                    AsciiChar::Question,
                    AsciiChar::_0,
                    AsciiChar::_1,
                    AsciiChar::_2,
                    AsciiChar::_3,
                    AsciiChar::_3,
                    AsciiChar::_4,
                    AsciiChar::_6,
                    AsciiChar::_7,
                    AsciiChar::_8,
                    AsciiChar::_9,
                ]
                .choose(g)
                .unwrap()
            }
            40..=99 => {
                // Completely arbitrary characters
                unsafe { ascii::AsciiChar::from_ascii_unchecked(g.gen_range(0u8, 0x80)) }
            }
            _ => unreachable!(),
        };

        AsciiChar(ret)
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(
            (self.0 as u8)
                .shrink()
                .filter_map(|x| ascii::AsciiChar::from_ascii(x).ok().map(AsciiChar)),
        )
    }
}
