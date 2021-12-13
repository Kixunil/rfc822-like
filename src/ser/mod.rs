//! # Serialization of RFC822-like format
//!
//! This module contains types and methods used for serialization of RFC822-like format.
//! The current implementation is very basic.
//! It lacks borrowing the contents of the string, reformatting paragraphs, etc.
//! It is mainly meant for serializing Debian `control` files, however if you have a use
//! case that needs more than that I will be happy to accept a PR.

use std::fmt::Write;
use std::fmt;
use serde::ser;
use unicode_segmentation::UnicodeSegmentation;
use std::borrow::Cow;
pub use error::Error;

pub mod error;

/// Convenience function serializing into `fmt::Writer`
pub fn to_fmt_writer<W: Write, T: ser::Serialize>(writer: W, value: T) -> Result<(), Error> {
    value.serialize(Serializer::new(writer))
}

macro_rules! unsupported_types {
    ($(fn $fn_name:ident$(<$($gen:ident),*>)?(self $(, $arg:ident: $arg_type:ty)*) -> Result<$ret:ty> $(where $type:ty: ?Sized + Serialize)?;)*) => {
        $(
            fn $fn_name$(<$($gen),*>)?(self $(, $arg: $arg_type)*) -> Result<$ret, Self::Error> $(where $type: ?Sized + serde::Serialize)? {
                $(
                    let _ = $arg;
                )*
                Err(Error::unsupported_data_type(stringify!($fn_name)))
            }
        )*
    }
}

/// Serializer backed by `fmt::Writer`
pub struct Serializer<Writer: Write> {
    writer: Writer,
    wrap_long_lines: bool,
}

impl<W> Serializer<W> where W: Write {
    /// Constructs the serializer
    pub fn new(writer: W) -> Self {
        Serializer {
            writer,
            wrap_long_lines: false,
        }
    }

    /// Causes lines longer than 80 characters to be wrapped on word boundaries.
    pub fn wrap_long_lines(mut self, wrap: bool) -> Self {
        self.wrap_long_lines = wrap;
        self
    }
}

impl<W> serde::Serializer for Serializer<W> where W: Write {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SeqSerializer<W>;
    type SerializeTuple = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = MapSerializer<W>;
    type SerializeStruct = StructSerializer<W>;
    type SerializeStructVariant = ser::Impossible<Self::Ok, Self::Error>;

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(StructSerializer {
            writer: self.writer,
            wrap_long_lines: self.wrap_long_lines,
        })
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<(), Self::Error> where T: ?Sized + ser::Serialize {
        value.serialize(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer {
            writer: self.writer,
            field_name: None,
            wrap_long_lines: self.wrap_long_lines,
        })
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SeqSerializer {
            output: self.writer,
            is_empty: true,
            wrap_long_lines: self.wrap_long_lines,
        })
    }

    unsupported_types! {
        fn serialize_bool(self, v: bool) -> Result<()>;
        fn serialize_i8(self, v: i8) -> Result<()>;
        fn serialize_i16(self, v: i16) -> Result<()>;
        fn serialize_i32(self, v: i32) -> Result<()>;
        fn serialize_i64(self, v: i64) -> Result<()>;
        fn serialize_u8(self, v: u8) -> Result<()>;
        fn serialize_u16(self, v: u16) -> Result<()>;
        fn serialize_u32(self, v: u32) -> Result<()>;
        fn serialize_u64(self, v: u64) -> Result<()>;
        fn serialize_f32(self, v: f32) -> Result<()>;
        fn serialize_f64(self, v: f64) -> Result<()>;
        fn serialize_char(self, v: char) -> Result<()>;
        fn serialize_str(self, v: &str) -> Result<()>;
        fn serialize_bytes(self, v: &[u8]) -> Result<()>;
        fn serialize_none(self) -> Result<()>;
        fn serialize_some<T>(self, value: &T) -> Result<()> where T: ?Sized + Serialize;
        fn serialize_unit(self) -> Result<()>; 
        fn serialize_unit_struct(self, name: &'static str) -> Result<()>; 
        fn serialize_unit_variant(self, name: &'static str, variant_index: u32, variant: &'static str) -> Result<()>;
        fn serialize_newtype_variant<T>(self, name: &'static str, variant_index: u32, variant: &'static str, value: &T) -> Result<()>
        where
            T: ?Sized + Serialize;
        fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple>;
        fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct>;
        fn serialize_tuple_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant>;
        fn serialize_struct_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeStructVariant>;
    }
}

