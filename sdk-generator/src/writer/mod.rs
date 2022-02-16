//! the module to abstract writing files to directory or archive file.

use crate::SdkFormat;
use relative_path::RelativePath;
use std::path::Path;
use std::{fs, io};

mod dir;
mod internal;
mod tar;
mod tgz;
mod zip;

type DirFileTree<'a> = self::dir::DirFileTree<'a>;
type DirFile<'a, 'b> = self::dir::DirFile<'a, 'b>;

type TarFileTree<W = io::BufWriter<fs::File>> = self::tar::TarFileTree<W>;
type TarFile<'a, W = io::BufWriter<fs::File>> = self::tar::TarFile<'a, W>;

type TgzFileTree<W = io::BufWriter<fs::File>> = self::tgz::TgzFileTree<W>;
type TgzFile<'a, W = io::BufWriter<fs::File>> = self::tgz::TgzFile<'a, W>;

type ZipFileTree<W = io::BufWriter<fs::File>> = self::zip::ZipFileTree<W>;
type ZipFile<'a, W = io::BufWriter<fs::File>> = self::zip::ZipFile<'a, W>;

pub(crate) fn new_tree(path: &Path, kind: SdkFormat) -> io::Result<FileTreeWrapper> {
    if matches!(kind, SdkFormat::Dir) {
        Ok(FileTreeWrapper(WriterInner::Dir(dir::new(path)?)))
    } else {
        let file = io::BufWriter::new(fs::File::create(path)?);
        match kind {
            SdkFormat::Dir => unreachable!(),
            SdkFormat::Tar => Ok(FileTreeWrapper(WriterInner::Tar(tar::new(file)?))),
            SdkFormat::Tgz => Ok(FileTreeWrapper(WriterInner::Tgz(tgz::new(file)?))),
            SdkFormat::Zip => Ok(FileTreeWrapper(WriterInner::Zip(zip::new(file)?))),
        }
    }
}

pub trait FileTree<'a> {
    type FileWrite: ClosableWrite;
    fn mkdirp(&mut self, path: &RelativePath) -> io::Result<()>;
    fn new_file(&'a mut self, path: &RelativePath) -> io::Result<Self::FileWrite>;
    fn new_file_symlink(
        &mut self,
        original: &RelativePath,
        link: &RelativePath,
    ) -> io::Result<()>;
    fn finish(&mut self) -> io::Result<()>;
}

pub trait ClosableWrite: io::Write {
    fn close(&mut self) -> io::Result<()>;
}

impl<T: ClosableWrite> ClosableWrite for &mut T {
    fn close(&mut self) -> io::Result<()> {
        (**self).close()
    }
}

pub(crate) struct FileTreeWrapper<'a>(WriterInner<'a>);

enum WriterInner<'a> {
    Dir(DirFileTree<'a>),
    Tar(TarFileTree),
    Tgz(TgzFileTree),
    Zip(ZipFileTree),
}

impl<'a, 'b : 'a> FileTree<'a> for FileTreeWrapper<'b> {
    type FileWrite = FileWriteWrapper<'a, 'b>;

    common_impl! {
        for WriterInner::Dir, WriterInner::Tar, WriterInner::Tgz, WriterInner::Zip {
            fn mkdirp(&mut self.0, path: &RelativePath) -> io::Result<()>;
            fn new_file(&'a mut self.0, path: &RelativePath) -> io::Result<Self::FileWrite> where result;
            fn new_file_symlink(&mut self.0, original: &RelativePath, link: &RelativePath) -> io::Result<()>;
            fn finish(&mut self.0) -> io::Result<()>;
        }
    }
}

pub(crate) struct FileWriteWrapper<'a, 'b>(FileInner<'a, 'b>);

enum FileInner<'a, 'b> {
    Dir(DirFile<'a, 'b>),
    Tar(TarFile<'a>),
    Tgz(TgzFile<'a>),
    Zip(ZipFile<'a>),
}

impl_from! {<'a, 'b> |file: DirFile<'a, 'b>| -> FileWriteWrapper<'a, 'b> { FileWriteWrapper(FileInner::Dir(file)) } }
impl_from! {<'a, 'b> |file: TarFile<'a>| -> FileWriteWrapper<'a, 'b> { FileWriteWrapper(FileInner::Tar(file)) } }
impl_from! {<'a, 'b> |file: TgzFile<'a>| -> FileWriteWrapper<'a, 'b> { FileWriteWrapper(FileInner::Tgz(file)) } }
impl_from! {<'a, 'b> |file: ZipFile<'a>| -> FileWriteWrapper<'a, 'b> { FileWriteWrapper(FileInner::Zip(file)) } }

impl<'a, 'b> io::Write for FileWriteWrapper<'a, 'b> {
    common_impl! {
        for FileInner::Dir, FileInner::Tar, FileInner::Tgz, FileInner::Zip {
            fn write(&mut self.0, buf: &[u8]) -> io::Result<usize>;
            fn flush(&mut self.0) -> io::Result<()>;
        }
    }
}

impl<'a, 'b> ClosableWrite for FileWriteWrapper<'a, 'b> {
    common_impl! {
        for FileInner::Dir, FileInner::Tar, FileInner::Tgz, FileInner::Zip {
            fn close(&mut self.0) -> io::Result<()>;
        }
    }
}
