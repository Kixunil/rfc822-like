//! # Deserialization of RFC822-like format
//!
//! This module contains types and methods used for deserialization of RFC822-like format.
//! The current implementation is very basic.
//! It lacks borrowing the contents of the string, etc.
//! It is mainly meant for quick look at `apt` metadata, however if you have a use case that needs
//! more than that I will be happy to accept a PR.

use serde::de::{Visitor, MapAccess, SeqAccess, DeserializeSeed, IntoDeserializer};
use std::io;
use error::ErrorInner;
pub use error::Error;

pub mod error;

/// Deserializes a single record or multiple records separated by empty lines.
///
/// Note that RFC822 is **not** self-describing, thus you must specify the type being deserialized.
/// That means something like `serde_json::Value` can not be deserialized.
///
/// The allowed types are:
///
/// * map
/// * struct
/// * sequence of maps with str-deserializable keys
/// * sequence of structs
///
/// Further, values of maps and types of fields of structs must be either deserializable from `str`
/// or sequence of `str`.
///
/// # Example
/// 
/// ```
/// use rfc822_like::de::Deserializer;
/// use serde::Deserialize;
///
/// let input = "Package: foo
/// Description: The Foo
///
/// Package: bar
/// Description: The Bar
/// ";
///
/// let mut reader = input.as_bytes();
///
/// #[derive(Debug, Eq, PartialEq, serde_derive::Deserialize)]
/// #[serde(rename_all = "PascalCase")]
/// struct Record {
///     package: String,
///     description: String,
/// }
/// 
/// let expected = vec![
///     Record {
///         package: "foo".to_owned(),
///         description:"The Foo".to_owned(),
///     },
///     Record {
///         package: "bar".to_owned(),
///         description: "The Bar".to_owned(),
///     },
/// ];
///
/// let deserialized = <Vec<Record>>::deserialize(Deserializer::new(&mut reader)).unwrap();
/// assert_eq!(deserialized, expected);
/// ```
///
/// Additionally, sequences of strings in fields are supported:
///
/// ```
/// use rfc822_like::de::Deserializer;
/// use serde::Deserialize;
///
/// let input = "Depends: bitcoind, python (>= 3.0.0)\n";
/// let mut reader = input.as_bytes();
///
/// #[derive(Debug, Eq, PartialEq, serde_derive::Deserialize)]
/// #[serde(rename_all = "PascalCase")]
/// struct Package {
///     depends: Vec<String>,
/// }
///
/// let expected = Package {
///     depends: vec!["bitcoind".to_owned(), "python (>= 3.0.0)".to_owned()]
/// };
///
/// let deserialized = Package::deserialize(Deserializer::new(&mut reader)).unwrap();
/// assert_eq!(deserialized, expected);
/// ```
pub struct Deserializer<R: io::BufRead> {
    state: DeserializerState<R>,
}

impl<'de, R: io::BufRead> Deserializer<R> {
    /// Creates a `Deserializer` from bufferred reader.
    pub fn new(reader: R) -> Self {
        Deserializer {
            state: DeserializerState::new(reader),
        }
    }
}

impl<'de, R: io::BufRead> serde::Deserializer<'de> for Deserializer<R> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(ErrorInner::AmbiguousType.into())
    }

    fn deserialize_seq<V: Visitor<'de>>(mut self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_seq(Seq(&mut self.state))
    }

    fn deserialize_map<V: Visitor<'de>>(mut self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_map(&mut self.state)
    }

    fn deserialize_struct<V: Visitor<'de>>(mut self, _name: &'static str, _fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_map(&mut self.state)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct tuple
        tuple_struct enum identifier ignored_any
    }
}

struct Seq<'a, R: io::BufRead>(&'a mut DeserializerState<R>);

impl<'a, 'de, R: io::BufRead> SeqAccess<'de> for Seq<'a, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> where T: DeserializeSeed<'de> {
        if self.0.eof {
            return Ok(None);
        }

        match seed.deserialize(SingleRecordDeserializer::new(self.0)) {
            Ok(value) => Ok(Some(value)),
            Err(_) if self.0.empty => Ok(None),
            Err(error) => Err(error),
        }
    }
}

