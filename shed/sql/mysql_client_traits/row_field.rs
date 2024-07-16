/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::fmt;

use chrono::NaiveDateTime;
use mysql_common::value::Value;

/// Representation of a field in a set of result rows from a query.
#[derive(Clone, Debug, PartialEq)]
pub enum RowField {
    /// Blobs, strings, and so on.
    Bytes(Vec<u8>),
    /// Floating point numbers.
    Double(f64),
    /// Signed integers.
    I64(i64),
    /// Unsigned integers.
    U64(u64),
    /// An SQL `NULL` value.
    Null,
}

impl fmt::Display for RowField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RowField::Bytes(bytes) => write!(
                f,
                "{}({:?})",
                "Bytes".to_owned(),
                String::from_utf8_lossy(&bytes[0..bytes.len().min(10)])
            ),
            RowField::Double(double) => write!(f, "{}({:?})", "Double".to_owned(), double),
            RowField::I64(int) => write!(f, "{}({})", "I64".to_owned(), int),
            RowField::U64(uint) => write!(f, "{}({})", "U64".to_owned(), uint),
            RowField::Null => write!(f, "NULL"),
        }
    }
}

/// `ValueError` represents an error in converting a particular [`RowField`]
/// into a corresponding Rust type via [`TryFromRowField`] or
/// [`OptionalTryFromRowField`].
#[derive(Debug, thiserror::Error)]
#[error("Value error: {0}")]
pub struct ValueError(pub String);

/// This represents conversion from a [`RowField`] for a value which may not be
/// present. The trait you need to implement to be able to read a query result
/// into the custom type.
pub trait OptionalTryFromRowField: Sized {
    fn try_from_opt(field: RowField) -> Result<Option<Self>, ValueError>;
}

/// Conversion from a [`RowField`] for a value which is expected to be present.
pub trait TryFromRowField: Sized {
    fn try_from(field: RowField) -> Result<Self, ValueError>;
}

impl<T: OptionalTryFromRowField> TryFromRowField for T {
    fn try_from(field: RowField) -> Result<Self, ValueError> {
        let opt_res = T::try_from_opt(field)?;
        opt_res.ok_or_else(|| {
            ValueError(format!(
                "Expected row field to be non-null, but it was null for type {}",
                std::any::type_name::<T>()
            ))
        })
    }
}

impl<T: OptionalTryFromRowField> TryFromRowField for Option<T> {
    fn try_from(field: RowField) -> Result<Self, ValueError> {
        T::try_from_opt(field)
    }
}

/// The function converts RowField object into Rust type if the type implements
/// mysql_async::FromValue.
pub fn opt_try_from_rowfield<T: mysql_common::value::convert::FromValue>(
    field: RowField,
) -> Result<T, ValueError> {
    let res: T = mysql_common::value::convert::from_value_opt(field.into()).map_err(|e| {
        ValueError(format!(
            "Failed to convert mysql_async::Value into the result type {}: {e}",
            std::any::type_name::<T>(),
        ))
    })?;
    Ok(res)
}

impl From<RowField> for Value {
    fn from(field: RowField) -> Self {
        match field {
            RowField::Bytes(bytes) => Value::Bytes(bytes),
            RowField::Double(double) => Value::Double(double),
            RowField::I64(int) => Value::Int(int),
            RowField::U64(int) => Value::UInt(int),
            RowField::Null => Value::NULL,
        }
    }
}
macro_rules! impl_try_from_for_int_types {
    ($field_type:ty) => {
        impl OptionalTryFromRowField for $field_type {
            fn try_from_opt(field: RowField) -> Result<Option<Self>, ValueError> {
                match field {
                    RowField::I64(value) => value
                        .try_into()
                        .map_err(|e| ValueError(format!("Failed to convert Int type: {e}")))
                        .map(Some),
                    RowField::U64(value) => value
                        .try_into()
                        .map_err(|e| ValueError(format!("Failed to convert Int type: {e}")))
                        .map(Some),
                    RowField::Double(_) => Err(ValueError(
                        "Expected row field to be RowField::Long, but got RowField::Double"
                            .to_string(),
                    )),
                    RowField::Bytes(_) => Err(ValueError(
                        "Expected row field to be RowField::Long, but got RowField::Bytes"
                            .to_string(),
                    )),
                    RowField::Null => Ok(None),
                }
            }
        }
    };
}

