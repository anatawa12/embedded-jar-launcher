use super::internal::FileTreeImpl;
use super::tar::{TarFile, TarFileTree};
use crate::writer::ClosableWrite;
use flate2::write::GzEncoder;
use flate2::Compression;
use relative_path::RelativePathBuf;
use std::io;

pub(super) fn new<W: io::Write>(writer: W) -> io::Result<TgzFileTree<W>> {
    super::tar::new(GzEncoder::new(writer, Compression::default()))
        .map(WriterInner::Tar)
        .map(TgzFileTree)
}

pub(super) struct TgzFileTree<W: io::Write>(WriterInner<W>);

// use enum to use common_impl! macro
enum WriterInner<W: io::Write> {
    Tar(TarFileTree<GzEncoder<W>>),
}

impl<'a, W: io::Write + 'a> FileTreeImpl<'a> for TgzFileTree<W> {
    type FileWrite = TgzFile<'a, W>;

    common_impl! {
        for WriterInner::Tar {
            unsafe fn mkdirp(&mut self.0, path: RelativePathBuf) -> io::Result<()>;
            unsafe fn new_file(&'a mut self.0, path: RelativePathBuf) -> io::Result<Self::FileWrite> where result;
            unsafe fn new_file_symlink(&mut self.0, original: RelativePathBuf, link: RelativePathBuf) -> io::Result<()>;
            fn finish(&mut self.0) -> io::Result<()>;
        }
    }
}

pub(super) struct TgzFile<'a, W: io::Write>(FileInner<'a, W>);

// use enum to use common_impl! macro
enum FileInner<'a, W: io::Write> {
    Tar(TarFile<'a, GzEncoder<W>>),
}

impl<'a, W: io::Write> io::Write for TgzFile<'a, W> {
    common_impl! {
        for FileInner::Tar {
            fn write(&mut self.0, buf: &[u8]) -> io::Result<usize>;
            fn flush(&mut self.0) -> io::Result<()>;
        }
    }
}

impl<'a, W: io::Write> ClosableWrite for TgzFile<'a, W> {
    common_impl! {
        for FileInner::Tar {
            fn close(&mut self.0) -> io::Result<()>;
        }
    }
}

impl_from! { <'a, W: io::Write> |tar: TarFile<'a, GzEncoder<W>>| -> TgzFile<'a, W> {TgzFile(FileInner::Tar(tar))} }
