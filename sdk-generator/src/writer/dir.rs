use super::internal::*;
use super::ClosableWrite;
use relative_path::RelativePathBuf;
use std::path::Path;
use std::{fs, io};

pub(super) fn new(root: &Path) -> io::Result<DirFileTree> {
    match fs::remove_dir_all(root) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
        Ok(_) => {}
    }
    fs::create_dir_all(root)?;
    Ok(DirFileTree {
        root,
        writing: None,
    })
}

pub struct DirFileTree<'a> {
    root: &'a Path,
    writing: Option<fs::File>,
}

impl<'a> DirFileTree<'a> {
    pub(super) fn try_close(&mut self) -> io::Result<()> {
        if let Some(ref mut file) = self.writing {
            io::Write::flush(file)?;
            self.writing = None;
            Ok(())
        } else {
            Ok(())
        }
    }
}

impl<'a, 'b: 'a> FileTreeImpl<'a> for DirFileTree<'b> {
    type FileWrite = DirFile<'a, 'b>;

    unsafe fn mkdirp(&mut self, path: RelativePathBuf) -> io::Result<()> {
        self.try_close()?;
        fs::create_dir_all(path.to_path(self.root))
    }

    unsafe fn new_file(&'a mut self, path: RelativePathBuf) -> io::Result<Self::FileWrite> {
        self.try_close()?;
        self.writing = Some(
            fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(path.to_path(self.root))?,
        );
        Ok(DirFile(self))
    }

    unsafe fn new_file_symlink(
        &mut self,
        original: RelativePathBuf,
        link: RelativePathBuf,
    ) -> io::Result<()> {
        self.try_close()?;

        #[cfg(unix)]
        use std::os::unix::fs::symlink;
        #[cfg(windows)]
        use std::os::windows::fs::symlink_file as symlink;

        symlink(original.to_path(Path::new("")), link.to_path(self.root))?;

        Ok(())
    }

    fn finish(&mut self) -> io::Result<()> {
        self.try_close()?;
        Ok(())
    }
}

pub struct DirFile<'a, 'b>(&'a mut DirFileTree<'b>);

impl<'a, 'b> DirFile<'a, 'b> {
    fn file(&mut self) -> io::Result<&mut fs::File> {
        self.0.writing.as_mut().ok_or_else(closed)
    }
}

impl<'a, 'b> io::Write for DirFile<'a, 'b> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file()?.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file()?.flush()
    }
}

impl<'a, 'b> ClosableWrite for DirFile<'a, 'b> {
    fn close(&mut self) -> io::Result<()> {
        io::Write::flush(self)?;
        self.0.writing = None;
        Ok(())
    }
}
