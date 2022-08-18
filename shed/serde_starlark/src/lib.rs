/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Serde serializer for generating syntactically correct Python code.
//!
//! This is primarily intended for generating Python data structures and declarative
//! method calls rather than actual code such as function bodies.

#![deny(warnings)]

use serde::Serialize;

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
    {
        let mut s = ser::CallSerializer::with(&mut ser, name);

        args.serialize(&mut s)?;
    }

    Ok(ser.output())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_int_conversions() {
        crate::to_string(&(123_i64)).unwrap();
        crate::to_string(&(123_u64)).unwrap();

        assert!(crate::to_string(&(i32::MAX as i64 + 10)).is_err());
        assert!(crate::to_string(&(u32::MAX as u64 + 10)).is_err());
    }
}
