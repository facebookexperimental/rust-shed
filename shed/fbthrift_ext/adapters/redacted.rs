/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Adapters that prevent logging of sensitive data.

use std::marker::PhantomData;

use fbthrift::adapter::ThriftAdapter;
use serde_derive::Deserialize;
use serde_derive::Serialize;

const REDACTED: &str = "<REDACTED>";

/// Prevents logging of sensitive data used in RPC requests.
///
/// The wrapped types are not affected on the wire, but the generated fields
/// will use [`Redacted`] to prevent logging of sensitive data.
///
/// For more information, see implementation documentation.
pub struct RedactedAdapter<T> {
    inner: PhantomData<T>,
}

/// Helper trait to make writing the adapter easier.
pub trait Redactable: Clone + PartialEq + Send + Sync {}
impl<T: Clone + PartialEq + Send + Sync> Redactable for T {}

/// A wrapper type that prevents logging of sensitive data via commonly used traits.
///
/// This type is intended to be used as a field in a Thrift struct. It will ensure that
/// the field is not logged when the struct is printed via [`Debug`] or [`Valuable`].
///
/// This does not affect serialization or deserialization of the field, and *no* extra processing is
/// done (such as zeroing the value on destruction).  The field has no extra security applied, it
/// only prevents accidental logging.
///
/// The inner value is accessible via the [`unredact*`] methods.
///
/// To prevent accidental misuse, this type does not (and should not) implement the following traits:
/// - [`Deref`]
/// - [`Display`] (enables an ambiguous `to_string()` impl)
/// - [`Into`]
///
/// [`Debug`]: std::fmt::Debug
/// [`Display`]: std::fmt::Display
/// [`Valuable`]: valuable::Valuable
#[derive(Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Redacted<T: Redactable> {
    inner: T,
}

/// Purposefully doesn't implement [`Deref`] as that would make it easy to accidentally misuse the
/// values.
impl<T: Redactable> Redacted<T> {
    /// Consumes the `Redacted` and returns the inner value.
    pub fn unredact(self) -> T {
        self.inner
    }

    /// Borrows the `Redacted` and returns a reference to the inner value.
    pub fn unredact_ref(&self) -> &T {
        &self.inner
    }

    /// Mutably borrows the `Redacted` and returns a mutable reference to the inner value.
    pub fn unredact_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: Redactable> From<T> for Redacted<T> {
    fn from(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: Redactable> std::fmt::Debug for Redacted<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(REDACTED)
    }
}

impl<T: Redactable> valuable::Valuable for Redacted<T> {
    fn as_value(&self) -> valuable::Value {
        REDACTED.as_value()
    }

    fn visit(&self, visitor: &mut dyn valuable::Visit) {
        REDACTED.visit(visitor)
    }
}

/// Implementation for redacting a thrift string.
///
/// This adapter can perform round-trip serialization and deserialization
/// without transforming data for all non-empty inputs.
///
/// # Examples
///
/// ```thrift
/// include "thrift/annotation/rust.thrift";
///
/// @rust.Adapter{name = "::fbthrift_adapters::Redacted<>"}
/// typedef string RedactedString;
///
/// struct CreateWorkflowRequest {
///   1: RedactedString target;
/// }
/// ```
impl<T: Redactable> ThriftAdapter for RedactedAdapter<T> {
    type StandardType = T;
    type AdaptedType = Redacted<T>;

    type Error = std::convert::Infallible;

    fn to_thrift(value: &Self::AdaptedType) -> Self::StandardType {
        value.unredact_ref().clone()
    }

    fn from_thrift(value: Self::StandardType) -> Result<Self::AdaptedType, Self::Error> {
        Ok(Redacted { inner: value })
    }
}

#[cfg(test)]
mod string_impl {
    use valuable::Valuable;

    use super::*;

    #[test]
    fn round_trip() {
        let raw = "korra".to_string();
        let adapted = RedactedAdapter::from_thrift(raw.clone()).unwrap();
        assert_eq!(RedactedAdapter::to_thrift(&adapted), raw);
    }

    #[test]
    fn debug_redacted() {
        let raw = "sokka".to_string();
        let adapted = RedactedAdapter::from_thrift(raw.clone()).unwrap();
        assert_ne!(raw, format!("{adapted:?}"));
        assert_eq!(REDACTED, format!("{adapted:?}"));
    }

    #[test]
    fn valuable_redacted() {
        let raw = "secret tunnel".to_string();
        let adapted = RedactedAdapter::from_thrift(raw.clone()).unwrap();
        assert_ne!(raw, adapted.as_value().as_str().unwrap());
        assert_eq!(REDACTED, adapted.as_value().as_str().unwrap());
    }
}
