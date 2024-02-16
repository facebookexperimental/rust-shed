/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! See the [ScubaValue] documentation

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::f32;
use std::f64;
use std::fmt;
use std::fmt::Display;

use serde::de;
use serde::de::Deserialize;
use serde::de::Deserializer;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::ser::Serialize;
use serde::ser::SerializeSeq;
use serde::ser::Serializer;
use serde_json::Number;
use serde_json::Value;

/// A typed version of the Null value - used in serialization to understand the
/// type of the value that is not set in this sample.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum NullScubaValue {
    /// Integer type
    Int,
    /// Double-precision floating-point type
    Double,
    /// Basically a String type
    Normal,
    /// A deprecated String type.
    #[deprecated(note = "use Normal instead, see <https://fburl.com/qa/ep3v9a1h>.")]
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
    /// A deprecated String type.
    #[deprecated(note = "use Normal instead, see <https://fburl.com/qa/ep3v9a1h>.")]
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
            #[allow(deprecated)]
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

impl Serialize for ScubaValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            &ScubaValue::Int(v) => serializer.serialize_i64(v),
            &ScubaValue::Double(v) => serializer.serialize_f64(v),
            #[allow(deprecated)]
            &ScubaValue::Normal(ref v) | &ScubaValue::Denorm(ref v) => serializer.collect_str(&v),
            ScubaValue::NormVector(v) => {
                let mut seq = serializer.serialize_seq(Some(v.len()))?;
                for element in v {
                    seq.serialize_element(&element)?;
                }
                seq.end()
            }
            ScubaValue::TagSet(v) => {
                // Need to sort HashSet values to ensure deterministic JSON output.
                let mut vec = v.iter().collect::<Vec<_>>();
                vec.sort();

                let mut seq = serializer.serialize_seq(Some(vec.len()))?;
                for element in vec {
                    seq.serialize_element(&element)?;
                }
                seq.end()
            }
            &ScubaValue::Null(_) => serializer.serialize_none(),
        }
    }
}

struct ScubaValueVisitor;

impl<'de> Visitor<'de> for ScubaValueVisitor {
    type Value = ScubaValue;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("invalid scuba value")
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value.into())
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value.into())
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value.into())
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_string(value.to_string())
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value.into())
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut norm_vector = Vec::<String>::new();
        while let Some(item) = seq.next_element()? {
            norm_vector.push(item);
        }
        Ok(norm_vector.into())
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(ScubaValue::Null(NullScubaValue::Int))
    }
}

impl<'de> Deserialize<'de> for ScubaValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ScubaValueVisitor)
    }
}

/// **DEPRECATED: Please use serde's serialization directly instead.**
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
            #[allow(deprecated)]
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

/// **DEPRECATED: Please use serde's deserialization directly instead.**
impl TryFrom<Value> for ScubaValue {
    type Error = Value;

    fn try_from(this: Value) -> Result<ScubaValue, Value> {
        match this {
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
            Value::Bool(b) => Ok(ScubaValue::Normal(format!("{b}"))),
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

            impl From<Option<$t>> for ScubaValue {
                fn from(value: Option<$t>) -> Self {
                    match value {
                        None => ScubaValue::Null(NullScubaValue::Int),
                        Some(v) => ScubaValue::Int(v as i64),
                    }
                }
            }
        )*
    };
}

from_int_types!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

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

