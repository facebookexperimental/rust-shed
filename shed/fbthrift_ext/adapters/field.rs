/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Locating an adapter failure within the struct it came from.

use std::fmt;

use fbthrift::metadata::ThriftAnnotations;

/// Identifies the struct field an adapter rejected a value for.
///
/// fbthrift hands every adapter the owning struct and the thrift field ID through
/// [`ThriftAdapter::from_thrift_field`](fbthrift::adapter::ThriftAdapter::from_thrift_field), but
/// the default implementation discards both. A struct with several adapted fields therefore
/// produces the same message whichever one failed, which is exactly when you most want to know.
/// Adapters in this crate capture it instead.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AdaptedField {
    /// Rust path of the struct owning the field.
    pub struct_name: &'static str,
    /// Thrift field ID.
    pub field_id: i16,
}

impl AdaptedField {
    /// Record the field currently being adapted.
    pub fn of<T: ThriftAnnotations>(field_id: i16) -> Self {
        Self {
            struct_name: std::any::type_name::<T>(),
            field_id,
        }
    }
}

impl fmt::Display for AdaptedField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} field {}", self.struct_name, self.field_id)
    }
}

/// Renders `" for <struct> field <id>"`, or nothing when there is no field context.
///
/// Lets an error's `Display` read naturally whether or not the adapter was applied to a field.
pub(crate) struct DisplayField(pub(crate) Option<AdaptedField>);

impl fmt::Display for DisplayField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(field) => write!(f, " for {field}"),
            None => Ok(()),
        }
    }
}
