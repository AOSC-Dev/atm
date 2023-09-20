use std::fmt::{Debug, Display};
use std::{collections::HashSet, error::Error};
use winnow::error::ErrorKind;
use winnow::{
    ascii::space0,
    combinator::{repeat, separated_pair, terminated},
    error::ParserError,
    token::{one_of, tag, take_until0},
    PResult, Parser,
};

#[inline]
fn key_name<'a, E: ParserError<&'a [u8]>>(input: &mut &'a [u8]) -> PResult<&'a [u8], E> {
    take_until0(":")
        .verify(|input: &[u8]| !input.is_empty() && input[0] != b'\n')
        .parse_next(input)
}

#[inline]
fn separator<'a, E: ParserError<&'a [u8]>>(input: &mut &'a [u8]) -> PResult<(), E> {
    (one_of(':'), space0).void().parse_next(input)
}

#[inline]
fn single_line<'a, E: ParserError<&'a [u8]>>(input: &mut &'a [u8]) -> PResult<&'a [u8], E> {
    take_until0("\n").parse_next(input)
}

#[inline]
fn key_value<'a, E: ParserError<&'a [u8]>>(
    input: &mut &'a [u8],
) -> PResult<(&'a [u8], &'a [u8]), E> {
    separated_pair(key_name, separator, single_line).parse_next(input)
}

#[inline]
fn single_package<'a, E: ParserError<&'a [u8]>>(
    input: &mut &'a [u8],
) -> PResult<Vec<(&'a [u8], &'a [u8])>, E> {
    repeat(1.., terminated(key_value, tag("\n"))).parse_next(input)
}

#[inline]
fn extract_name<'a, E: ParserError<&'a [u8]>>(input: &mut &'a [u8]) -> PResult<&'a [u8], E> {
    let info = single_package(input)?;
    let mut found: Option<&[u8]> = None;
    for i in info {
        if i.0 == &b"Package"[..] {
            found = Some(i.1);
        }
        if i.0 == &b"Status"[..] && i.1.len() > 8 && i.1[..8] == b"install "[..] && found.is_some()
        {
            return Ok(found.unwrap());
        }
    }

    Ok(&[])
}

#[inline]
pub fn extract_all_names<'a, E: ParserError<&'a [u8]>>(
    input: &mut &'a [u8],
) -> PResult<Vec<&'a [u8]>, E> {
    repeat(1.., terminated(extract_name, tag("\n"))).parse_next(input)
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct AtmParseError {
    input: String,
    kind: winnow::error::ErrorKind,
}

impl ParserError<&[u8]> for AtmParseError {
    fn from_error_kind(input: &&[u8], kind: ErrorKind) -> Self {
        Self {
            input: String::from_utf8_lossy(input).to_string(),
            kind,
        }
    }

    fn append(self, _: &&[u8], _: ErrorKind) -> Self {
        self
    }
}

impl Display for AtmParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{self:?}")
    }
}

impl Error for AtmParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

pub fn list_installed<'a>(input: &mut &'a [u8]) -> PResult<HashSet<String>, AtmParseError> {
    let names: PResult<Vec<&[u8]>, AtmParseError> = extract_all_names(input);
    let mut result: HashSet<String> = HashSet::new();
    for name in names? {
        if name.is_empty() {
            continue;
        }
        result.insert(String::from_utf8_lossy(name).to_string());
    }

    Ok(result)
}

// tests
#[test]
fn test_key_name() {
    let test = &mut &b"name: value"[..];
    assert_eq!(key_name::<()>(test), Ok(&b"name"[..]));
}

#[test]
fn test_seperator() {
    let test = &mut &b": value"[..];
    let test_2 = &mut &b": \tvalue"[..];
    assert_eq!(separator::<()>(test), Ok(()));
    assert_eq!(separator::<()>(test_2), Ok(()));
}

#[test]
fn test_single_line() {
    let test = &mut &b"value\n"[..];
    let test_2 = &mut &b"value\t\r\n"[..];
    let test_3 = &mut &b"value \x23\xff\n"[..];
    assert_eq!(single_line::<()>(test), Ok(&b"value"[..]));
    assert_eq!(single_line::<()>(test_2), Ok(&b"value\t\r"[..]));
    assert_eq!(single_line::<()>(test_3), Ok(&b"value \x23\xff"[..]));
}

#[test]
fn test_key_value() {
    let test = &mut &b"name1: value\n"[..];
    let test_2 = &mut &b"name2: value\t\r\n"[..];
    let test_3 = &mut &b"name3: value \x23\xff\n"[..];
    assert_eq!(key_value::<()>(test), Ok((&b"name1"[..], &b"value"[..])));
    assert_eq!(
        key_value::<()>(test_2),
        Ok((&b"name2"[..], &b"value\t\r"[..]))
    );
    assert_eq!(
        key_value::<()>(test_3),
        Ok((&b"name3"[..], &b"value \x23\xff"[..]))
    );
}

#[test]
fn test_package() {
    let test = &mut &b"Package: zsync\nVersion: 0.6.2-1\nStatus: install ok installed\nArchitecture: amd64\nInstalled-Size: 256\n\n"[..];
    assert_eq!(
        single_package::<()>(test),
        Ok(vec![
            (&b"Package"[..], &b"zsync"[..]),
            (&b"Version"[..], &b"0.6.2-1"[..]),
            (&b"Status"[..], &b"install ok installed"[..]),
            (&b"Architecture"[..], &b"amd64"[..]),
            (&b"Installed-Size"[..], &b"256"[..])
        ])
    );
    let test = &mut &b"Package: zsync\nVersion: 0.6.2-1\nStatus: install ok installed\nArchitecture: amd64\nInstalled-Size: 256\n\n"[..];
    assert_eq!(extract_name::<()>(test), Ok(&b"zsync"[..]));
}

#[test]
fn test_multi_package() {
    let test =
        &mut &b"Package: zsync\nStatus: b\n\nPackage: rsync\nStatus: install ok installed\n\n"[..];
    assert_eq!(
        extract_all_names::<()>(test),
        Ok(vec![&b""[..], &b"rsync"[..]])
    );
}