struct NonSeqSerializer<Writer: Write> {
    writer: Writer,
    wrap_long_lines: bool,
}

impl<W> serde::Serializer for NonSeqSerializer<W> where W: Write {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = MapSerializer<W>;
    type SerializeStruct = StructSerializer<W>;
    type SerializeStructVariant = ser::Impossible<Self::Ok, Self::Error>;

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(StructSerializer {
            writer: self.writer,
            wrap_long_lines: self.wrap_long_lines,
        })
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<(), Self::Error> where T: ?Sized + ser::Serialize {
        value.serialize(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer {
            writer: self.writer,
            field_name: None,
            wrap_long_lines: self.wrap_long_lines,
        })
    }

    unsupported_types! {
        fn serialize_bool(self, v: bool) -> Result<()>;
        fn serialize_i8(self, v: i8) -> Result<()>;
        fn serialize_i16(self, v: i16) -> Result<()>;
        fn serialize_i32(self, v: i32) -> Result<()>;
        fn serialize_i64(self, v: i64) -> Result<()>;
        fn serialize_u8(self, v: u8) -> Result<()>;
        fn serialize_u16(self, v: u16) -> Result<()>;
        fn serialize_u32(self, v: u32) -> Result<()>;
        fn serialize_u64(self, v: u64) -> Result<()>;
        fn serialize_f32(self, v: f32) -> Result<()>;
        fn serialize_f64(self, v: f64) -> Result<()>;
        fn serialize_char(self, v: char) -> Result<()>;
        fn serialize_str(self, v: &str) -> Result<()>;
        fn serialize_bytes(self, v: &[u8]) -> Result<()>;
        fn serialize_none(self) -> Result<()>;
        fn serialize_some<T>(self, value: &T) -> Result<()> where T: ?Sized + Serialize;
        fn serialize_unit(self) -> Result<()>; 
        fn serialize_unit_struct(self, name: &'static str) -> Result<()>; 
        fn serialize_unit_variant(self, name: &'static str, variant_index: u32, variant: &'static str) -> Result<()>;
        fn serialize_newtype_variant<T>(self, name: &'static str, variant_index: u32, variant: &'static str, value: &T) -> Result<()>
        where
            T: ?Sized + Serialize;
        fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq>;
        fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple>;
        fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct>;
        fn serialize_tuple_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant>;
        fn serialize_struct_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeStructVariant>;
    }
}

/// Serializer used for serializing sequences of records.
///
/// This type is internal and should not be used directly. If you need to refer to it it's best to use
/// `Serializer::SerializeSeq`.
pub struct SeqSerializer<Writer: Write> {
    output: Writer,
    wrap_long_lines: bool,
    is_empty: bool,
}

impl<W> ser::SerializeSeq for SeqSerializer<W> where W: Write {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error> where T: ser::Serialize + ?Sized {
        if !self.is_empty {
            writeln!(self.output).map_err(Error::failed_write)?;
        }
        value.serialize(NonSeqSerializer { writer: &mut self.output, wrap_long_lines: self.wrap_long_lines })
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

/// Internal serializer for structs
pub struct StructSerializer<Writer: Write> {
    writer: Writer,
    wrap_long_lines: bool,
}

impl<W: Write> ser::SerializeStruct for StructSerializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<Self::Ok, Self::Error> where T: ?Sized + ser::Serialize {
        value.serialize(FieldSerializer {
            field_name: key.into(),
            output: &mut self.writer,
            wrap_long_lines: self.wrap_long_lines,
        })?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

/// Internal serializer for maps
// Can't use non-static lifetime because of lack of GAT
pub struct MapSerializer<Writer: Write> {
    writer: Writer,
    field_name: Option<Cow<'static, str>>,
    wrap_long_lines: bool,
}

impl<W: Write> ser::SerializeMap for MapSerializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, value: &T) -> Result<Self::Ok, Self::Error> where T: ?Sized + ser::Serialize {
        value.serialize(KeySerializer {
            key: &mut self.field_name,
        })?;
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<Self::Ok, Self::Error> where T: ?Sized + ser::Serialize {
        value.serialize(FieldSerializer {
            field_name: self.field_name.take().expect("serialize_value() called before serialize_key()"),
            output: &mut self.writer,
            wrap_long_lines: self.wrap_long_lines,
        })?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

fn check_and_write_key(mut output: impl Write, key: &str) -> Result<(), Error> {
    if key.is_empty() {
        return Err(error::ErrorInternal::EmptyKey.into());
    }

    if let Some(pos) = key.find(&[':', '\n'] as &[char]) {
        let c = key[pos..].chars().next().expect("char found at the end - WTF");
        return Err(error::ErrorInternal::InvalidKeyChar { key: key.to_owned(), c, pos, }.into());
    }

    write!(output, "{}: ", key).map_err(Error::failed_write)
}

struct KeySerializer<'a> {
    key: &'a mut Option<Cow<'static, str>>,
}


impl<'a> serde::Serializer for KeySerializer<'a> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = ser::Impossible<Self::Ok, Self::Error>;

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        *self.key = Some(value.to_owned().into());
        Ok(())
    }

