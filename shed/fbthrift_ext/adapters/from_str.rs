/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Parsing thrift strings into a domain type, with an error that says what failed.

use std::error::Error;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use std::marker::PhantomData;
use std::str::FromStr;

use fbthrift::adapter::ThriftAdapter;
use fbthrift::metadata::ThriftAnnotations;

use crate::field::AdaptedField;
use crate::field::DisplayField;

/// Adapts thrift strings via [`FromStr`], reporting the input when parsing fails.
///
/// [`fbthrift::adapter::FromStrAdapter`] does the same conversion, but surfaces the underlying
/// `FromStr::Err` unchanged. Several of those errors carry nothing at all — `AddrParseError`
/// renders as `invalid IP address syntax`, naming neither the string that was rejected nor the
/// field it came from — which leaves a deserialisation failure with nothing to act on. This keeps
/// the cause and adds both.
///
/// # Rejected values appear in errors
///
/// The rejected string is kept verbatim in [`FromStrError::input`] and printed by its `Display`,
/// so a failed deserialisation puts the raw field value wherever the error ends up -- logs, or a
/// response returned to the caller. Do not apply this adapter to fields that may carry secrets or
/// user data; [`fbthrift::adapter::FromStrAdapter`] surfaces only the underlying `FromStr::Err`.
pub struct FromStrAdapter<Adapted, Standard = String>(PhantomData<(Adapted, Standard)>);

/// The error returned by [`FromStrAdapter`] when the input does not parse.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FromStrError<E> {
    /// The string that failed to parse, verbatim. Printed by `Display`; see the note on
    /// [`FromStrAdapter`] before applying the adapter to sensitive fields.
    pub input: String,
    /// Rust path of the type it was being parsed into.
    pub target: &'static str,
    /// The struct field this came from, when the adapter was applied to a field rather than to a
    /// bare typedef.
    pub field: Option<AdaptedField>,
    /// The underlying [`FromStr`] failure.
    pub source: E,
}

impl<E: Display> Display for FromStrError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            input,
            target,
            field,
            source,
        } = self;
        write!(
            f,
            "cannot parse {input:?} as {target}{}: {source}",
            DisplayField(*field)
        )
    }
}

impl<E: Error + 'static> Error for FromStrError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

impl<T> ThriftAdapter for FromStrAdapter<T, String>
where
    T: FromStr + Display + Clone + Debug + Send + Sync + PartialEq,
    FromStrError<<T as FromStr>::Err>: Into<anyhow::Error> + Debug,
{
    type StandardType = String;
    type AdaptedType = T;
    type Error = FromStrError<<T as FromStr>::Err>;

    fn to_thrift(value: &Self::AdaptedType) -> String {
        value.to_string()
    }

    fn from_thrift(value: String) -> Result<Self::AdaptedType, Self::Error> {
        value.parse().map_err(|source| FromStrError {
            input: value,
            target: std::any::type_name::<T>(),
            field: None,
            source,
        })
    }

    fn from_thrift_field<S: ThriftAnnotations>(
        value: String,
        field_id: i16,
    ) -> Result<Self::AdaptedType, Self::Error> {
        value.parse().map_err(|source| FromStrError {
            input: value,
            target: std::any::type_name::<T>(),
            field: Some(AdaptedField::of::<S>(field_id)),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv6Addr;

    use super::*;

    type Adapter = FromStrAdapter<Ipv6Addr>;

    /// Stands in for a generated thrift struct so `from_thrift_field` can be exercised.
    struct DummyStruct;

    impl ThriftAnnotations for DummyStruct {}

    #[test]
    fn round_trip() {
        let raw = "::1".to_owned();
        let adapted = Adapter::from_thrift(raw.clone()).expect("::1 parses");
        assert_eq!(Adapter::to_thrift(&adapted), raw);
    }

    #[test]
    fn error_reports_the_rejected_input() {
        let error = Adapter::from_thrift("not_an_ip".to_owned()).expect_err("must not parse");
        assert_eq!(error.input, "not_an_ip");
        assert!(
            error.to_string().contains("not_an_ip"),
            "the rejected input is the whole point, got {error}"
        );
        assert!(
            error.to_string().contains("Ipv6Addr"),
            "the target type should be named, got {error}"
        );
    }

    #[test]
    fn error_reports_the_field() {
        let error = Adapter::from_thrift_field::<DummyStruct>(":::::1".to_owned(), 4)
            .expect_err("must not parse");
        assert_eq!(
            error
                .field
                .expect("field context must be recorded")
                .field_id,
            4
        );
        assert!(
            error.to_string().contains("field 4"),
            "message should name the field, got {error}"
        );
    }
}
