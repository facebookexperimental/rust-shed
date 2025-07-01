/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::borrow::Cow;

use chrono::NaiveDateTime;
use mysql_common::value::convert::ToValue;
use seq_macro::seq;

/// Trait required for converting/formatting values into a SQL query.
pub trait ToSQL {
    /// Produce a properly-escaped string representation of this value, suitable for interpolation
    /// into a query to MySQL.
    fn to_sql_string(&self) -> Cow<str>;
}

impl<T: ToSQL + ?Sized> ToSQL for &T {
    fn to_sql_string(&self) -> Cow<str> {
        T::to_sql_string(self)
    }
}

impl<T: ToSQL> ToSQL for Option<T> {
    fn to_sql_string(&self) -> Cow<str> {
        match self {
            None => Cow::Borrowed("NULL"),
            Some(v) => v.to_sql_string(),
        }
    }
}

/// Trait required for trying to convert/format values into a SQL query.
/// Fallible version of ToSQL.
pub trait TryToSQL {
    type Error;
    /// Produce a properly-escaped string representation of this value, suitable for interpolation
    /// into a query to MySQL.
    fn try_to_sql_string(&self) -> Result<Cow<str>, Self::Error>;
}

impl<T: TryToSQL + ?Sized> TryToSQL for &T {
    type Error = T::Error;

    fn try_to_sql_string(&self) -> Result<Cow<str>, Self::Error> {
        T::try_to_sql_string(self)
    }
}

impl<T: TryToSQL> TryToSQL for Option<T> {
    type Error = T::Error;

    fn try_to_sql_string(&self) -> Result<Cow<str>, Self::Error> {
        match self {
            None => Ok(Cow::Borrowed("NULL")),
            Some(v) => v.try_to_sql_string(),
        }
    }
}

macro_rules! impl_to_sql_from_to_value {
    ($type:ty) => {
        impl ToSQL for $type {
            fn to_sql_string(&self) -> Cow<str> {
                Cow::Owned(ToValue::to_value(&self).as_sql(false))
            }
        }

        impl TryToSQL for $type {
            type Error = std::convert::Infallible;

            fn try_to_sql_string(&self) -> Result<Cow<str>, Self::Error> {
                Ok(Cow::Owned(ToValue::to_value(&self).as_sql(false)))
            }
        }
    };
}

macro_rules! impl_to_sql_for_sql_common_values_ref {
    ($type:ty) => {
        impl_to_sql_from_to_value!(&$type);
    };
}

macro_rules! impl_to_sql_for_sql_common_values {
    ($type:ty) => {
        impl_to_sql_from_to_value!($type);
    };
}

// TODO: Make a generic `impl<T: ToValue> ToSQL for T` when
// specialization for trait impl is stabilized in Rust
impl_to_sql_for_sql_common_values!(i8);
impl_to_sql_for_sql_common_values!(i16);
impl_to_sql_for_sql_common_values!(i32);
impl_to_sql_for_sql_common_values!(i64);
impl_to_sql_for_sql_common_values!(usize);
impl_to_sql_for_sql_common_values!(isize);
impl_to_sql_for_sql_common_values!(u8);
impl_to_sql_for_sql_common_values!(u16);
impl_to_sql_for_sql_common_values!(u32);
impl_to_sql_for_sql_common_values!(u64);
impl_to_sql_for_sql_common_values!(f64);
impl_to_sql_for_sql_common_values!(String);
impl_to_sql_for_sql_common_values!(Vec<u8>);
impl_to_sql_for_sql_common_values_ref!([u8]);
impl_to_sql_for_sql_common_values_ref!(str);
impl_to_sql_for_sql_common_values!(bool);

macro_rules! impl_to_sql_for_arrays {
    ($($n:expr_2021,)*) => {
        $(impl_to_sql_for_sql_common_values!([u8; $n]);)*
    }
}

impl_to_sql_for_arrays!(
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32,
);

macro_rules! impl_to_sql_for_tuple {
    ($($type:ident,)*) => {
        impl<$($type: ToSQL),*> ToSQL for ($($type,)*) {
            #[allow(non_snake_case, unused_assignments)]
            fn to_sql_string(&self) -> Cow<str> {
                let ($($type,)*) = self;

                $(
                    let $type = ToSQL::to_sql_string($type);
                )*

                // $size is >=1, so we can ascribe either a "("+")" or
                // a ", " to each element, both of which are len 2.
                let len = 0 $(+ 2 + $type.len())*;
                let mut ret = String::with_capacity(len);
                let mut sep = "";

                ret.push('(');

                $(
                    ret.push_str(sep);
                    sep = ", ";
                    ret.push_str(&*$type);
                )*

                ret.push(')');

                Cow::Owned(ret)
            }
        }

        impl<$($type: ToSQL),*> TryToSQL for ($($type,)*) {
            type Error = std::convert::Infallible;

            #[allow(non_snake_case, unused_assignments)]
            fn try_to_sql_string(&self) -> Result<Cow<str>, Self::Error> {
                Ok(ToSQL::to_sql_string(self))
            }
        }
    };
}

// Generates impls for up to 30 parameters
seq!(N in 1..=30 {
    #(
        seq!(J in 1..=N {
            impl_to_sql_for_tuple!(
                #(T~J,)*
            );
        });
    )*
});

impl ToSQL for NaiveDateTime {
    fn to_sql_string(&self) -> Cow<str> {
        Cow::Owned(
            self.format("%Y-%m-%d %H:%M:%S%.6f")
                .to_string()
                .to_sql_string()
                .to_string(),
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_naive_date_time() {
        let value = NaiveDateTime::parse_from_str("2022-05-30T13:39:58.012345Z", "%+").unwrap();
        assert_eq!(value.to_sql_string(), "'2022-05-30 13:39:58.012345'");
    }
}
