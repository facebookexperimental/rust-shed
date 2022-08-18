/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::fmt;
use std::fmt::Display;
use std::fmt::Write;
use std::io;
use std::iter::once;
use std::iter::repeat;

use serde::ser;
use serde::ser::Impossible;
use serde::Serialize;

#[derive(Debug)]
pub enum Error {
    Uncallable,
    NonStringKey,
    InvalidIntLiteral(String),
    Message(String),
}

impl Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Uncallable => write!(fmt, "uncallable"),
            Error::NonStringKey => write!(fmt, "key/parameter must be a string"),
            Error::InvalidIntLiteral(lit) => write!(
                fmt,
                "Starlark doesn't support 64 bit ints and provided literal {} doesn't \
                fit in a 32 bit int",
                lit
            ),
            Error::Message(msg) => write!(fmt, "Error: {}", msg),
        }
    }
}

impl From<Error> for io::Error {
    fn from(err: Error) -> Self {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Serialization error: {}", err),
        )
    }
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl std::error::Error for Error {}

type Result<T> = std::result::Result<T, Error>;

pub struct Serializer {
    output: String,
    pretty: bool,
    indent: usize,
}

impl Serializer {
    pub fn new() -> Self {
        Serializer {
            output: String::new(),
            pretty: false,
            indent: 0,
        }
    }

    pub fn new_pretty() -> Self {
        Serializer {
            output: String::new(),
            pretty: true,
            indent: 0,
        }
    }

    pub fn append(&mut self, s: &str) {
        self.output += s;
    }

    pub fn output(self) -> String {
        self.output
    }

    fn newline(&mut self) {
        if self.pretty {
            self.output
                .extend(once('\n').chain(repeat(' ').take(self.indent)))
        }
    }

    fn indent(&mut self) {
        self.indent += 4;
    }

    fn outdent(&mut self) {
        self.indent -= 4;
        self.newline();
    }
}

/// Returns character for backslash quoting
fn needs_quoting(c: char) -> Option<char> {
    match c {
        '"' => Some('"'),
        '\'' => Some('\''),
        '\\' => Some('\\'),
        '\n' => Some('n'),
        _ => None,
    }
}

pub struct SeqSerializer<T> {
    newlines: bool,
    serializer: T,
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = SeqSerializer<Self>;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.output += if v { "True" } else { "False" };
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i32(i32::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i32(i32::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        let v: i32 = match v.try_into() {
            Ok(v) => v,
            Err(_) => {
                return Err(Error::InvalidIntLiteral(v.to_string()));
            }
        };
        let _ = write!(&mut self.output, "{}", v);
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u32(u32::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u32(u32::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        let v: i32 = match v.try_into() {
            Ok(v) => v,
            Err(_) => {
                return Err(Error::InvalidIntLiteral(v.to_string()));
            }
        };
        let _ = write!(&mut self.output, "{}", v);
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.output += "\""; // `u` prefix for unicode?

        for c in v.chars() {
            if let Some(quote) = needs_quoting(c) {
                self.output.push('\\');
                self.output.push(quote);
            } else {
                self.output.push(c);
            }
        }

        self.output.push('"');

        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        self.output += "b\"";

        for b in v {
            if b.is_ascii_graphic() && needs_quoting(*b as char).is_none() {
                self.output.push(*b as char);
            } else {
                let _ = write!(&mut self.output, "\\x{:02x}", b);
            }
        }
        self.output.push('"');

        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        self.output += "None";
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if let Some(s) = name.strip_prefix("call:") {
            value.serialize(&mut CallSerializer::with(self, s))
        } else {
            value.serialize(self)
        }
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.output += "{";
        variant.serialize(&mut *self)?;
        self.output += ": ";
        value.serialize(&mut *self)?;
        self.output += "}";
        Ok(())
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.output += "[";
        Ok(SeqSerializer {
            newlines: len.unwrap_or(1) > 1,
            serializer: self,
        })
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        self.output += "(";
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.output += "{";
        variant.serialize(&mut *self)?;
        self.output += ":(";
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        self.output += "{";
        self.indent();
        Ok(self)
    }

    fn serialize_struct(self, name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        let _ = write!(&mut self.output, "{}(", name);
        self.indent();

        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.output += "{";
        variant.serialize(&mut *self)?;
        self.output += ":{";
        Ok(self)
    }
}

impl<'a> ser::SerializeSeq for SeqSerializer<&'a mut Serializer> {
    type Ok = ();
    type Error = Error;

    // Serialize a single element of the sequence.
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut *self.serializer)?;
        self.serializer.output += ",";
        if self.newlines {
            self.serializer.newline();
        }
        Ok(())
    }

    // Close the sequence.
    fn end(self) -> Result<()> {
        self.serializer.output += "]";
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)?;
        self.output += ",";
        Ok(())
    }

