/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Source locations for parsed nodes, as byte ranges into the source text: see [`SourceLocation`]
//! & [`Loc`].
//!
//! ```
//! use token_location::IntoLoc;
//! use token_location::Located;
//! use token_location::SourceLocation;
//!
//! let source = "let answer = 42;";
//! let name = "answer".at(4..10);
//! let value = 42.at(13..15);
//! assert_eq!(&source[name.location.range()], "answer");
//!
//! // Spanning a larger node from its parts
//! let binding = name.location().to(value.location());
//! assert_eq!(binding, SourceLocation::new(4, 15));
//! assert_eq!(&source[binding.range()], "answer = 42");
//! ```

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]

use std::ops::Deref;
use std::ops::DerefMut;
use std::ops::Range;

/// The location of a node within a stream, as a half-open `[start, end)` range of byte offsets.
///
/// `start` is expected not to exceed `end`. Line & column information isn't tracked; derive it from
/// the source text when needed, e.g. with a line index.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SourceLocation {
    /// The offset of the first byte
    pub start: usize,
    /// The offset just past the last byte, or `start` for an empty location
    pub end: usize,
}

impl SourceLocation {
    /// Returns the location of the bytes from `start` up to, but excluding, `end`.
    #[inline(always)]
    pub fn new(start: usize, end: usize) -> Self {
        SourceLocation { start, end }
    }

    /// Returns a zero-width [`SourceLocation`] at `offset`.
    #[inline(always)]
    pub fn point(offset: usize) -> Self {
        Self::new(offset, offset)
    }

    /// Returns a zero-width [`SourceLocation`] at the start of the input.
    #[inline(always)]
    pub fn begin() -> Self {
        Self::point(0)
    }

    /// The number of bytes covered.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Whether no bytes are covered, e.g. for a [`point`](Self::point).
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether the byte at `offset` is covered.
    #[inline(always)]
    pub fn contains(&self, offset: usize) -> bool {
        self.start <= offset && offset < self.end
    }

    /// Whether any byte is covered by both `self` & `other`.
    #[inline(always)]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    /// Returns the range from the start of `self` up to the end of `other`, e.g. to span a node from
    /// its first to its last child.
    #[inline(always)]
    pub fn to(&self, other: SourceLocation) -> Self {
        Self::new(self.start, other.end)
    }

    /// Returns a zero-width [`SourceLocation`] "point" at the end of `self`.
    ///
    /// Useful for anchoring an empty or synthesized node (e.g. an absent field
    /// list) at a single position rather than spanning a range.
    #[inline(always)]
    pub fn end_point(&self) -> SourceLocation {
        Self::point(self.end)
    }

    /// The byte range of `self`, e.g. for slicing the source text.
    #[inline(always)]
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

impl From<Range<usize>> for SourceLocation {
    #[inline(always)]
    fn from(value: Range<usize>) -> Self {
        Self::new(value.start, value.end)
    }
}

impl From<SourceLocation> for Range<usize> {
    #[inline(always)]
    fn from(value: SourceLocation) -> Self {
        value.range()
    }
}

/// A node alongside its location in the source text.
///
/// Derefs to the node, so its fields & methods can be used directly.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Loc<T> {
    /// The located node
    pub node: T,
    /// Where the node is in the source text
    pub location: SourceLocation,
}

impl<T> Loc<T> {
    /// Pairs `node` with its `location`.
    #[inline(always)]
    pub fn new(node: T, location: SourceLocation) -> Self {
        Self { node, location }
    }

    /// Maps the node, keeping its location.
    #[inline(always)]
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Loc<U> {
        Loc {
            node: f(self.node),
            location: self.location,
        }
    }

    /// Maps the node with a fallible function, keeping its location.
    #[inline(always)]
    pub fn try_map<U, E>(self, f: impl FnOnce(T) -> Result<U, E>) -> Result<Loc<U>, E> {
        Ok(Loc {
            node: f(self.node)?,
            location: self.location,
        })
    }
}

impl<T: Deref> Loc<T> {
    /// Borrows the node's target, keeping its location, like [`Option::as_deref`].
    #[inline(always)]
    pub fn as_deref(&self) -> Loc<&T::Target> {
        Loc {
            node: &self.node,
            location: self.location,
        }
    }
}

impl<T> Deref for Loc<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<T> DerefMut for Loc<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.node
    }
}

/// Extension trait to pair any value with a location, converting it into a [`Loc`].
///
/// ```
/// use token_location::IntoLoc;
///
/// let name = "foo".at(0..3);
/// assert_eq!(*name, "foo");
/// assert_eq!(name.location.len(), 3);
/// ```
pub trait IntoLoc: Sized {
    /// Pairs `self` with `location`.
    fn at(self, location: impl Into<SourceLocation>) -> Loc<Self>;

    /// Pairs `self` with an empty location at the start of the input.
    #[inline(always)]
    fn at_begin(self) -> Loc<Self> {
        self.at(SourceLocation::begin())
    }
}

/// Blanket implementation for all types to provide a default implementation of [`IntoLoc`]
impl<T> IntoLoc for T {
    #[inline(always)]
    fn at(self, location: impl Into<SourceLocation>) -> Loc<T> {
        Loc {
            node: self,
            location: location.into(),
        }
    }
}

/// Values that know their location in the source text
pub trait Located {
    /// The location of `self` in the source text.
    fn location(&self) -> SourceLocation;
}

/// Implementation of [`Located`] for SourceLocation itself
impl Located for SourceLocation {
    #[inline(always)]
    fn location(&self) -> SourceLocation {
        *self
    }
}

/// Blanket implementation of [`Located`] for all [`Loc`] wrappers
impl<T> Located for Loc<T> {
    #[inline(always)]
    fn location(&self) -> SourceLocation {
        self.location
    }
}

impl<T: Located> Located for std::rc::Rc<T> {
    #[inline(always)]
    fn location(&self) -> SourceLocation {
        self.deref().location()
    }
}

impl<T: Located> Located for &T {
    #[inline(always)]
    fn location(&self) -> SourceLocation {
        (*self).location()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_is_end_exclusive() {
        let loc = SourceLocation::new(2, 5);
        assert!(!loc.contains(1));
        assert!(loc.contains(2));
        assert!(loc.contains(4));
        assert!(!loc.contains(5), "the end offset is one past the last byte");
    }

    #[test]
    fn overlaps() {
        let loc = SourceLocation::new(2, 5);
        assert!(loc.overlaps(&SourceLocation::new(4, 8)));
        assert!(loc.overlaps(&SourceLocation::new(0, 3)));
        assert!(
            !loc.overlaps(&SourceLocation::new(5, 8)),
            "adjacent ranges share no bytes"
        );
    }
}