    unsupported_types! {
        fn serialize_bool(self, v: bool) -> Result<()>;
        fn serialize_i8(self, v: i8) -> Result<()>;
        fn serialize_i16(self, v: i16) -> Result<()>;
        fn serialize_i32(self, v: i32) -> Result<()>;
        fn serialize_i64(self, v: i64) -> Result<()>;
        fn serialize_u8(self, v: u8) -> Result<()>;
        fn serialize_u16(self, v: u16) -> Result<()>;
        fn serialize_u32(self, v: u32) -> Result<()>;
        fn serialize_u64(self, v: u64) -> Result<()>;
        fn serialize_f32(self, v: f32) -> Result<()>;
        fn serialize_f64(self, v: f64) -> Result<()>;
        fn serialize_char(self, v: char) -> Result<()>;
        fn serialize_bytes(self, v: &[u8]) -> Result<()>;
        fn serialize_unit(self) -> Result<()>; 
        fn serialize_unit_struct(self, name: &'static str) -> Result<()>; 
        fn serialize_unit_variant(self, name: &'static str, variant_index: u32, variant: &'static str) -> Result<()>;
        fn serialize_newtype_variant<T>(self, name: &'static str, variant_index: u32, variant: &'static str, value: &T) -> Result<()>
        where
            T: ?Sized + Serialize;
        fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple>;
        fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct>;
        fn serialize_tuple_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant>;
        fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap>;
        fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct>;
        fn serialize_struct_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeStructVariant>;
        fn serialize_none(self) -> Result<()>;
        fn serialize_some<T>(self, value: &T) -> Result<()> where T: ?Sized + Serialize;
        fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq>;
        fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()> where T: ?Sized + Serialize;
    }
}

#[derive(Copy, Clone)]
enum FieldWriterState {
    FirstLine,
    Neutral,
    EndedWithNewline,
}

struct FieldWriter<Writer: Write> {
    output: Writer,
    wrap_long_lines: bool,
    state: FieldWriterState,
}

impl<W: Write> FieldWriter<W> {
    fn new(output: W, wrap_long_lines: bool) -> Self {
        FieldWriter {
            output,
            wrap_long_lines,
            state: FieldWriterState::FirstLine,
        }
    }

    fn finish(&mut self) -> fmt::Result {
        if let FieldWriterState::EndedWithNewline = self.state {
            Ok(())
        } else {
            self.output.write_str("\n")
        }
    }
}

impl<W: Write> Write for FieldWriter<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if s.is_empty() {
            return Ok(())
        }

        let mut iter = s.split('\n');
        let line = iter.next().expect("split() returned an empty iterator");
        match self.state {
            // We must not wrap the first line
            FieldWriterState::FirstLine => self.output.write_str(line)?,
            FieldWriterState::EndedWithNewline if line.is_empty() => self.output.write_str(".")?,
            FieldWriterState::EndedWithNewline | FieldWriterState::Neutral if self.wrap_long_lines => write_wraped(&mut self.output, line)?,
            FieldWriterState::EndedWithNewline | FieldWriterState::Neutral => self.output.write_str(line)?,
        }

        let mut contained_newline = false;
        let mut iter = iter.peekable();
        while let Some(line) = iter.next() {
            contained_newline = true;
            self.output.write_str("\n ")?;
            if line.is_empty() {
                // if it's last we don't know what follows
                if iter.peek().is_some() {
                    self.output.write_str(".")?;
                }
            } else if self.wrap_long_lines {
                write_wraped(&mut self.output, line)?;
            } else {
                self.output.write_str(line)?;
            }
        }

