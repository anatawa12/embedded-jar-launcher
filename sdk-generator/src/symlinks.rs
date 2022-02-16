//! the module to handle symlinks file

use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, Lines};
use std::path::Path;
use std::str::FromStr;
use relative_path::{RelativePath, RelativePathBuf};

pub(crate) fn parse_symlinks_file(path: impl AsRef<Path>) -> io::Result<SymlinksParser<Lines<BufReader<File>>, io::Error, String>> {
    Ok(SymlinksParser(File::open(path).map(BufReader::new).map(BufReader::lines)?))
}

pub(crate) struct SymlinksParser<I, E, S: AsRef<str>>(I)
    where I : Iterator<Item = Result<S, E>>,
        S: AsRef<str>;

impl <I : Iterator<Item = Result<S, io::Error>>, S: AsRef<str>> Iterator for SymlinksParser<I, io::Error, S> {
    type Item = Result<SymlinkDescriptor, io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.0.next() {
            None => None,
            Some(r) => match r {
                Ok(str) => {
                    let str = str.as_ref();
                    if str.trim_start().starts_with('#') {
                        // comment line: skip
                        self.next()
                    } else {
                        Some(SymlinkDescriptor::from_str(str)
                            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)))
                    }
                }
                Err(e) => Some(Err(e))
            }
        }
    }
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub(crate) struct SymlinkDescriptor {
    link: RelativePathBuf,
    original: RelativePathBuf,
}

impl SymlinkDescriptor {
    pub(crate) fn new(link: impl Into<RelativePathBuf>, original: impl Into<RelativePathBuf>) -> Self {
        Self { link: link.into(), original: original.into() }
    }

    pub(crate) fn link(&self) -> &RelativePath {
        &self.link
    }

    pub(crate) fn original(&self) -> &RelativePath {
        &self.original
    }
}

#[derive(Debug)]
pub(crate) struct SymlinkDescriptorParseErr(());

impl std::error::Error for SymlinkDescriptorParseErr {}

impl std::fmt::Display for SymlinkDescriptorParseErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("no '->', '=>', ':' are not found in symlink descriptor")
    }
}

impl FromStr for SymlinkDescriptor {
    type Err = SymlinkDescriptorParseErr;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let a = s.rsplit_once("->");
        let b = s.rsplit_once("=>");
        let c = s.rsplit_once(':');

        let mut best: Option<(&str, &str)> = None;
        for x in [a, b, c] {
            best = match (best, x) {
                (None, Some(value)) => Some(value),
                (Some(value), None) => Some(value),
                (None, None) => None,
                (Some(prev), Some(newer)) => {
                    // use *.0 is longer
                    if prev.0.len() >= newer.0.len() {
                        Some(prev)
                    } else {
                        Some(newer)
                    }
                }
            }
        }
        if let Some((link, original)) = best {
            Ok(SymlinkDescriptor::new(link.trim_end(), original.trim_start()))
        } else {
            Err(SymlinkDescriptorParseErr(()))
        }
    }
}

#[test]
fn test_from_str() {
    assert_eq!(SymlinkDescriptor::new("", ""), SymlinkDescriptor::from_str("->").unwrap());
    assert_eq!(SymlinkDescriptor::new("/abc/def", "ghi"), SymlinkDescriptor::from_str("/abc/def->ghi").unwrap());
    assert_eq!(SymlinkDescriptor::new("", ""), SymlinkDescriptor::from_str(":").unwrap());
    assert_eq!(SymlinkDescriptor::new("/abc/def", "ghi"), SymlinkDescriptor::from_str("/abc/def:ghi").unwrap());
    assert_eq!(SymlinkDescriptor::new("", ""), SymlinkDescriptor::from_str("=>").unwrap());
    assert_eq!(SymlinkDescriptor::new("/abc/def", "ghi"), SymlinkDescriptor::from_str("/abc/def=>ghi").unwrap());
}
