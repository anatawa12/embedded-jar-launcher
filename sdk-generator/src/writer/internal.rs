use crate::writer::{ClosableWrite, FileTree};
use relative_path::{RelativePath, RelativePathBuf};
use std::io;
use log::trace;
use crate::util::safe_normalize;

impl<'a, T: FileTreeImpl<'a>> FileTree<'a> for T {
    type FileWrite = <T as FileTreeImpl<'a>>::FileWrite;

    fn mkdirp(&mut self, path: &RelativePath) -> io::Result<()> {
        let path = safe_normalize(path);
        if path == RelativePathBuf::from("") {
            return Ok(()); // nop so return
        }
        trace!("creating dir {}", path);
        unsafe { FileTreeImpl::mkdirp(self, path) }
    }

    fn new_file(&'a mut self, path: &RelativePath) -> io::Result<Self::FileWrite> {
        let path = safe_normalize(path);
        if path == RelativePathBuf::from("") {
            return Err(already_exists());
        }
        trace!("creating file {}", path);
        unsafe { FileTreeImpl::new_file(self, path) }
    }

    fn new_file_symlink(
        &mut self,
        original: &RelativePath,
        link: &RelativePath,
    ) -> io::Result<()> {
        let link = safe_normalize(link);
        let original_absolute = if original.as_str().as_bytes()[0] == b'/' {
            // if original is a absolute (/-started) path normalize it and use it
            original.normalize()
        } else {
            // if original is relative path, join with link's parent and normalize it.
            link.parent().unwrap().join(original).normalize()
        };
        let original_relative = match link.parent() {
            None => original_absolute,
            Some(parent) => parent.relative(&original_absolute),
        };

        trace!("creating symlink {} ({})", link, original);

        unsafe { FileTreeImpl::new_file_symlink(self, original_relative, link) }
    }

    fn finish(&mut self) -> io::Result<()> {
        FileTreeImpl::finish(self)
    }
}

pub trait FileTreeImpl<'a> {
    type FileWrite: ClosableWrite;
    /// this is unsafe to call because path must be safe-normalized
    unsafe fn mkdirp(&mut self, path: RelativePathBuf) -> io::Result<()>;
    /// this is unsafe to call because path must be safe-normalized
    unsafe fn new_file(&'a mut self, path: RelativePathBuf) -> io::Result<Self::FileWrite>;
    /// this is unsafe to call because link must be safe-normalized and
    /// original must be relative to the directory has original and normalized
    unsafe fn new_file_symlink(
        &mut self,
        original: RelativePathBuf,
        link: RelativePathBuf,
    ) -> io::Result<()>;
    fn finish(&mut self) -> io::Result<()>;
}

#[inline(always)]
pub(super) fn pathname_too_long() -> io::Error {
    //io::Error::new(io::ErrorKind::FilenameTooLong, "pathname too long for ustar")
    io::Error::new(io::ErrorKind::InvalidInput, "pathname too long for ustar")
}

#[inline(always)]
pub(super) fn already_exists() -> io::Error {
    io::Error::new(io::ErrorKind::AlreadyExists, "File exists")
}

#[inline(always)]
pub(super) fn not_a_directory() -> io::Error {
    //io::Error::new(io::ErrorKind::NotADirectory, "Not a directory")
    io::Error::new(io::ErrorKind::InvalidInput, "Not a directory")
}

#[inline(always)]
pub(super) fn no_such_file() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, "No such file or directory")
}

pub(super) fn closed() -> io::Error {
    io::Error::new(io::ErrorKind::Other, "closed")
}