        match (self.state, contained_newline) {
            (FieldWriterState::FirstLine, false) => (),
            _ if s.ends_with('\n') => self.state = FieldWriterState::EndedWithNewline,
            _ => self.state = FieldWriterState::Neutral,
        }

        Ok(())
    }
}

struct FieldSerializer<Writer: Write> {
    field_name: Cow<'static, str>,
    output: Writer,
    wrap_long_lines: bool,
}

fn write_wraped<W: Write>(mut out: W, line: &str) -> std::fmt::Result {
    let mut written = 1;

    for word in line.split_word_bounds() {
        let word_len = word.graphemes(true).count();
        if written + word_len > 80 {
            out.write_str("\n ")?;
            written = 1;
        }

        if !(word.trim().len() == 0 && written == 1) {
            out.write_str(word)?;
            written += word_len;
        }
    }
    Ok(())
}

impl<W> serde::Serializer for FieldSerializer<W> where W: Write {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SubSeqSerializer<W>;
    type SerializeTuple = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = ser::Impossible<Self::Ok, Self::Error>;

    fn collect_str<T: fmt::Display + ?Sized>(mut self, value: &T) -> Result<Self::Ok, Self::Error> {
        check_and_write_key(&mut self.output, &self.field_name)?;
        let mut writer = FieldWriter::new(&mut self.output, self.wrap_long_lines);
        (move || {
            write!(writer, "{}", value)?;
            writer.finish()
        })().map_err(Error::failed_write)
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        self.collect_str(value)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error> where T: ?Sized + ser::Serialize {
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SubSeqSerializer {
            output: self.output,
            state: SubSeqSerializerState::Empty { field_name: self.field_name, },
        })
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<(), Self::Error> where T: ?Sized + ser::Serialize {
        value.serialize(self)
    }

    fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, variant: &'static str) -> Result<(), Self::Error> {
        self.serialize_str(variant)
    }

    unsupported_types! {
        fn serialize_bool(self, v: bool) -> Result<()>;
        fn serialize_i8(self, v: i8) -> Result<()>;
        fn serialize_i16(self, v: i16) -> Result<()>;
        fn serialize_i32(self, v: i32) -> Result<()>;
        fn serialize_i64(self, v: i64) -> Result<()>;
        fn serialize_u8(self, v: u8) -> Result<()>;
        fn serialize_u16(self, v: u16) -> Result<()>;
        fn serialize_u32(self, v: u32) -> Result<()>;
        fn serialize_u64(self, v: u64) -> Result<()>;
        fn serialize_f32(self, v: f32) -> Result<()>;
        fn serialize_f64(self, v: f64) -> Result<()>;
        fn serialize_char(self, v: char) -> Result<()>;
        fn serialize_bytes(self, v: &[u8]) -> Result<()>;
        fn serialize_unit(self) -> Result<()>; 
        fn serialize_unit_struct(self, name: &'static str) -> Result<()>; 
        fn serialize_newtype_variant<T>(self, name: &'static str, variant_index: u32, variant: &'static str, value: &T) -> Result<()>
        where
            T: ?Sized + Serialize;
        fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple>;
        fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct>;
        fn serialize_tuple_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant>;
        fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap>;
        fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct>;
        fn serialize_struct_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeStructVariant>;
    }
}

#[derive(Clone)]
enum SubSeqSerializerState {
    Empty { field_name: Cow<'static, str>, },
    NonEmpty { indent: usize, },
}

struct SubSeqSerializer<Writer: Write> {
    output: Writer,
    state: SubSeqSerializerState,
}

impl<W> ser::SerializeSeq for SubSeqSerializer<W> where W: Write {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error> where T: ser::Serialize + ?Sized {
        use SubSeqSerializerState::*;

        (|| -> Result<_, _> {
            match &self.state {
                Empty { field_name, } => {
                    write!(self.output, "{}: ", field_name)?;
                    let indent = field_name.graphemes(true).count() + 2;
                    self.state = NonEmpty { indent, };
                },
                NonEmpty { indent, } => {
                    self.output.write_str(",\n")?;
                    for _ in 0..*indent {
                        self.output.write_char(' ')?;
                    }
                }
            }
            Ok(())
        })().map_err(Error::failed_write)?;
        value.serialize(StringSerializer(&mut self.output))
    }

    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        match self.state {
            SubSeqSerializerState::NonEmpty { .. } => self.output.write_char('\n'),
            SubSeqSerializerState::Empty { .. } => Ok(())
        }.map_err(Error::failed_write)
    }
}

struct StringSerializer<Writer: Write>(Writer);

impl<W> serde::Serializer for StringSerializer<W> where W: Write {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = ser::Impossible<Self::Ok, Self::Error>;

