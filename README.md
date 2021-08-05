# Rust library for handling RFC822-like format used in Debian

## About

This crate implements the file format inpired by RFC822 that is used in Debian packages.
It is the format of the `debian/control` files in packages and `Packages` file of `apt`.
It is called `rfc822-like` instead of just `rfc822` because Debian does not claim to implement RFC822 exactly
and this crate is focused on working with Debian tools, not parsing exact RFC822 file format.
Frankly, I didn't even bother to read RFC822 itself.

If you need to strictly parse RFC822, I suggest you to fork this crate and fix whatever differences there are.
I'm not interested in maintaining strict RFC822 crate, so don't send PRs for that but I'm willing to put
common pieces into their own crate.
If you're interested in this approach feel free to file a PR (or ask beforehand if you have questions).

Note that this crate is currently not optimized for performance.
There are multiple places where allocation could be avoided and other optimizations may be missing.
It's absolutely fine for my own use cases, and probably will be for yours as well.
If you need it to be faster or just want to have fun improving its performance I'll be happy to accept PRs.

The API is currently not set in stone and may change over time.
Basic steps to minimize the impact of changes were taken (e.g. encapsulation of `Error` type).
The crate also currently lacks serialization but it will be implemented eventually.
Feel free to send PRs!


## Example

Check the crate documentation for more examples and detailed explanation.

```rust
use rfc822_like::de::Deserializer;
use serde::Deserialize;

let input = "Package: foo
Description: The Foo

Package: bar
Description: The Bar
";

let mut reader = input.as_bytes();

#[derive(Debug, Eq, PartialEq, serde_derive::Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Record {
    package: String,
    description: String,
}

let expected = vec![
    Record {
        package: "foo".to_owned(),
        description:"The Foo".to_owned(),
    },
    Record {
        package: "bar".to_owned(),
        description: "The Bar".to_owned(),
    },
];

let deserialized = <Vec<Record>>::deserialize(Deserializer::new(&mut reader)).unwrap();
assert_eq!(deserialized, expected);
```

## MSRV

Whatever Rust version is available in the latest Debian stable, currently 1.41.1.

## License

MITNFA
