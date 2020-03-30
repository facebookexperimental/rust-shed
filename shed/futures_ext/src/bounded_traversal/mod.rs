/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Read the documentation of [bounded_traversal](crate::bounded_traversal::bounded_traversal),
//! [bounded_traversal_dag](crate::bounded_traversal::bounded_traversal_dag) and
//! [bounded_traversal_stream](crate::bounded_traversal::bounded_traversal_stream)

mod tree;
pub use tree::bounded_traversal;

mod dag;
pub use dag::bounded_traversal_dag;

mod stream;
pub use stream::bounded_traversal_stream;

mod common;

#[cfg(test)]
mod tests;

/// A type used frequently in fold-like invocations inside this module
pub type Iter<Out> = std::iter::Flatten<std::vec::IntoIter<Option<Out>>>;
