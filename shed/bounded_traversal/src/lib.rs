/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Bounded traversal of trees and dags.
//!
//! This crate implements generalized traversal over large trees and dags,
//! limiting the number of concurrent processing steps.
//!
//! Use [`bounded_traversal`] to traverse and process a tree to a single
//! result.  The tree will be unfolded to all leaves and then folded back
//! together again to accumulate the result.
//!
//! Use [`bounded_traversal_dag`] to traverse a dag in the same way.
//!
//! Use [`bounded_traversal_stream`] to traverse a tree and produce a stream
//! of items.  The tree is processed in an arbitrary order.
//!
//! Use [`bounded_traversal_ordered_stream`] to traverse a tree an produce an
//! ordered stream of elements.  The tree is processed in order, however this
//! requires additional processing and may be slower than unordered traversal.

#[macro_use]
mod error;
pub use error::BoundedTraversalError;

mod tree;
pub use tree::bounded_traversal;

mod dag;
pub use dag::bounded_traversal_dag;
pub use dag::bounded_traversal_dag_limited;

mod stream;
pub use stream::bounded_traversal_stream;
pub use stream::limited_by_key_shardable;

mod ordered_stream;
pub use ordered_stream::bounded_traversal_limited_ordered_stream;
pub use ordered_stream::bounded_traversal_ordered_stream;

mod common;
pub use common::OrderedTraversal;

#[cfg(test)]
mod tests;

/// A type used frequently in fold-like invocations inside this module
pub type Iter<Out> = std::iter::Flatten<std::vec::IntoIter<Option<Out>>>;
