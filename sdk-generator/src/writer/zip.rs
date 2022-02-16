use super::internal::*;
use super::ClosableWrite;
use relative_path::{RelativePath, RelativePathBuf};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io;
use std::io::Write;
use zip::write::FileOptions;
use zip::ZipWriter;

pub(super) fn new<W: io::Write + io::Seek>(writer: W) -> io::Result<ZipFileTree<W>> {
    Ok(ZipFileTree {
        file: ZipFileImpl {
            builder: ZipWriter::new(writer),
            writing: false,
        },
        files: HashMap::new(),
    })
}

/// the class to implement fast hash and eq for relative path
#[derive(Debug)]
struct FastRP(RelativePathBuf);

impl PartialEq<Self> for FastRP {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl Eq for FastRP {}

impl Hash for FastRP {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_str().hash(state)
    }
}

#[derive(Eq, PartialEq, Debug)]
enum ZipEntryStatus {
    Dir,
    File,
    Symlink,
}

pub(super) struct ZipFileTree<W: io::Write + io::Seek> {
    file: ZipFileImpl<W>,
    files: HashMap<FastRP, ZipEntryStatus>,
}

impl<W: io::Write + io::Seek> ZipFileTree<W> {
    fn check_dir(&self, path: &RelativePath) -> io::Result<()> {
        if path.as_str().is_empty() {
            return Ok(());
        }
        match self.files.get(&FastRP(path.to_owned())) {
            Some(ZipEntryStatus::Dir) => Ok(()),
            Some(_) => Err(not_a_directory()),
            None => Err(no_such_file()),
        }
    }

    fn builder(&mut self) -> &mut ZipWriter<W> {
        &mut self.file.builder
    }
}

impl<'a, W: io::Write + io::Seek + 'a> FileTreeImpl<'a> for ZipFileTree<W> {
    type FileWrite = ZipFile<'a, W>;

    unsafe fn mkdirp(&mut self, path: RelativePathBuf) -> io::Result<()> {
        if path.components().next().is_none() {
            return Ok(());
        }
        self.file.try_close()?;
        if let Some(status) = self.files.get(&FastRP(path.clone())) {
            return match status {
                ZipEntryStatus::Dir => Ok(()),
                _ => Err(already_exists()),
            };
        }
        if let Some(parent) = path.parent() {
            FileTreeImpl::mkdirp(self, parent.to_owned())?;
        }

        // TODO: see https://github.com/udoprog/relative-path/pull/34
        self.builder().add_directory(
            path.as_str().to_owned(),
            FileOptions::default().unix_permissions(0o0040755),
        )?;

        self.files.insert(FastRP(path), ZipEntryStatus::Dir);
        Ok(())
    }

    unsafe fn new_file(&'a mut self, path: RelativePathBuf) -> io::Result<Self::FileWrite> {
        self.file.try_close()?;
        self.check_dir(path.parent().unwrap())?;

        // TODO: see https://github.com/udoprog/relative-path/pull/34
        self.builder().start_file(
            path.as_str().to_owned(),
            FileOptions::default().unix_permissions(0o644),
        )?;

        self.files.insert(FastRP(path), ZipEntryStatus::File);
        self.file.writing = true;

        Ok(ZipFile(&mut self.file))
    }

    unsafe fn new_file_symlink(
        &mut self,
        original: RelativePathBuf,
        link: RelativePathBuf,
    ) -> io::Result<()> {
        self.file.try_close()?;
        self.check_dir(link.parent().unwrap())?;

        // TODO: re-implement with zip-rs's api instead of our unsafe implementation
        fn force_symlink(opt: FileOptions) -> FileOptions {
            pub struct FileOptions0 {
                _compression_method: zip::CompressionMethod,
                _last_modified_time: zip::DateTime,
                permissions: Option<u32>,
                _large_file: bool,
            }
            unsafe {
                let mut zero: FileOptions0 = std::mem::transmute(opt);
                *zero.permissions.as_mut().unwrap() |= 0o120000;
                std::mem::transmute(zero)
            }
        }

        self.builder().start_file(
            link.as_str().to_owned(),
            force_symlink(FileOptions::default().unix_permissions(0o777)),
        )?;
        self.builder().write(original.as_str().as_bytes())?;
        self.builder().flush()?;

        self.files.insert(FastRP(link), ZipEntryStatus::Symlink);

        Ok(())
    }

    fn finish(&mut self) -> io::Result<()> {
        self.builder().finish()?;
        Ok(())
    }
}

pub struct ZipFile<'a, W: io::Write + io::Seek>(&'a mut ZipFileImpl<W>);

struct ZipFileImpl<W: io::Write + io::Seek> {
    builder: ZipWriter<W>,
    writing: bool,
}

impl<W: io::Write + io::Seek> ZipFileImpl<W> {
    pub(super) fn try_close(&mut self) -> io::Result<()> {
        if self.writing {
            ZipFile(self).close()
        } else {
            Ok(())
        }
    }

    fn builder(&mut self) -> io::Result<&mut ZipWriter<W>> {
        if self.writing {
            Ok(&mut self.builder)
        } else {
            Err(closed())
        }
    }
}

impl<'a, W: io::Write + io::Seek> io::Write for ZipFile<'a, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.builder()?.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a, W: io::Write + io::Seek> ClosableWrite for ZipFile<'a, W> {
    fn close(&mut self) -> io::Result<()> {
        io::Write::flush(&mut self.0.builder)?;
        self.0.writing = false;
        Ok(())
    }
}
