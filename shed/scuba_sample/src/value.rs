/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! See the [ScubaValue] documentation

use serde_json::{Number, Value};
use std::collections::HashSet;
use std::convert::{Into, TryFrom};
use std::fmt::{self, Display};
use std::iter::FromIterator;
use std::{f32, f64};

/// A typed version of the Null value - used in serialization to understand the
/// type of the value that is not set in this sample.
#[derive(Clone, PartialEq, Debug)]
pub enum NullScubaValue {
    /// Integer type
    Int,
    /// Double-precision floating-point type
    Double,
    /// Basically a String type
    Normal,
    /// A deprecated String type, see <https://fburl.com/qa/ep3v9a1h>
    Denorm,
    /// A list of String
    NormVector,
    /// A set of Strings
    TagSet,
}

/// An enum defining all the possible types consumable by Scuba samples.
#[derive(Clone, PartialEq, Debug)]
pub enum ScubaValue {
    /// Integer type
    Int(i64),
    /// Double-precision floating-point type
    Double(f64),
    /// Basically a String type
    Normal(String),
    /// A deprecated String type, see <https://fburl.com/qa/ep3v9a1h>
    Denorm(String),
    /// A list of String
    NormVector(Vec<String>),
    /// A set of Strings
    TagSet(HashSet<String>),
    /// The null type - it is itself typed for the serialization to work properly
    Null(NullScubaValue),
}

impl Display for ScubaValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match *self {
            ScubaValue::Int(i) => i.fmt(f),
            ScubaValue::Double(d) => d.fmt(f),
            ScubaValue::Normal(ref s) => s.fmt(f),
            ScubaValue::Denorm(ref s) => s.fmt(f),
            ScubaValue::NormVector(ref norms) => <Vec<String> as fmt::Debug>::fmt(norms, f),
            ScubaValue::TagSet(ref tags) => {
                let mut tags: Vec<&String> = tags.iter().collect();
                tags.sort();
                f.debug_set().entries(tags.iter()).finish()
            }
            ScubaValue::Null(_) => "null".fmt(f),
        }
    }
}

impl From<ScubaValue> for Value {
    fn from(val: ScubaValue) -> Value {
        match val {
            ScubaValue::Int(v) => Value::Number(Number::from(v)),
            ScubaValue::Double(v) => {
                // NaN and Infinity are not valid JSON numeric values, so in those cases
                // emit a JSON null value, which when logged to Scuba will make the
                // corresponding column appear to be missing from the sample.
                if let Some(value) = Number::from_f64(v) {
                    Value::Number(value)
                } else {
                    Value::Null
                }
            }
            ScubaValue::Normal(v) => Value::String(v),
            ScubaValue::Denorm(v) => Value::String(v),
            ScubaValue::NormVector(v) => Value::Array(v.into_iter().map(Value::String).collect()),
            ScubaValue::TagSet(v) => {
                // Need to sort HashSet values to ensure deterministic JSON output.
                let mut vec = v.into_iter().collect::<Vec<_>>();
                vec.sort();
                Value::Array(vec.into_iter().map(Value::String).collect())
            }
            ScubaValue::Null(_) => Value::Null,
        }
    }
}

impl TryFrom<Value> for ScubaValue {
    type Error = Value;

    fn try_from(this: Value) -> Result<ScubaValue, Value> {
        match this {
            // Use Normal instead of Denorm. See https://our.inter.facebook.com/intern/qa/4462
            Value::String(s) => Ok(ScubaValue::Normal(s)),
            Value::Number(i) => {
                if let Some(i) = i.as_i64() {
                    Ok(ScubaValue::Int(i))
                } else if let Some(f) = i.as_f64() {
                    Ok(ScubaValue::Double(f))
                } else {
                    Err(Value::Number(i))
                }
            }
            Value::Bool(b) => Ok(ScubaValue::Normal(format!("{}", b))),
            Value::Array(a) => {
                if let Ok(strings) = a
                    .iter()
                    .map(|i| {
                        if let Value::String(s) = i {
                            Ok(s.clone())
                        } else {
                            Err(())
                        }
                    })
                    .collect()
                {
                    Ok(ScubaValue::NormVector(strings))
                } else {
                    Err(Value::Array(a))
                }
            }
            Value::Null | Value::Object(_) => Err(this),
        }
    }
}