struct SingleRecordDeserializer<'a, R: io::BufRead> {
    state: &'a mut DeserializerState<R>,
}

impl<'a, R: io::BufRead> SingleRecordDeserializer<'a, R> {
    fn new(state: &'a mut DeserializerState<R>) -> Self {
        SingleRecordDeserializer {
            state,
        }
    }
}

struct DeserializerState<R: io::BufRead> {
    reader: R,
    buf: String,
    line: usize,
    eof: bool,
    empty: bool,
}

impl<R: io::BufRead> DeserializerState<R> {
    fn new(reader: R) -> Self {
        DeserializerState {
            reader,
            buf: String::new(),
            line: 0,
            eof: false,
            empty: true,
        }
    }

    fn get_key(&mut self) -> Result<Option<&str>, Error> {
        if self.buf.is_empty() {
            match self.reader.read_line(&mut self.buf).map_err(ErrorInner::from)? {
                0 => {
                    self.eof = true;
                    return Ok(None)
                },
                // just \n
                1 => {
                    self.buf.clear();
                    self.empty = true;
                    self.line += 1;
                    return Ok(None);
                },
                _ => self.line += 1,
            }
        }
        if self.buf == "\n" {
            self.buf.clear();
            self.empty = true;
            return Ok(None);
        }

        match self.buf.find(':') {
            Some(pos) => {
                self.empty = false;
                Ok(Some(&self.buf[..pos]))
            },
            None => {
                Err(ErrorInner::MissingColon(self.line).into())
            },
        }
    }

    fn get_value(&mut self) -> Result<(&str, usize), Error> {
        let mut pos = self.buf.len();
        loop {
            let amount = self.reader.read_line(&mut self.buf).map_err(ErrorInner::from)?;
            if amount == 0 || !(self.buf[pos..].starts_with(' ') || self.buf[pos..].starts_with('\t')) {
                break;
            }
            pos += amount;
        }
        let begin = self.buf.find(':').expect("The caller didn't handle the error") + 1;
        Ok((self.buf[begin..pos].trim(), pos))
    }

    fn clear_buf(&mut self, pos: usize) {
        self.buf.replace_range(0..pos, "");
    }
}

impl<'a, 'de, R: io::BufRead> MapAccess<'de> for &'a mut DeserializerState<R> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error> where K: DeserializeSeed<'de> {
        self
            .get_key()?
            .map(move |key| seed.deserialize(KeyDeserializer(key)))
            .transpose()
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error> where V: DeserializeSeed<'de> {
        let (value, pos) = self
            .get_value()?;
        let result = seed.deserialize(ValueDeserializer(value));
        self.clear_buf(pos);
        result
    }
}

impl<'a, 'de, R: io::BufRead> serde::Deserializer<'de> for SingleRecordDeserializer<'a, R> {
    type Error = Error;

    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_map(&mut self.state)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct KeyDeserializer<'a>(&'a str);

impl<'a, 'de> serde::Deserializer<'de> for KeyDeserializer<'a> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_str(self.0)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct ValueDeserializer<'a>(&'a str);

impl<'a, 'de> serde::Deserializer<'de> for ValueDeserializer<'a> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        if self.0.contains("\n ") {
            let mut string = String::with_capacity(self.0.len());
            let mut iter = self.0.split('\n');
            string.push_str(iter.next().expect("split didn't return any item"));

            for line in iter {
                string.push('\n');
                if line != " ." {
                    string.push_str(line.trim_start());
                }
            }

            visitor.visit_string(string)
        } else {
            visitor.visit_str(self.0)
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        // Possible optimization: we could instad have a visitor that points to appropriate
        // position inside the buffer and removes the beginning if called here, or turns it into a
        // slice if called above.
        self.deserialize_str(visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_seq(StrSeq(self.0.split(',')))
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_some(self)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(self.0.into_deserializer())
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char
        bytes byte_buf unit unit_struct newtype_struct tuple
        tuple_struct map struct identifier ignored_any
    }
}

struct StrDeserializer<'a>(&'a str);

impl<'a, 'de> serde::Deserializer<'de> for StrDeserializer<'a> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error> where V: Visitor<'de> {
        visitor.visit_str(self.0)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct StrSeq<'a>(std::str::Split<'a, char>);