impl_try_from_for_int_types!(u8);
impl_try_from_for_int_types!(i8);
impl_try_from_for_int_types!(u16);
impl_try_from_for_int_types!(i16);
impl_try_from_for_int_types!(u32);
impl_try_from_for_int_types!(i32);
impl_try_from_for_int_types!(u64);
impl_try_from_for_int_types!(i64);
impl_try_from_for_int_types!(usize);
impl_try_from_for_int_types!(isize);

impl OptionalTryFromRowField for f64 {
    fn try_from_opt(field: RowField) -> Result<Option<Self>, ValueError> {
        match field {
            RowField::Double(value) => Ok(Some(value)),
            RowField::I64(_) => Err(ValueError(
                "Expected row field to be RowField::Double, but got RowField::I64".to_string(),
            )),
            RowField::U64(_) => Err(ValueError(
                "Expected row field to be RowField::Double, but got RowField::U64".to_string(),
            )),
            RowField::Bytes(_) => Err(ValueError(
                "Expected row field to be RowField::Double, but got RowField::Bytes".to_string(),
            )),
            RowField::Null => Ok(None),
        }
    }
}

impl OptionalTryFromRowField for Vec<u8> {
    fn try_from_opt(field: RowField) -> Result<Option<Self>, ValueError> {
        match field {
            RowField::Bytes(value) => Ok(Some(value)),
            RowField::I64(_) => Err(ValueError(
                "Expected row field to be RowField::Bytes, but got RowField::I64".to_string(),
            )),
            RowField::U64(_) => Err(ValueError(
                "Expected row field to be RowField::Bytes, but got RowField::U64".to_string(),
            )),
            RowField::Double(_) => Err(ValueError(
                "Expected row field to be RowField::Bytes, but got RowField::Double".to_string(),
            )),
            RowField::Null => Ok(None),
        }
    }
}

impl<const N: usize> OptionalTryFromRowField for [u8; N] {
    fn try_from_opt(field: RowField) -> Result<Option<Self>, ValueError> {
        match field {
            RowField::Bytes(value) => (&value[..])
                .try_into()
                .map_err(|_| {
                    ValueError(format!(
                        "Expected row field to be [u8; {}], but got {} bytes",
                        N,
                        value.len(),
                    ))
                })
                .map(Some),
            RowField::Double(_) => Err(ValueError(format!(
                "Expected row field to be [u8; {}], but got RowField::Double",
                N,
            ))),
            RowField::I64(_) => Err(ValueError(format!(
                "Expected row field to be [u8; {}], but got RowField::I64",
                N,
            ))),
            RowField::U64(_) => Err(ValueError(format!(
                "Expected row field to be [u8; {}], but got RowField::U64",
                N,
            ))),
            RowField::Null => Ok(None),
        }
    }
}

impl OptionalTryFromRowField for String {
    fn try_from_opt(field: RowField) -> Result<Option<Self>, ValueError> {
        let bytes: Option<Vec<u8>> = TryFromRowField::try_from(field)?;
        Ok(bytes.map(|bytes| String::from_utf8_lossy(&bytes[..]).into_owned()))
    }
}

impl OptionalTryFromRowField for bool {
    fn try_from_opt(field: RowField) -> Result<Option<Self>, ValueError> {
        // If we turn the field into `mysql_async::Value`, then `Value::Int(0)` and `Value::Int(1)`
        // can be converted into `false` and `true`, but `Value::UInt(0)` and `Value::UInt(1)`
        // cannot. So we have to deal with them separately.
        if let RowField::U64(0) = &field {
            return Ok(Some(false));
        } else if let RowField::U64(1) = &field {
            return Ok(Some(true));
        }

        opt_try_from_rowfield(field)
    }
}

/// Use NaiveDateTime values in Rust along with DATETIME columns in MySQL:
/// rust will format them as string and MySQL processes them as is,
/// not respecting time_zone setting.
///
/// It can be used with TIMESTAMP columns as long as time_zone MySQL setting
/// is constant and NaiveDateTime is represented in the same time zome.
impl OptionalTryFromRowField for NaiveDateTime {
    fn try_from_opt(field: RowField) -> Result<Option<Self>, ValueError> {
        opt_try_from_rowfield(field)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_naive_date_time() -> Result<(), ValueError> {
        assert_eq!(
            NaiveDateTime::parse_from_str("2022-05-30T13:39:58.012345Z", "%+").unwrap(),
            <NaiveDateTime as TryFromRowField>::try_from(RowField::Bytes(
                "2022-05-30 13:39:58.012345".as_bytes().to_vec()
            ))
            .unwrap()
        );
        Ok(())
    }
}
