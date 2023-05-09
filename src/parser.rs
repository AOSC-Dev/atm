use anyhow::{anyhow, Result};
use std::collections::HashSet;
use winnow::{
    ascii::space0,
    bytes::{one_of, tag, take_until0},
    combinator::repeat,
    sequence::{separated_pair, terminated},
    IResult, Parser,
};

#[inline]
fn key_name(input: &[u8]) -> IResult<&[u8], &[u8]> {
    take_until0(":")
        .verify(|input: &[u8]| !input.is_empty() && input[0] != b'\n')
        .parse_next(input)
}

#[inline]
fn separator(input: &[u8]) -> IResult<&[u8], ()> {
    (one_of(':'), space0).void().parse_next(input)
}

#[inline]
fn single_line(input: &[u8]) -> IResult<&[u8], &[u8]> {
    take_until0("\n").parse_next(input)
}

#[inline]
fn key_value(input: &[u8]) -> IResult<&[u8], (&[u8], &[u8])> {
    separated_pair(key_name, separator, single_line).parse_next(input)
}

#[inline]
fn single_package(input: &[u8]) -> IResult<&[u8], Vec<(&[u8], &[u8])>> {
    repeat(1.., terminated(key_value, tag("\n"))).parse_next(input)
}

#[inline]
fn extract_name(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let info = single_package(input)?;
    let mut found: Option<&[u8]> = None;
    for i in info.1 {
        if i.0 == &b"Package"[..] {
            found = Some(i.1);
        }
        if i.0 == &b"Status"[..] && i.1.len() > 8 && i.1[..8] == b"install "[..] && found.is_some()
        {
            return Ok((info.0, found.unwrap()));
        }
    }

    Ok((info.0, &[]))
}

#[inline]
pub fn extract_all_names(input: &[u8]) -> IResult<&[u8], Vec<&[u8]>> {
    repeat(1.., terminated(extract_name, tag("\n"))).parse_next(input)
}

pub fn list_installed(input: &[u8]) -> Result<HashSet<String>> {
    let names = extract_all_names(input);
    let mut result: HashSet<String> = HashSet::new();
    for name in names
        .map_err(|_| anyhow!("Failed to parse dpkg status file"))?
        .1
    {
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
    let test = &b"name: value"[..];
    assert_eq!(key_name(test), Ok((&b": value"[..], &b"name"[..])));
}

#[test]
fn test_seperator() {
    let test = &b": value"[..];
    let test_2 = &b": \tvalue"[..];
    assert_eq!(separator(test), Ok((&b"value"[..], ())));
    assert_eq!(separator(test_2), Ok((&b"value"[..], ())));
}

#[test]
fn test_single_line() {
    let test = &b"value\n"[..];
    let test_2 = &b"value\t\r\n"[..];
    let test_3 = &b"value \x23\xff\n"[..];
    assert_eq!(single_line(test), Ok((&b"\n"[..], &b"value"[..])));
    assert_eq!(single_line(test_2), Ok((&b"\n"[..], &b"value\t\r"[..])));
    assert_eq!(
        single_line(test_3),
        Ok((&b"\n"[..], &b"value \x23\xff"[..]))
    );
}

#[test]
fn test_key_value() {
    let test = &b"name1: value\n"[..];
    let test_2 = &b"name2: value\t\r\n"[..];
    let test_3 = &b"name3: value \x23\xff\n"[..];
    assert_eq!(
        key_value(test),
        Ok((&b"\n"[..], (&b"name1"[..], &b"value"[..])))
    );
    assert_eq!(
        key_value(test_2),
        Ok((&b"\n"[..], (&b"name2"[..], &b"value\t\r"[..])))
    );
    assert_eq!(
        key_value(test_3),
        Ok((&b"\n"[..], (&b"name3"[..], &b"value \x23\xff"[..])))
    );
}

#[test]
fn test_package() {
    let test = &b"Package: zsync\nVersion: 0.6.2-1\nStatus: install ok installed\nArchitecture: amd64\nInstalled-Size: 256\n\n"[..];
    assert_eq!(
        single_package(test),
        Ok((
            &b"\n"[..],
            vec![
                (&b"Package"[..], &b"zsync"[..]),
                (&b"Version"[..], &b"0.6.2-1"[..]),
                (&b"Status"[..], &b"install ok installed"[..]),
                (&b"Architecture"[..], &b"amd64"[..]),
                (&b"Installed-Size"[..], &b"256"[..])
            ]
        ))
    );
    assert_eq!(extract_name(test), Ok((&b"\n"[..], (&b"zsync"[..]))));
}

#[test]
fn test_multi_package() {
    let test =
        &b"Package: zsync\nStatus: b\n\nPackage: rsync\nStatus: install ok installed\n\n"[..];
    assert_eq!(
        extract_all_names(test),
        Ok((&b""[..], vec![&b""[..], &b"rsync"[..]]))
    );
}
