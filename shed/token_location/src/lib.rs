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

use std::ops::Deref;
use std::ops::DerefMut;
use std::ops::Range;

/// The location of a node within a stream, as a half-open `[start, end)` range of byte offsets.
///
/// Line & column information isn't tracked; derive it from the source text when needed, e.g. with
/// a line index.
#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize
)]
pub struct SourceLocation {
    pub start: usize,
    pub end: usize,
}

impl SourceLocation {
    #[inline(always)]
    pub fn new(start: usize, end: usize) -> Self {
        SourceLocation { start, end }
    }

    /// Returns a zero-width [`SourceLocation`] at `offset`.
    #[inline(always)]
    pub fn point(offset: usize) -> Self {
        Self::new(offset, offset)
    }

    /// Returns a [`SourceLocation`] at the "beginning" of any input.
    /// This is often used as a sentinel value for generic diagnostics.
    #[inline(always)]
    pub fn begin() -> Self {
        Self::point(0)
    }

    /// Returns a [`SourceLocation`] with invalid offsets.
    /// This can be used when source location isn't available
    #[inline(always)]
    pub fn invalid() -> Self {
        Self::point(usize::MAX)
    }

    pub fn is_invalid(&self) -> bool {
        self.start == usize::MAX && self.end == usize::MAX
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline(always)]
    pub fn contains(&self, offset: usize) -> bool {
        self.start <= offset && offset < self.end
    }

    #[inline(always)]
    pub fn overlaps(&self, other: &Self) -> bool {
        if self.is_invalid() || other.is_invalid() {
            return false;
        }
        self.start < other.end && other.start < self.end
    }

    /// Returns the range from the start of [`self`] up to the end of [`other`]
    #[inline(always)]
    pub fn to(&self, other: SourceLocation) -> Self {
        Self::new(self.start, other.end)
    }

    /// Returns a zero-width [`SourceLocation`] "point" at the end of [`self`].
    ///
    /// Useful for anchoring an empty or synthesized node (e.g. an absent field
    /// list) at a single position rather than spanning a range.
    #[inline(always)]
    pub fn end_point(&self) -> SourceLocation {
        Self::point(self.end)
    }

    /// The byte range of [`self`], e.g. for slicing the source text
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

/// A node with a location
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, serde::Serialize)]
pub struct Loc<T> {
    pub node: T,
    pub location: SourceLocation,
}

impl<T> Loc<T> {
    #[inline(always)]
    pub fn new(node: T, location: SourceLocation) -> Self {
        Self { node, location }
    }

    #[inline(always)]
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Loc<U> {
        Loc {
            node: f(self.node),
            location: self.location,
        }
    }

    #[inline(always)]
    pub fn try_map<U, E>(self, f: impl FnOnce(T) -> Result<U, E>) -> Result<Loc<U>, E> {
        Ok(Loc {
            node: f(self.node)?,
            location: self.location,
        })
    }
}

// Mirror Option::as_deref
impl<T: Deref> Loc<T> {
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

impl<T, U> PartialOrd<Loc<U>> for Loc<T>
where
    Loc<T>: std::cmp::PartialEq<Loc<U>>,
{
    fn partial_cmp(&self, other: &Loc<U>) -> Option<std::cmp::Ordering> {
        self.location.partial_cmp(&other.location)
    }
}

/// Utility trait to annotate a node [`T`] with its location, converting it into a [`Loc<T>`].
/// ```ignore
/// "foo".at(0..3)
/// ```
pub trait IntoLoc<T> {
    fn at(self, location: impl Into<SourceLocation>) -> Loc<T>;

    #[inline(always)]
    fn at_begin(self) -> Loc<T>
    where
        Self: Sized,
    {
        self.at(SourceLocation::begin())
    }

    #[inline(always)]
    fn at_invalid(self) -> Loc<T>
    where
        Self: Sized,
    {
        self.at(SourceLocation::invalid())
    }
}

/// Blanket implementation for all types to provide a default implementation of [`IntoLoc`]
impl<T> IntoLoc<T> for T {
    #[inline(always)]
    fn at(self, location: impl Into<SourceLocation>) -> Loc<T> {
        Loc {
            node: self,
            location: location.into(),
        }
    }
}

/// Utility trait to get the location of a node containing location information
pub trait Located {
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
        assert!(!loc.overlaps(&SourceLocation::invalid()));
    }
}