    fn end(self) -> Result<()> {
        self.output += ")";
        Ok(())
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)?;
        self.output += ",";
        Ok(())
    }

    fn end(self) -> Result<()> {
        self.output += ")";
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut **self)?;
        self.output += ",";
        Ok(())
    }

    fn end(self) -> Result<()> {
        self.output += ")}";
        Ok(())
    }
}

impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.newline();
        key.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.output += ": ";
        value.serialize(&mut **self)?;
        self.output += ",";
        Ok(())
    }

    fn end(self) -> Result<()> {
        self.outdent();
        self.output += "}";
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.newline();
        self.output += key;
        self.output += " = ";
        value.serialize(&mut **self)?;
        self.output += ",";

        Ok(())
    }

    fn end(self) -> Result<()> {
        self.outdent();
        self.output += ")";

        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        key.serialize(&mut **self)?;
        self.output += ": ";
        value.serialize(&mut **self)?;
        self.output += ",";
        self.newline();
        Ok(())
    }

    fn end(self) -> Result<()> {
        self.outdent();
        self.output += "}}";
        Ok(())
    }
}

/// Serializer specialized for Starlark calls. Structs and maps turn into named argument
/// calls, whereas tuples, sequences, units and newtypes turn into positional calls.
/// Arguments and inner parts are serialized with Serializer.
pub struct CallSerializer<'a>(&'a mut Serializer);

impl<'a> CallSerializer<'a> {
    pub fn with(ser: &'a mut Serializer, name: &str) -> Self {
        ser.output += name;
        CallSerializer(ser)
    }
}

impl<'a> ser::Serializer for &'a mut CallSerializer<'_> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Impossible<(), Error>;

    fn serialize_bool(self, _v: bool) -> Result<()> {
        Err(Error::Uncallable)
    }

    fn serialize_i8(self, _v: i8) -> Result<()> {
        Err(Error::Uncallable)
    }

    fn serialize_i16(self, _v: i16) -> Result<()> {
        Err(Error::Uncallable)
    }

    fn serialize_i32(self, _v: i32) -> Result<()> {
        Err(Error::Uncallable)
    }

    fn serialize_i64(self, _v: i64) -> Result<()> {
        Err(Error::Uncallable)
    }

    fn serialize_u8(self, _v: u8) -> Result<()> {
        Err(Error::Uncallable)
    }

    fn serialize_u16(self, _v: u16) -> Result<()> {
        Err(Error::Uncallable)
    }

    fn serialize_u32(self, _v: u32) -> Result<()> {
        Err(Error::Uncallable)
    }

    fn serialize_u64(self, _v: u64) -> Result<()> {
        Err(Error::Uncallable)
    }

    fn serialize_f32(self, _v: f32) -> Result<()> {
        Err(Error::Uncallable)
    }

    fn serialize_f64(self, _v: f64) -> Result<()> {
        Err(Error::Uncallable)
    }

    fn serialize_char(self, _v: char) -> Result<()> {
        Err(Error::Uncallable)
    }

    fn serialize_str(self, _v: &str) -> Result<()> {
        Err(Error::Uncallable)
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        Err(Error::Uncallable)
    }

    fn serialize_none(self) -> Result<()> {
        Err(Error::Uncallable)
    }

    fn serialize_some<T>(self, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Uncallable)
    }

    fn serialize_unit(self) -> Result<()> {
        Err(Error::Uncallable)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.0.output += "()";
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        Err(Error::Uncallable)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.0.output += "(";
        let res = value.serialize(&mut *self.0)?;
        self.0.output += ")";
        Ok(res)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::Uncallable)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.0.output += "(";
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        self.0.output += "(";
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::Uncallable)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        self.0.output += "(";
        self.0.indent();
        Ok(self)
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::Uncallable)
    }
}