impl<'a, 'de> SeqAccess<'de> for StrSeq<'a> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> where T: DeserializeSeed<'de> {
        self.0.next().map(|item| seed.deserialize(StrDeserializer(item.trim()))).transpose()
    }

    // fn size_hint(&self) -> Option<usize> { ... } not specialized for split
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    #[test]
    fn test_single() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            name: String,
        }

        let mut input = b"Name: bitcoin" as &[u8];
        let deserializer = super::Deserializer::new(&mut input);
        let packages = <Vec<Record>>::deserialize(deserializer).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "bitcoin");
    }

    #[test]
    fn test_two_fields() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            name: String,
            summary: String,
        }

        let mut input = b"Name: bitcoin\nSummary: Magic Internet Money" as &[u8];
        let deserializer = super::Deserializer::new(&mut input);
        let packages = <Vec<Record>>::deserialize(deserializer).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "bitcoin");
        assert_eq!(packages[0].summary, "Magic Internet Money");
    }

    #[test]
    fn test_two_packages() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            name: String,
        }

        let mut input = b"Name: bitcoin\n\nName: lightning" as &[u8];
        let deserializer = super::Deserializer::new(&mut input);
        let packages = <Vec<Record>>::deserialize(deserializer).unwrap();
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "bitcoin");
        assert_eq!(packages[1].name, "lightning");
    }

    #[test]
    fn test_two_packages_two_fields() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            name: String,
            summary: String,
        }

        let mut input = b"Name: bitcoin\nSummary: Magic Internet Money\n\nName: lightning\nSummary: The payment rail" as &[u8];
        let deserializer = super::Deserializer::new(&mut input);
        let packages = <Vec<Record>>::deserialize(deserializer).unwrap();
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "bitcoin");
        assert_eq!(packages[0].summary, "Magic Internet Money");
        assert_eq!(packages[1].name, "lightning");
        assert_eq!(packages[1].summary, "The payment rail");
    }

    #[test]
    fn test_single_newline_end() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            name: String,
        }

        let mut input = b"Name: bitcoin\n" as &[u8];
        let deserializer = super::Deserializer::new(&mut input);
        let packages = <Vec<Record>>::deserialize(deserializer).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "bitcoin");
    }

    #[test]
    fn test_two_packages_newline_end() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            name: String,
        }

        let mut input = b"Name: bitcoin\n\nName: lightning\n" as &[u8];
        let deserializer = super::Deserializer::new(&mut input);
        let packages = <Vec<Record>>::deserialize(deserializer).unwrap();
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "bitcoin");
        assert_eq!(packages[1].name, "lightning");
    }

    #[test]
    fn test_two_packages_two_fields_newline_end() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            name: String,
            summary: String,
        }

        let mut input = b"Name: bitcoin\nSummary: Magic Internet Money\n\nName: lightning\nSummary: The payment rail\n" as &[u8];
        let deserializer = super::Deserializer::new(&mut input);
        let packages = <Vec<Record>>::deserialize(deserializer).unwrap();
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "bitcoin");
        assert_eq!(packages[0].summary, "Magic Internet Money");
        assert_eq!(packages[1].name, "lightning");
        assert_eq!(packages[1].summary, "The payment rail");
    }

    #[test]
    fn test_single_package_double_newline_end() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            name: String,
        }

        let mut input = b"Name: bitcoin\n\n" as &[u8];
        let deserializer = super::Deserializer::new(&mut input);
        let packages = <Vec<Record>>::deserialize(deserializer).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "bitcoin");
    }

    #[test]
    fn test_no_space() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            name: String,
        }

        let mut input = b"Name:bitcoin" as &[u8];
        let deserializer = super::Deserializer::new(&mut input);
        let packages = <Vec<Record>>::deserialize(deserializer).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "bitcoin");
    }

    #[test]
    fn test_val_on_new_line() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            name: String,
        }

        let mut input = b"Name:\n bitcoin" as &[u8];
        let deserializer = super::Deserializer::new(&mut input);
        let packages = <Vec<Record>>::deserialize(deserializer).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "bitcoin");
    }

    #[test]
    fn test_seq_single() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            name: Vec<String>,
        }

        let mut input = b"Name: bitcoin" as &[u8];
        let deserializer = super::Deserializer::new(&mut input);
        let packages = <Vec<Record>>::deserialize(deserializer).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name.len(), 1);
        assert_eq!(packages[0].name[0], "bitcoin");
    }

    #[test]
    fn test_seq_two() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            name: Vec<String>,
        }

        let mut input = b"Name: bitcoin,lightning" as &[u8];
        let deserializer = super::Deserializer::new(&mut input);
        let packages = <Vec<Record>>::deserialize(deserializer).unwrap();
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name.len(), 2);
        assert_eq!(packages[0].name[0], "bitcoin");
        assert_eq!(packages[0].name[1], "lightning");
    }

    #[test]
    fn test_extra_fields() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            name: String,
        }

        let mut input = b"Name:\n bitcoin\nVersion: 0.21.1" as &[u8];
        let deserializer = super::Deserializer::new(&mut input);
        let package = Record::deserialize(deserializer).unwrap();
        assert_eq!(package.name, "bitcoin");
    }

    #[test]
    fn test_multiline() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            description: String,
        }

        let mut input = b"Description:\n A very nice package\n This package is very nice\n because it has a multi-line\n description." as &[u8];
        let deserializer = super::Deserializer::new(&mut input);
        let package = Record::deserialize(deserializer).unwrap();
        assert_eq!(package.description, "A very nice package\nThis package is very nice\nbecause it has a multi-line\ndescription.");
    }

    #[test]
    fn test_multiparagraph() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            description: String,
        }

        let mut input = b"Description:\n A very nice package\n This package is very nice\n because it has a multi-line\n description.\n .\n It also has another paragraph in\n the description.\n .\n And another one." as &[u8];
        let deserializer = super::Deserializer::new(&mut input);
        let package = Record::deserialize(deserializer).unwrap();
        assert_eq!(package.description, "A very nice package\nThis package is very nice\nbecause it has a multi-line\ndescription.\n\nIt also has another paragraph in\nthe description.\n\nAnd another one.");
    }

    #[test]
    #[cfg(feature = "live_test")]
    fn live() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            #[allow(dead_code)]
            package: String,
            #[allow(dead_code)]
            description: String,
        }

        let dir = std::fs::read_dir("/var/lib/apt/lists").unwrap();
        for entry in dir {
            let entry = entry.unwrap();
            match entry.path().to_str() {
                None => continue,
                Some(path) if !path.ends_with("_Records") => continue,
                Some(_) => (),
            }

            let file = std::fs::File::open(entry.path()).unwrap();
            let deserializer = super::Deserializer::new(std::io::BufReader::new(file));
            <Vec<Record>>::deserialize(deserializer).unwrap_or_else(|error| panic!("Failed to parse {}: {:?}", entry.path().display(), error));
        }
    }

    #[test]
    fn test_option_none() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            name: String,
            description: Option<String>,
        }

        let mut input = b"Name: bitcoin" as &[u8];
        let deserializer = super::Deserializer::new(&mut input);
        let package = Record::deserialize(deserializer).unwrap();
        assert_eq!(package.name, "bitcoin");
        assert!(package.description.is_none());
    }

    #[test]
    fn test_option_some() {
        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Record {
            name: String,
            description: Option<String>,
        }

        let mut input = b"Name: bitcoin\nDescription: The Internet of Money" as &[u8];
        let deserializer = super::Deserializer::new(&mut input);
        let package = Record::deserialize(deserializer).unwrap();
        assert_eq!(package.name, "bitcoin");
        assert_eq!(package.description, Some("The Internet of Money".to_owned()));
    }

    #[test]
    fn test_deserialize_unit_variant() {
        #[derive(serde_derive::Deserialize, PartialEq, Eq, Debug)]
        #[serde(rename_all = "snake_case")]
        enum Foo {
            Bar,
        }

        #[derive(serde_derive::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Baz {
            foo: Foo,
        }

        let mut input = b"Foo: bar\n" as &[u8];
        let deserializer = super::Deserializer::new(&mut input);
        let baz = Baz::deserialize(deserializer).unwrap();
        assert_eq!(baz.foo, Foo::Bar);
    }
}
