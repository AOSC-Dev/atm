use anyhow::{anyhow, Result};
use std::collections::HashSet;
use winnow::{
    ascii::space0,
    combinator::{repeat, separated_pair, terminated},
    token::{literal, one_of, take_until},
    Parser, Result as IResult,
};

#[inline]
fn key_name<'a>(input: &mut &'a [u8]) -> IResult<&'a [u8]> {
    take_until(0.., ":")
        .verify(|input: &[u8]| !input.is_empty() && input[0] != b'\n')
        .parse_next(input)
}

#[inline]
fn separator(input: &mut &[u8]) -> IResult<()> {
    (one_of(':'), space0).void().parse_next(input)
}

#[inline]
fn single_line<'a>(input: &mut &'a [u8]) -> IResult<&'a [u8]> {
    take_until(0.., "\n").parse_next(input)
}

#[inline]
fn key_value<'a>(input: &mut &'a [u8]) -> IResult<(&'a [u8], &'a [u8])> {
    separated_pair(key_name, separator, single_line).parse_next(input)
}

#[inline]
fn single_package<'a>(input: &mut &'a [u8]) -> IResult<Vec<(&'a [u8], &'a [u8])>> {
    repeat(1.., terminated(key_value, literal("\n"))).parse_next(input)
}

#[inline]
fn extract_name<'a>(input: &mut &'a [u8]) -> IResult<&'a [u8]> {
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
pub fn extract_all_names<'a>(input: &mut &'a [u8]) -> IResult<Vec<&'a [u8]>> {
    repeat(1.., terminated(extract_name, literal("\n"))).parse_next(input)
}

pub fn list_installed(input: &mut &[u8]) -> Result<HashSet<String>> {
    let names = extract_all_names(input);
    let mut result: HashSet<String> = HashSet::new();
    for name in names.map_err(|_| anyhow!("Failed to parse dpkg status file"))? {
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
    let mut test = &b"name: value"[..];
    assert_eq!(key_name(&mut test), Ok(&b"name"[..]));
}

#[test]
fn test_seperator() {
    let mut test = &b": value"[..];
    let mut test_2 = &b": \tvalue"[..];
    assert_eq!(separator(&mut test), Ok(()));
    assert_eq!(separator(&mut test_2), Ok(()));
}

#[test]
fn test_single_line() {
    let mut test = &b"value\n"[..];
    let mut test_2 = &b"value\t\r\n"[..];
    let mut test_3 = &b"value \x23\xff\n"[..];
    assert_eq!(single_line(&mut test), Ok(&b"value"[..]));
    assert_eq!(single_line(&mut test_2), Ok(&b"value\t\r"[..]));
    assert_eq!(single_line(&mut test_3), Ok(&b"value \x23\xff"[..]));
}

#[test]
fn test_key_value() {
    let mut test = &b"name1: value\n"[..];
    let mut test_2 = &b"name2: value\t\r\n"[..];
    let mut test_3 = &b"name3: value \x23\xff\n"[..];
    assert_eq!(key_value(&mut test), Ok((&b"name1"[..], &b"value"[..])));
    assert_eq!(
        key_value(&mut test_2),
        Ok((&b"name2"[..], &b"value\t\r"[..]))
    );
    assert_eq!(
        key_value(&mut test_3),
        Ok((&b"name3"[..], &b"value \x23\xff"[..]))
    );
}

#[test]
fn test_package() {
    let mut test = &b"Package: zsync\nVersion: 0.6.2-1\nStatus: install ok installed\nArchitecture: amd64\nInstalled-Size: 256\n\n"[..];
    assert_eq!(
        single_package(&mut test),
        Ok(vec![
            (&b"Package"[..], &b"zsync"[..]),
            (&b"Version"[..], &b"0.6.2-1"[..]),
            (&b"Status"[..], &b"install ok installed"[..]),
            (&b"Architecture"[..], &b"amd64"[..]),
            (&b"Installed-Size"[..], &b"256"[..])
        ])
    );
    assert_eq!(extract_name(&mut test), Ok(&b"zsync"[..]));
}

#[test]
fn test_multi_package() {
    let mut test =
        &b"Package: zsync\nStatus: b\n\nPackage: rsync\nStatus: install ok installed\n\n"[..];
    assert_eq!(
        extract_all_names(&mut test),
        Ok(vec![&b""[..], &b"rsync"[..]])
    );
}