impl<'a> ser::SerializeSeq for &'a mut CallSerializer<'_> {
    type Ok = ();
    type Error = Error;

    // Serialize a single element of the sequence.
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut *self.0)?;
        self.0.output += ", ";
        Ok(())
    }

    // Close the sequence.
    fn end(self) -> Result<()> {
        self.0.output += ")";
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut CallSerializer<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut *self.0)?;
        self.0.output += ", ";
        Ok(())
    }

    fn end(self) -> Result<()> {
        self.0.output += ")";
        Ok(())
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut CallSerializer<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(&mut *self.0)?;
        self.0.output += ", ";
        Ok(())
    }

    fn end(self) -> Result<()> {
        self.0.output += ")";
        Ok(())
    }
}

impl<'a> ser::SerializeMap for &'a mut CallSerializer<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.0.newline();
        key.serialize(BareStringSerializer(&mut *self.0))
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.0.output += " = ";
        value.serialize(&mut *self.0)?;
        self.0.output += ",";
        Ok(())
    }

    fn end(self) -> Result<()> {
        self.0.outdent();
        self.0.output += ")";
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for &'a mut CallSerializer<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.0.newline();
        self.0.output += key;
        self.0.output += " = ";
        value.serialize(&mut **self)?;
        self.0.output += ",";

        Ok(())
    }

    fn end(self) -> Result<()> {
        self.0.outdent();
        self.0.output += ")";

        Ok(())
    }
}

/// Emit a bare unquoted string for a function parameter name (assumes the name is OK).
/// Fails on everything else.
struct BareStringSerializer<'a>(&'a mut Serializer);

impl<'a> ser::Serializer for BareStringSerializer<'a> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Impossible<(), Error>;
    type SerializeStruct = Impossible<(), Error>;
    type SerializeStructVariant = Impossible<(), Error>;

    fn serialize_bool(self, _v: bool) -> Result<()> {
        Err(Error::NonStringKey)
    }

    fn serialize_i8(self, _v: i8) -> Result<()> {
        Err(Error::NonStringKey)
    }

    fn serialize_i16(self, _v: i16) -> Result<()> {
        Err(Error::NonStringKey)
    }

    fn serialize_i32(self, _v: i32) -> Result<()> {
        Err(Error::NonStringKey)
    }

    fn serialize_i64(self, _v: i64) -> Result<()> {
        Err(Error::NonStringKey)
    }

    fn serialize_u8(self, _v: u8) -> Result<()> {
        Err(Error::NonStringKey)
    }

    fn serialize_u16(self, _v: u16) -> Result<()> {
        Err(Error::NonStringKey)
    }

    fn serialize_u32(self, _v: u32) -> Result<()> {
        Err(Error::NonStringKey)
    }

    fn serialize_u64(self, _v: u64) -> Result<()> {
        Err(Error::NonStringKey)
    }

    fn serialize_f32(self, _v: f32) -> Result<()> {
        Err(Error::NonStringKey)
    }

    fn serialize_f64(self, _v: f64) -> Result<()> {
        Err(Error::NonStringKey)
    }

    fn serialize_char(self, _v: char) -> Result<()> {
        Err(Error::NonStringKey)
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.0.output += v;
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        Err(Error::NonStringKey)
    }

    fn serialize_none(self) -> Result<()> {
        Err(Error::NonStringKey)
    }

    fn serialize_some<T>(self, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::NonStringKey)
    }

    fn serialize_unit(self) -> Result<()> {
        Err(Error::NonStringKey)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Err(Error::NonStringKey)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        Err(Error::NonStringKey)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::NonStringKey)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(Error::NonStringKey)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(Error::NonStringKey)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(Error::NonStringKey)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Error::NonStringKey)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::NonStringKey)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::NonStringKey)
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Err(Error::NonStringKey)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::NonStringKey)
    }
}