    fn serialize_str(mut self, value: &str) -> Result<Self::Ok, Self::Error> {
        self.0.write_str(value).map_err(Error::failed_write)
    }

    fn collect_str<T>(mut self, value: &T) -> Result<Self::Ok, Self::Error> where T: ?Sized + std::fmt::Display {
        write!(self.0, "{}", value).map_err(Error::failed_write)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<(), Self::Error> where T: ?Sized + ser::Serialize {
        value.serialize(self)
    }

    fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, variant: &'static str) -> Result<(), Self::Error> {
        self.serialize_str(variant)
    }

    unsupported_types! {
        fn serialize_bool(self, v: bool) -> Result<()>;
        fn serialize_i8(self, v: i8) -> Result<()>;
        fn serialize_i16(self, v: i16) -> Result<()>;
        fn serialize_i32(self, v: i32) -> Result<()>;
        fn serialize_i64(self, v: i64) -> Result<()>;
        fn serialize_u8(self, v: u8) -> Result<()>;
        fn serialize_u16(self, v: u16) -> Result<()>;
        fn serialize_u32(self, v: u32) -> Result<()>;
        fn serialize_u64(self, v: u64) -> Result<()>;
        fn serialize_f32(self, v: f32) -> Result<()>;
        fn serialize_f64(self, v: f64) -> Result<()>;
        fn serialize_char(self, v: char) -> Result<()>;
        fn serialize_bytes(self, v: &[u8]) -> Result<()>;
        fn serialize_none(self) -> Result<()>; 
        fn serialize_some<T>(self, value: &T) -> Result<()>
        where
            T: ?Sized + Serialize;
        fn serialize_unit(self) -> Result<()>; 
        fn serialize_unit_struct(self, name: &'static str) -> Result<()>; 
        fn serialize_newtype_variant<T>(self, name: &'static str, variant_index: u32, variant: &'static str, value: &T) -> Result<()>
        where
            T: ?Sized + Serialize;
        fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq>;
        fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple>;
        fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct>;
        fn serialize_tuple_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant>;
        fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap>;
        fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct>;
        fn serialize_struct_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeStructVariant>;
    }
}

#[cfg(test)]
mod tests {
    use super::Serializer;
    use serde::Serialize;
    use serde::Serializer as SerdeSerializer;
    use std::fmt::Write;

    #[test]
    fn error() {
        assert!(Serializer::new(String::new()).serialize_str("foo").is_err());
    }

    #[test]
    fn empty() {
        #[derive(serde_derive::Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Foo {}

        let mut out = String::new();
        Foo {}.serialize(Serializer::new(&mut out)).expect("Failed to serialize");
        assert_eq!(out, "");
    }

    #[test]
    fn single() {
        #[derive(serde_derive::Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Foo {
            bar: &'static str,
        }

        let mut out = String::new();
        Foo { bar: "baz" }.serialize(Serializer::new(&mut out)).expect("Failed to serialize");
        assert_eq!(out, "Bar: baz\n");
    }

    #[test]
    fn two() {
        #[derive(serde_derive::Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Foo {
            bar: &'static str,
            baz: &'static str,
        }

        let mut out = String::new();
        Foo { bar: "bar value", baz: "baz value" }.serialize(Serializer::new(&mut out)).expect("Failed to serialize");
        assert_eq!(out, "Bar: bar value\nBaz: baz value\n");
    }

    #[test]
    fn long_begin() {
        #[derive(serde_derive::Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Foo {
            bar: &'static str,
        }

        let mut out = String::new();
        Foo { bar: "Insanely long string meant for testing, that will be over eighty characters long, I believe." }.serialize(Serializer::new(&mut out)).expect("Failed to serialize");
        assert_eq!(out, "Bar: Insanely long string meant for testing, that will be over eighty characters long, I believe.\n");
    }

    #[test]
    fn long_body() {
        #[derive(serde_derive::Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Foo {
            bar: &'static str,
        }