impl From<&str> for ScubaValue {
    fn from(value: &str) -> Self {
        ScubaValue::Normal(value.to_string())
    }
}
impl<T: Into<String>> From<Vec<T>> for ScubaValue {
    fn from(value: Vec<T>) -> Self {
        ScubaValue::NormVector(value.into_iter().map(|v| v.into()).collect())
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

impl<K: AsRef<str>, V: AsRef<str>> From<HashMap<K, V>> for ScubaValue {
    fn from(value: HashMap<K, V>) -> Self {
        let mut values: Vec<String> = value
            .iter()
            .map(|(k, v)| format!("{}:{}", k.as_ref(), v.as_ref()))
            .collect();
        values.sort();
        ScubaValue::NormVector(values)
    }
}

impl From<BTreeSet<String>> for ScubaValue {
    fn from(value: BTreeSet<String>) -> Self {
        ScubaValue::TagSet(HashSet::from_iter(value))
    }
}

impl<K: AsRef<str>, V: AsRef<str>> From<BTreeMap<K, V>> for ScubaValue {
    fn from(value: BTreeMap<K, V>) -> Self {
        let mut values: Vec<String> = value
            .iter()
            .map(|(k, v)| format!("{}:{}", k.as_ref(), v.as_ref()))
            .collect();
        values.sort();
        ScubaValue::NormVector(values)
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

impl From<Option<&String>> for ScubaValue {
    fn from(value: Option<&String>) -> Self {
        ScubaValue::from(value.map(|s| &s[..]))
    }
}

impl<'a> From<Option<&'a str>> for ScubaValue {
    fn from(value: Option<&'a str>) -> Self {
        match value {
            None => ScubaValue::Null(NullScubaValue::Normal),
            Some(s) => ScubaValue::Normal(s.to_string()),
        }
    }
}

impl From<Option<f64>> for ScubaValue {
    fn from(value: Option<f64>) -> Self {
        match value {
            None => ScubaValue::Null(NullScubaValue::Double),
            Some(v) => ScubaValue::Double(v),
        }
    }
}

impl From<Option<f32>> for ScubaValue {
    fn from(value: Option<f32>) -> Self {
        match value {
            None => ScubaValue::Null(NullScubaValue::Double),
            Some(v) => ScubaValue::Double(v as f64),
        }
    }
}

impl From<Option<bool>> for ScubaValue {
    fn from(value: Option<bool>) -> Self {
        match value {
            None => ScubaValue::Null(NullScubaValue::Normal),
            Some(v) => ScubaValue::Normal(v.to_string()),
        }
    }
}

impl<'a> From<HashSet<&'a str>> for ScubaValue {
    fn from(value: HashSet<&'a str>) -> Self {
        let set = value.into_iter().map(|s| s.to_string()).collect();
        ScubaValue::TagSet(set)
    }
}

impl<'a> From<BTreeSet<&'a str>> for ScubaValue {
    fn from(value: BTreeSet<&'a str>) -> Self {
        let set = value.into_iter().map(|s| s.to_string()).collect();
        ScubaValue::TagSet(set)
    }
}

#[cfg(test)]
mod tests {
    #![allow(deprecated)]

    use assert_matches::assert_matches;
    use quickcheck::quickcheck;
    use serde_json::json;
    use serde_json::Value;

    use super::*;

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
                    ScubaValue::Double(v) => v == ($x as f64) || (v.is_nan() && $x.is_nan()),
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
        fn from_i128(n: i128) -> bool { test_from_int!(n) }
        fn from_isize(n: isize) -> bool { test_from_int!(n) }
        fn from_u8(n: u8) -> bool { test_from_int!(n) }
        fn from_u16(n: u16) -> bool { test_from_int!(n) }
        fn from_u32(n: u32) -> bool { test_from_int!(n) }
        fn from_u64(n: u64) -> bool { test_from_int!(n) }
        fn from_u128(n: u128) -> bool { test_from_int!(n) }
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
    fn from_option_string() {
        assert_matches!(ScubaValue::from(Some("str")), ScubaValue::Normal(_));
        assert_matches!(
            ScubaValue::from(Some("str".to_string())),
            ScubaValue::Normal(_)
        );
        assert_matches!(
            ScubaValue::from(Some(&"str".to_string())),
            ScubaValue::Normal(_)
        );
        assert_matches!(
            ScubaValue::from(None::<String>),
            ScubaValue::Null(NullScubaValue::Normal)
        );
        assert_matches!(
            ScubaValue::from(None::<&'static str>),
            ScubaValue::Null(NullScubaValue::Normal)
        );
        // Option<bool>
        assert_matches!(
            ScubaValue::from(None::<bool>),
            ScubaValue::Null(NullScubaValue::Normal)
        );
    }

    #[test]
    fn from_hashmap_string() {
        let mut input = HashMap::new();
        input.insert("foo", "bar");
        input.insert("bar", "10");

        let expected = vec!["bar:10", "foo:bar"];

        assert_matches!(
            ScubaValue::from(input),
            ScubaValue::NormVector(ref actual) if *actual == expected
        );
    }

    #[test]
    fn from_btree_map_string() {
        let mut input = BTreeMap::new();
        input.insert("foo", "bar");
        input.insert("bar", "10");

        let expected = vec!["bar:10", "foo:bar"];

        assert_matches!(
            ScubaValue::from(input),
            ScubaValue::NormVector(ref actual) if *actual == expected
        );
    }

    macro_rules! test_option_int {
        ( $( $t:ty ),* ) => {
            $(
                assert_matches!(ScubaValue::from(Some(1 as $t)), ScubaValue::Int(1));
                assert_matches!(
                    ScubaValue::from(None::<$t>),
                    ScubaValue::Null(NullScubaValue::Int)
                );
            )*
        };
    }

    #[test]
    fn from_option_int() {
        test_option_int!(
            i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
        );
    }

    #[test]
    fn from_option_float() {
        assert_matches!(ScubaValue::from(Some(1f32)), ScubaValue::Double(_));
        assert_matches!(ScubaValue::from(Some(1f64)), ScubaValue::Double(_));

        assert_matches!(
            ScubaValue::from(None::<f32>),
            ScubaValue::Null(NullScubaValue::Double)
        );
        assert_matches!(
            ScubaValue::from(None::<f64>),
            ScubaValue::Null(NullScubaValue::Double)
        );
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
    fn serialize() {
        use serde_json::to_value;
        assert_eq!(to_value(ScubaValue::Int(123)).unwrap(), json!(123),);
        assert_eq!(to_value(ScubaValue::Int(-123)).unwrap(), json!(-123),);

        assert_eq!(to_value(ScubaValue::Double(1.5)).unwrap(), json!(1.5),);

        assert_eq!(
            to_value(&ScubaValue::Normal("abc".to_string())).unwrap(),
            json!("abc")
        );
        assert_eq!(
            to_value(&ScubaValue::Denorm("abc".to_string())).unwrap(),
            json!("abc")
        );

        assert_eq!(to_value(ScubaValue::NormVector(vec![])).unwrap(), json!([]),);
        assert_eq!(
            to_value(ScubaValue::NormVector(vec![
                "b".to_string(),
                "".to_string(),
                "a".to_string()
            ]))
            .unwrap(),
            json!(["b", "", "a"]),
        );

        assert_eq!(
            to_value(ScubaValue::TagSet(vec![].into_iter().collect())).unwrap(),
            json!([]),
        );
        assert_eq!(
            to_value(ScubaValue::TagSet(
                vec!["b".to_string(), "".to_string(), "a".to_string()]
                    .into_iter()
                    .collect()
            ))
            .unwrap(),
            json!(["", "a", "b"]),
        );

        assert_eq!(
            to_value(ScubaValue::Null(NullScubaValue::Int)).unwrap(),
            json!(null),
        );
        assert_eq!(
            to_value(ScubaValue::Null(NullScubaValue::Double)).unwrap(),
            json!(null),
        );
        assert_eq!(
            to_value(ScubaValue::Null(NullScubaValue::Normal)).unwrap(),
            json!(null),
        );
        assert_eq!(
            to_value(ScubaValue::Null(NullScubaValue::Denorm)).unwrap(),
            json!(null),
        );
        assert_eq!(
            to_value(ScubaValue::Null(NullScubaValue::NormVector)).unwrap(),
            json!(null),
        );
        assert_eq!(
            to_value(ScubaValue::Null(NullScubaValue::TagSet)).unwrap(),
            json!(null),
        );
    }

    macro_rules! test_deserialize_int {
        ( $( $t:ty ),* ) => {
            $(
                assert_eq!(
                    from_str::<'_, ScubaValue>(&json!(123 as $t).to_string()).unwrap(),
                    ScubaValue::Int(123)
                );
            )*
        };
    }

    #[test]
    fn deserialize() {
        use serde_json::from_str;

        test_deserialize_int!(
            i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
        );

        assert_eq!(
            from_str::<'_, ScubaValue>(&json!(1.5_f32).to_string()).unwrap(),
            ScubaValue::Double(1.5)
        );
        assert_eq!(
            from_str::<'_, ScubaValue>(&json!(1.5_f64).to_string()).unwrap(),
            ScubaValue::Double(1.5)
        );

        assert_eq!(
            from_str::<'_, ScubaValue>(&json!("abc").to_string()).unwrap(),
            ScubaValue::Normal("abc".to_string())
        );

        assert_eq!(
            from_str::<'_, ScubaValue>(&json!(vec![] as Vec<String>).to_string()).unwrap(),
            ScubaValue::NormVector(vec![])
        );
        assert_eq!(
            from_str::<'_, ScubaValue>(&json!(["b", "", "a"]).to_string()).unwrap(),
            ScubaValue::NormVector(vec!["b".to_string(), "".to_string(), "a".to_string()])
        );

        assert_eq!(
            from_str::<'_, ScubaValue>(&json!(null).to_string()).unwrap(),
            ScubaValue::Null(NullScubaValue::Int)
        );
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
    fn btree_tagset_sorted() {
        let vec = vec!["6", "8", "5", "4", "3", "9", "2", "0", "7", "1"];
        let set = vec.into_iter().collect::<BTreeSet<_>>();
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
