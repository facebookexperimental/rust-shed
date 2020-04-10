// (c) Facebook, Inc. and its affiliates. Confidential and proprietary.

//! Serde serializer for generating syntactically correct Python code.
//!
//! This is primarily intended for generating Python data structures and declarative
//! method calls rather than actual code such as function bodies.

#![deny(warnings)]

use serde::Serialize;
use std::mem;

mod ser;

pub fn to_string<T>(value: &T) -> Result<String, ser::Error>
where
    T: Serialize,
{
    let mut s = ser::Serializer::new();

    value.serialize(&mut s)?;

    Ok(s.output())
}

pub fn to_string_pretty<T>(value: &T) -> Result<String, ser::Error>
where
    T: Serialize,
{
    let mut s = ser::Serializer::new_pretty();

    value.serialize(&mut s)?;

    Ok(s.output())
}

/// Serialize structs, maps, sequences and tuples as function calls
pub fn function_call<T: Serialize>(name: &str, args: &T) -> Result<String, ser::Error> {
    let mut ser = ser::Serializer::new_pretty();
    let mut s = ser::CallSerializer::with(&mut ser, name);

    args.serialize(&mut s)?;

    mem::drop(s);

    Ok(ser.output())
}