        let mut out = String::new();
        Foo { bar: "Begin\nInsanely long string meant for testing, that will be over eighty characters long, I believe." }
            .serialize(Serializer::new(&mut out).wrap_long_lines(true)).expect("Failed to serialize");
        assert_eq!(out, "Bar: Begin\n Insanely long string meant for testing, that will be over eighty characters \n long, I believe.\n");
    }

    #[test]
    fn multiline() {
        #[derive(serde_derive::Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Foo {
            bar: &'static str,
        }

        let mut out = String::new();
        Foo { bar: "first line\nsecond line" }.serialize(Serializer::new(&mut out)).expect("Failed to serialize");
        assert_eq!(out, "Bar: first line\n second line\n");
    }

    #[test]
    fn empty_line() {
        #[derive(serde_derive::Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Foo {
            bar: &'static str,
        }

        let mut out = String::new();
        Foo { bar: "begin\nfirst line\n\nsecond line" }.serialize(Serializer::new(&mut out)).expect("Failed to serialize");
        assert_eq!(out, "Bar: begin\n first line\n .\n second line\n");
    }

    #[test]
    fn none() {
        #[derive(serde_derive::Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Foo {
            bar: Option<&'static str>,
        }

        let mut out = String::new();
        Foo { bar: None }.serialize(Serializer::new(&mut out)).expect("Failed to serialize");
        assert_eq!(out, "");
    }

    #[test]
    fn some() {
        #[derive(serde_derive::Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Foo {
            bar: Option<&'static str>,
        }

        let mut out = String::new();
        Foo { bar: Some("baz") }.serialize(Serializer::new(&mut out)).expect("Failed to serialize");
        assert_eq!(out, "Bar: baz\n");
    }

    #[test]
    fn seq_empty() {
        #[derive(serde_derive::Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Foo {
            bar: &'static [&'static str],
        }

        let mut out = String::new();
        Foo { bar: &[] }.serialize(Serializer::new(&mut out)).expect("Failed to serialize");
        assert_eq!(out, "");
    }

    #[test]
    fn seq_single() {
        #[derive(serde_derive::Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Foo {
            bar: &'static [&'static str],
        }

        let mut out = String::new();
        Foo { bar: &["baz"] }.serialize(Serializer::new(&mut out)).expect("Failed to serialize");
        assert_eq!(out, "Bar: baz\n");
    }

    #[test]
    fn seq_two() {
        #[derive(serde_derive::Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Foo {
            bar: &'static [&'static str],
        }

        let mut out = String::new();
        Foo { bar: &["baz", "bitcoin"] }.serialize(Serializer::new(&mut out)).expect("Failed to serialize");
        assert_eq!(out, "Bar: baz,\n     bitcoin\n");
    }

    #[test]
    fn field_writer_empty() {
        let mut output = String::new();
        let mut writer = super::FieldWriter::new(&mut output, false);
        write!(writer, "").unwrap();
        writer.finish().unwrap();
        assert_eq!(output, "\n");
    }

    #[test]
    fn field_writer_no_newline() {
        let mut output = String::new();
        let mut writer = super::FieldWriter::new(&mut output, false);
        write!(writer, "satoshi").unwrap();
        writer.finish().unwrap();
        assert_eq!(output, "satoshi\n");
    }

    #[test]
    fn field_writer_single_newline() {
        let mut output = String::new();
        let mut writer = super::FieldWriter::new(&mut output, false);
        write!(writer, "satoshi\nnakamoto").unwrap();
        writer.finish().unwrap();
        assert_eq!(output, "satoshi\n nakamoto\n");
    }

    #[test]
    fn field_writer_two_newlines() {
        let mut output = String::new();
        let mut writer = super::FieldWriter::new(&mut output, false);
        write!(writer, "satoshi\nnakamoto\nbitcoin").unwrap();
        writer.finish().unwrap();
        assert_eq!(output, "satoshi\n nakamoto\n bitcoin\n");
    }

    #[test]
    fn field_writer_split_first_line() {
        let mut output = String::new();
        let mut writer = super::FieldWriter::new(&mut output, false);
        write!(writer, "satoshi").unwrap();
        write!(writer, " nakamoto").unwrap();
        writer.finish().unwrap();
        assert_eq!(output, "satoshi nakamoto\n");
    }

    #[test]
    fn field_writer_split_before_first_line_end() {
        let mut output = String::new();
        let mut writer = super::FieldWriter::new(&mut output, false);
        write!(writer, "satoshi").unwrap();
        write!(writer, "\nnakamoto").unwrap();
        writer.finish().unwrap();
        assert_eq!(output, "satoshi\n nakamoto\n");
    }

    #[test]
    fn field_writer_split_after_first_line_end() {
        let mut output = String::new();
        let mut writer = super::FieldWriter::new(&mut output, false);
        write!(writer, "satoshi\n").unwrap();
        write!(writer, "nakamoto").unwrap();
        writer.finish().unwrap();
        assert_eq!(output, "satoshi\n nakamoto\n");
    }

    #[test]
    fn field_writer_split_second_line() {
        let mut output = String::new();
        let mut writer = super::FieldWriter::new(&mut output, false);
        write!(writer, "satoshi nakamoto\ninvented").unwrap();
        write!(writer, " bitcoin").unwrap();
        writer.finish().unwrap();
        assert_eq!(output, "satoshi nakamoto\n invented bitcoin\n");
    }

    #[test]
    fn field_writer_empty_line() {
        let mut output = String::new();
        let mut writer = super::FieldWriter::new(&mut output, false);
        write!(writer, "satoshi nakamoto\n\ninvented bitcoin").unwrap();
        writer.finish().unwrap();
        assert_eq!(output, "satoshi nakamoto\n .\n invented bitcoin\n");
    }

    #[test]
    fn field_writer_split_before_empty_line() {
        let mut output = String::new();
        let mut writer = super::FieldWriter::new(&mut output, false);
        write!(writer, "satoshi nakamoto").unwrap();
        write!(writer, "\n\ninvented bitcoin").unwrap();
        writer.finish().unwrap();
        assert_eq!(output, "satoshi nakamoto\n .\n invented bitcoin\n");
    }

    #[test]
    fn field_writer_split_in_empty_line() {
        let mut output = String::new();
        let mut writer = super::FieldWriter::new(&mut output, false);
        write!(writer, "satoshi nakamoto\n").unwrap();
        write!(writer, "\ninvented bitcoin").unwrap();
        writer.finish().unwrap();
        assert_eq!(output, "satoshi nakamoto\n .\n invented bitcoin\n");
    }

    #[test]
    fn field_writer_split_after_empty_line() {
        let mut output = String::new();
        let mut writer = super::FieldWriter::new(&mut output, false);
        write!(writer, "satoshi nakamoto\n\n").unwrap();
        write!(writer, "invented bitcoin").unwrap();
        writer.finish().unwrap();
        assert_eq!(output, "satoshi nakamoto\n .\n invented bitcoin\n");
    }

    #[test]
    fn field_writer_split_empty_line_twice1() {
        let mut output = String::new();
        let mut writer = super::FieldWriter::new(&mut output, false);
        write!(writer, "satoshi nakamoto\n").unwrap();
        write!(writer, "\n").unwrap();
        write!(writer, "invented bitcoin").unwrap();
        writer.finish().unwrap();
        assert_eq!(output, "satoshi nakamoto\n .\n invented bitcoin\n");
    }

    #[test]
    fn field_writer_split_empty_line_twice2() {
        let mut output = String::new();
        let mut writer = super::FieldWriter::new(&mut output, false);
        write!(writer, "satoshi nakamoto").unwrap();
        write!(writer, "\n").unwrap();
        write!(writer, "\ninvented bitcoin").unwrap();
        writer.finish().unwrap();
        assert_eq!(output, "satoshi nakamoto\n .\n invented bitcoin\n");
    }

    #[test]
    fn field_writer_multi_split_empty_line_three_times() {
        let mut output = String::new();
        let mut writer = super::FieldWriter::new(&mut output, false);
        write!(writer, "satoshi nakamoto").unwrap();
        write!(writer, "\n").unwrap();
        write!(writer, "\n").unwrap();
        write!(writer, "invented bitcoin").unwrap();
        writer.finish().unwrap();
        assert_eq!(output, "satoshi nakamoto\n .\n invented bitcoin\n");
    }

    #[test]
    fn serialize_unit_variant() {
        #[derive(serde_derive::Serialize)]
        #[serde(rename_all = "snake_case")]
        enum Foo {
            Bar,
        }

        #[derive(serde_derive::Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Baz {
            foo: Foo,
        }

        let mut out = String::new();
        let baz = Baz { foo: Foo::Bar, };
        baz.serialize(Serializer::new(&mut out)).expect("Failed to serialize");
        assert_eq!(out, "Foo: bar\n");
    }
}