macro_rules! from_int_types {
    ( $( $t:ty ),* ) => {
        $(
            impl From<$t> for ScubaValue {
                fn from(value: $t) -> Self {
                    ScubaValue::Int(value as i64)
                }
            }
        )*
    };
}

from_int_types!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

impl From<bool> for ScubaValue {
    fn from(value: bool) -> Self {
        ScubaValue::Normal(value.to_string())
    }
}

impl From<f32> for ScubaValue {
    fn from(value: f32) -> Self {
        ScubaValue::Double(value as f64)
    }
}

impl From<f64> for ScubaValue {
    fn from(value: f64) -> Self {
        ScubaValue::Double(value)
    }
}

impl From<String> for ScubaValue {
    fn from(value: String) -> Self {
        ScubaValue::Normal(value)
    }
}

impl<'a> From<&'a str> for ScubaValue {
    fn from(value: &'a str) -> Self {
        ScubaValue::Normal(value.to_string())
    }
}

impl From<Vec<String>> for ScubaValue {
    fn from(value: Vec<String>) -> Self {
        ScubaValue::NormVector(value)
    }
}

impl<'a> From<Vec<&'a str>> for ScubaValue {
    fn from(value: Vec<&'a str>) -> Self {
        let vec = value.into_iter().map(|s| s.to_string()).collect();
        ScubaValue::NormVector(vec)
    }
}

impl<T: Into<String>> FromIterator<T> for ScubaValue {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let vec = iter.into_iter().map(|s| s.into()).collect();
        ScubaValue::NormVector(vec)
    }
}

impl From<HashSet<String>> for ScubaValue {
    fn from(value: HashSet<String>) -> Self {
        ScubaValue::TagSet(value)
    }
}

impl From<Option<String>> for ScubaValue {
    fn from(value: Option<String>) -> Self {
        match value {
            None => ScubaValue::Null(NullScubaValue::Normal),
            Some(s) => ScubaValue::Normal(s),
        }
    }
}

