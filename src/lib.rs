//! A library for handling RFC822-like format used in Debian
//!
//! This crate implements the file format inpired by RFC822 that is used in Debian packages.
//! It is the format of the `debian/control` files in packages and `Packages` file of `apt`.
//! It is called `rfc822-like` instead of just `rfc822` because Debian does not claim to implement RFC822 exactly
//! and this crate is focused on working with Debian tools, not parsing exact RFC822 file format.
//! Frankly, I didn't even bother to read RFC822 itself.
//! 
//! If you need to strictly parse RFC822, I suggest you to fork this crate and fix whatever differences there are.
//! I'm not interested in maintaining strict RFC822 crate, so don't send PRs for that but I'm willing to put
//! common pieces into their own crate.
//! If you're interested in this approach feel free to file a PR (or ask beforehand if you have questions).
//! 
//! Note that this crate is currently not optimized for performance.
//! There are multiple places where allocation could be avoided and other optimizations may be missing.
//! It's absolutely fine for my own use cases, and probably will be for yours as well.
//! If you need it to be faster or just want to have fun improving its performance I'll be happy to accept PRs.
//! 
//! The API is currently not set in stone and may change over time.
//! Basic steps to minimize the impact of changes were taken (e.g. encapsulation of `Error` type).
//! The crate also currently lacks serialization but it will be implemented eventually.
//! Feel free to send PRs!
//!
//! Check [`Deserializer`] type for deserialization API reference and examples.
//! Check [`Seserializer`] type for serialization API reference and examples.

pub mod de;
pub mod ser;

pub use de::Deserializer;
pub use ser::Serializer;

use serde::{Serialize, Deserialize};
use std::{io, fmt};
use std::path::{Path, PathBuf};
use de::error::ReadFileError;

pub fn from_reader<T: for<'a> Deserialize<'a>, R: io::BufRead>(reader: R) -> Result<T, de::Error> {
    T::deserialize(Deserializer::new(reader))
}

pub fn from_file<T: for<'a> Deserialize<'a>, P: AsRef<Path> + Into<PathBuf>>(path: P) -> Result<T, ReadFileError> {
    let file = match std::fs::File::open(&path) {
        Ok(file) => file,
        Err(error) => return Err(ReadFileError::Open { path: path.into(), error, })
    };
    let reader = io::BufReader::new(file);
    T::deserialize(Deserializer::new(reader)).map_err(|error| ReadFileError::Load { path: path.into(), error, })
}

pub fn from_bytes<'a, T: Deserialize<'a>>(mut bytes: &'a [u8]) -> Result<T, de::Error> {
    T::deserialize(Deserializer::new(&mut bytes))
}

pub fn from_str<'a, T: Deserialize<'a>>(s: &'a str) -> Result<T, de::Error> {
    from_bytes(s.as_bytes())
}

pub fn to_fmt_writer<T: Serialize, W: fmt::Write>(writer: W, value: &T) -> Result<(), ser::Error> {
    value.serialize(Serializer::new(writer))
}

pub fn to_writer<T: Serialize, W: io::Write>(writer: W, value: &T) -> Result<(), ser::Error> {
    fmt2io::write(writer, |writer| to_fmt_writer(writer, value).map(Ok).or_else(ser::Error::to_fmt))
        .map_err(ser::error::ErrorInternal::IoWriteFailed)?
}

pub fn to_string<T: Serialize>(value: &T) -> Result<String, ser::Error> {
    let mut result = String::new();
    to_fmt_writer(&mut result, value)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use quickcheck::{quickcheck, TestResult};
    use std::collections::HashMap;

    quickcheck! {
        fn reversible_map_string_serialization(map: HashMap<String, String>) -> TestResult {
            for (key, value) in &map {
                if key.is_empty() || key.contains(&[':', '\n', '\0'] as &[_]) || key.trim() != key || value.trim() != value || value.contains('\0') {
                    return TestResult::discard();
                }
                if let Some(_) = value.split('\n').find(|line| line.trim() != *line || *line == ".") {
                    return TestResult::discard();
                }
            }
            let s = super::to_string(&map).unwrap();
            let deserialized = super::from_str::<HashMap<String, String>>(&s).unwrap();
            TestResult::from_bool(deserialized == map)
        }

        fn reversible_map_vec_serialization(map: HashMap<String, Vec<String>>) -> TestResult {
            for (key, value) in &map {
                if key.is_empty() || key.contains(&[':', '\n', '\0'] as &[_]) || key.trim() != key || value.is_empty() {
                    return TestResult::discard();
                }

                for item in value {
                    if item.trim() != item || item.contains(&[',', '\n'] as &[_]) {
                        return TestResult::discard();
                    }
                }
            }
            let s = super::to_string(&map).unwrap();
            let deserialized = super::from_str::<HashMap<String, Vec<String>>>(&s).unwrap();
            TestResult::from_bool(deserialized == map)
        }
    }

    #[test]
    fn empty_val() {
        let mut map = HashMap::new();
        map.insert("X".to_owned(), String::new());
        let s = super::to_string(&map).unwrap();
        let deserialized = super::from_str::<HashMap<String, String>>(&s).unwrap();
        assert_eq!(deserialized, map);
    }

    #[test]
    fn funny_value() {
        let mut map = HashMap::new();
        map.insert("\u{1}".to_owned(), "\u{1}\n\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}\u{1}".to_owned());
        let s = super::to_string(&map).unwrap();
        let deserialized = super::from_str::<HashMap<String, String>>(&s).unwrap();
        assert_eq!(deserialized, map);
    }

    #[test]
    fn multi_line() {
        let mut map = HashMap::new();
        map.insert("X".to_owned(), "a\nb\nc\nd".to_owned());
        let s = super::to_string(&map).unwrap();
        let deserialized = super::from_str::<HashMap<String, String>>(&s).unwrap();
        assert_eq!(deserialized, map);
    }
}
