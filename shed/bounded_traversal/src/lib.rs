/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Read the documentation of [bounded_traversal](crate::bounded_traversal),
//! [bounded_traversal_dag](crate::bounded_traversal_dag) and
//! [bounded_traversal_stream](crate::bounded_traversal_stream)

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