impl<'a> From<HashSet<&'a str>> for ScubaValue {
    fn from(value: HashSet<&'a str>) -> Self {
        let set = value.into_iter().map(|s| s.to_string()).collect();
        ScubaValue::TagSet(set)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use quickcheck::quickcheck;
    use serde_json::{json, Value};

    macro_rules! test_from_int {
        ( $x:expr ) => {
            match ScubaValue::from($x) {
                ScubaValue::Int(v) => v == ($x as i64),
                _ => false,
            }
        };
    }

    macro_rules! test_from_float {
        ( $x:expr ) => {
            #[allow(clippy::float_cmp)]
            {
                match ScubaValue::from($x) {
                    ScubaValue::Double(v) => v == ($x as f64),
                    _ => false,
                }
            }
        };
    }

    quickcheck! {
        fn from_i8(n: i8) -> bool { test_from_int!(n) }
        fn from_i16(n: i16) -> bool { test_from_int!(n) }
        fn from_i32(n: i32) -> bool { test_from_int!(n) }
        fn from_i64(n: i64) -> bool { test_from_int!(n) }
        fn from_isize(n: isize) -> bool { test_from_int!(n) }
        fn from_u8(n: u8) -> bool { test_from_int!(n) }
        fn from_u16(n: u16) -> bool { test_from_int!(n) }
        fn from_u32(n: u32) -> bool { test_from_int!(n) }
        fn from_u64(n: u64) -> bool { test_from_int!(n) }
        fn from_usize(n: usize) -> bool { test_from_int!(n) }
        fn from_f32(n: f32) -> bool { test_from_float!(n) }
        fn from_f64(n: f64) -> bool { test_from_float!(n) }
    }

    #[test]
    fn to_string() {
        assert_eq!(format!("{}", ScubaValue::from(6)), "6");
        assert_eq!(format!("{}", ScubaValue::from(888.8)), "888.8");

        assert_eq!(format!("{}", ScubaValue::from("scuba norm")), "scuba norm");
        assert_eq!(
            format!("{}", ScubaValue::Denorm("scuba denorm".into())),
            "scuba denorm"
        );

        assert_eq!(
            format!("{}", ScubaValue::from(vec!["hello", "world"])),
            "[\"hello\", \"world\"]"
        );
        assert_eq!(
            format!("{}", ScubaValue::from(vec!["world", "hello"])),
            "[\"world\", \"hello\"]"
        );

        let mut tags_one = HashSet::new();
        let mut tags_two = HashSet::new();

        tags_two.insert("hello");
        tags_one.insert("world");

        tags_one.insert("hello");
        tags_two.insert("world");

        assert_eq!(
            format!("{}", ScubaValue::from(tags_two)),
            "{\"hello\", \"world\"}"
        );
        assert_eq!(
            format!("{}", ScubaValue::from(tags_one)),
            "{\"hello\", \"world\"}"
        );
    }

    #[test]
    fn from_string() {
        assert_matches!(ScubaValue::from("test"), ScubaValue::Normal(_));
        assert_matches!(ScubaValue::from("test".to_string()), ScubaValue::Normal(_));
    }

    #[test]
    fn from_vec() {
        let str_vec = vec!["a", "b", "c"];
        let string_vec = vec!["a".to_string(), "b".to_string(), "c".to_string()];

        assert_matches!(
            ScubaValue::from(str_vec),
            ScubaValue::NormVector(ref v) if *v == string_vec
        );
        assert_matches!(
            ScubaValue::from(string_vec.clone()),
            ScubaValue::NormVector(ref v) if *v == string_vec
        );
    }

    #[test]
    fn from_set() {
        let str_vec = vec!["a", "b", "c"];
        let string_vec = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let str_set = str_vec.into_iter().collect::<HashSet<_>>();
        let string_set = string_vec.into_iter().collect::<HashSet<_>>();

        assert_matches!(
            ScubaValue::from(str_set),
            ScubaValue::TagSet(ref s) if *s == string_set
        );
        assert_matches!(
            ScubaValue::from(string_set.clone()),
            ScubaValue::TagSet(ref s) if *s == string_set
        );
    }

    #[test]
    fn from_iter() {
        let vec = vec!["a", "b", "c"];
        let value = vec.iter().cloned().collect::<ScubaValue>();
        assert_matches!(value, ScubaValue::NormVector(ref v) if *v == vec);
    }

    #[test]
    fn from_json_value() {
        use serde_json::json;
        assert_eq!(
            ScubaValue::try_from(json!("abc")).unwrap(),
            ScubaValue::Normal("abc".to_string())
        );
        assert_eq!(
            ScubaValue::try_from(json!(true)).unwrap(),
            ScubaValue::Normal("true".to_string())
        );
        assert_eq!(
            ScubaValue::try_from(json!(false)).unwrap(),
            ScubaValue::Normal("false".to_string())
        );
        assert_eq!(
            ScubaValue::try_from(json!(123)).unwrap(),
            ScubaValue::Int(123)
        );
        assert_eq!(
            ScubaValue::try_from(json!(-123)).unwrap(),
            ScubaValue::Int(-123)
        );
        assert_eq!(
            ScubaValue::try_from(json!(1.5)).unwrap(),
            ScubaValue::Double(1.5)
        );
        assert_eq!(
            ScubaValue::try_from(json!([])).unwrap(),
            ScubaValue::NormVector(vec![])
        );
        assert_eq!(
            ScubaValue::try_from(json!(["b", "", "a"])).unwrap(),
            ScubaValue::NormVector(vec!["b".to_string(), "".to_string(), "a".to_string()])
        );
        assert!(ScubaValue::try_from(json!({})).is_err());
        assert!(ScubaValue::try_from(json!(null)).is_err());
        assert!(ScubaValue::try_from(json!([null])).is_err());
    }

    #[test]
    fn tagset_sorted() {
        let vec = vec!["6", "8", "5", "4", "3", "9", "2", "0", "7", "1"];
        let set = vec.into_iter().collect::<HashSet<_>>();
        let value: Value = ScubaValue::from(set).into();
        let expected = json!(["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"]);
        assert_eq!(value, expected);
    }

    #[test]
    fn invalid_json_number() {
        assert_matches!(ScubaValue::from(f32::NAN).into(), Value::Null);
        assert_matches!(ScubaValue::from(f32::INFINITY).into(), Value::Null);
        assert_matches!(ScubaValue::from(f32::NEG_INFINITY).into(), Value::Null);
        assert_matches!(ScubaValue::from(f64::NAN).into(), Value::Null);
        assert_matches!(ScubaValue::from(f64::INFINITY).into(), Value::Null);
        assert_matches!(ScubaValue::from(f64::NEG_INFINITY).into(), Value::Null);
    }
}
