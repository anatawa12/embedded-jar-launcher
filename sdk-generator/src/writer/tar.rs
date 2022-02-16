use super::internal::*;
use super::ClosableWrite;
use relative_path::RelativePathBuf;
use std::cmp::min;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io;
use tar::{Builder, EntryType, Header};

pub(super) fn new<W: io::Write>(writer: W) -> io::Result<TarFileTree<W>> {
    Ok(TarFileTree {
        file: TarFileImpl {
            builder: Builder::new(writer),
            data: vec![],
            path: vec![],
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
enum TarEntryStatus {
    Dir,
    File,
    Symlink,
}

pub(super) struct TarFileTree<W: io::Write> {
    file: TarFileImpl<W>,
    files: HashMap<FastRP, TarEntryStatus>,
}

impl<W: io::Write> TarFileTree<W> {
    fn check_dir(&self, path: RelativePathBuf) -> io::Result<()> {
        if path.as_str().is_empty() {
            return Ok(());
        }
        match self.files.get(&FastRP(path)) {
            Some(TarEntryStatus::Dir) => Ok(()),
            Some(_) => Err(not_a_directory()),
            None => Err(no_such_file()),
        }
    }

    fn builder(&mut self) -> &mut Builder<W> {
        &mut self.file.builder
    }
}

impl<'a, W: io::Write + 'a> FileTreeImpl<'a> for TarFileTree<W> {
    type FileWrite = TarFile<'a, W>;

    unsafe fn mkdirp(&mut self, path: RelativePathBuf) -> io::Result<()> {
        if path.components().next().is_none() {
            return Ok(());
        }
        self.file.try_close()?;
        if let Some(status) = self.files.get(&FastRP(path.clone())) {
            return match status {
                TarEntryStatus::Dir => Ok(()),
                _ => Err(already_exists()),
            };
        }
        if let Some(parent) = path.parent() {
            FileTreeImpl::mkdirp(self, parent.to_owned())?;
        }

        self.builder().append(
            &new_ustar_header(path.as_str().as_bytes(), b"", EntryType::Directory, 0)?,
            &mut &b""[..],
        )?;

        self.files.insert(FastRP(path), TarEntryStatus::Dir);
        Ok(())
    }

    unsafe fn new_file(&'a mut self, path: RelativePathBuf) -> io::Result<Self::FileWrite> {
        self.file.try_close()?;
        self.check_dir(path.parent().unwrap().to_owned())?;

        self.file.path.extend(path.as_str().as_bytes());

        self.files.insert(FastRP(path), TarEntryStatus::File);

        Ok(TarFile(&mut self.file))
    }

    unsafe fn new_file_symlink(
        &mut self,
        original: RelativePathBuf,
        link: RelativePathBuf,
    ) -> io::Result<()> {
        self.file.try_close()?;
        self.check_dir(link.parent().unwrap().to_owned())?;

        self.builder().append(
            &new_ustar_header(
                link.as_str().as_bytes(),
                original.as_str().as_bytes(),
                EntryType::Symlink,
                0,
            )?,
            &mut &b""[..],
        )?;

        self.files.insert(FastRP(link), TarEntryStatus::Symlink);

        Ok(())
    }

    fn finish(&mut self) -> io::Result<()> {
        self.builder().finish()
    }
}

pub struct TarFile<'a, W: io::Write>(&'a mut TarFileImpl<W>);

struct TarFileImpl<W: io::Write> {
    builder: Builder<W>,
    data: Vec<u8>,
    path: Vec<u8>,
}

impl<W: io::Write> TarFileImpl<W> {
    pub(super) fn try_close(&mut self) -> io::Result<()> {
        if self.path.is_empty() {
            Ok(())
        } else {
            TarFile(self).close()
        }
    }

    fn data(&mut self) -> io::Result<&mut Vec<u8>> {
        if self.path.is_empty() {
            Err(closed())
        } else {
            Ok(&mut self.data)
        }
    }
}

impl<'a, W: io::Write> io::Write for TarFile<'a, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.data()?.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a, W: io::Write> ClosableWrite for TarFile<'a, W> {
    fn close(&mut self) -> io::Result<()> {
        self.0.builder.append(
            &new_ustar_header(&self.0.path, b"", EntryType::Regular, self.0.data.len())?,
            &mut self.0.data.as_slice(),
        )?;
        self.0.path.truncate(0);
        self.0.data.truncate(0);
        Ok(())
    }
}

fn new_ustar_header(
    path: &[u8],
    link_target: &[u8],
    kind: EntryType,
    size: usize,
) -> io::Result<Header> {
    fn write_octal_8(buf: &mut [u8; 8], value: u64) {
        const OCTAL_8_MAX: u64 = 0x20_0000 - 1;
        if value == 0 {
            buf.copy_from_slice(b"000000 \0");
            return;
        }
        if value <= OCTAL_8_MAX {
            let formatted = format!("{:06o} \0", value & OCTAL_8_MAX);
            assert_eq!(formatted.len(), 8, "invalid time by formatting");
            buf.copy_from_slice(formatted.as_bytes());
        } else {
            buf[0] = ((value >> (8 * 7)) & 0xFF) as u8 | 0x80;
            buf[1] = ((value >> (8 * 6)) & 0xFF) as u8;
            buf[2] = ((value >> (8 * 5)) & 0xFF) as u8;
            buf[3] = ((value >> (8 * 4)) & 0xFF) as u8;
            buf[4] = ((value >> (8 * 3)) & 0xFF) as u8;
            buf[5] = ((value >> (8 * 2)) & 0xFF) as u8;
            buf[6] = ((value >> (8 * 1)) & 0xFF) as u8;
            buf[7] = ((value >> (8 * 0)) & 0xFF) as u8;
        }
    }

    fn write_octal_11(buf: &mut [u8; 12], value: u64) {
        const OCTAL_11_MAX: u64 = 0x2_0000_0000 - 1;
        if value <= OCTAL_11_MAX {
            let formatted = format!("{:011o} ", value & OCTAL_11_MAX);
            assert_eq!(formatted.len(), 12, "invalid time by formatting");
            buf.copy_from_slice(formatted.as_bytes());
        } else {
            buf[0] = 0x80;
            buf[4] = ((value >> (8 * 7)) & 0xFF) as u8;
            buf[5] = ((value >> (8 * 6)) & 0xFF) as u8;
            buf[6] = ((value >> (8 * 5)) & 0xFF) as u8;
            buf[7] = ((value >> (8 * 4)) & 0xFF) as u8;
            buf[8] = ((value >> (8 * 3)) & 0xFF) as u8;
            buf[9] = ((value >> (8 * 2)) & 0xFF) as u8;
            buf[10] = ((value >> (8 * 1)) & 0xFF) as u8;
            buf[11] = ((value >> (8 * 0)) & 0xFF) as u8;
        }
    }

    let mut header = Header::new_ustar();

    set_ustar_path(&mut header, path, kind.is_dir())?;

    let ustar = header.as_ustar_mut().unwrap();
    if kind.is_dir() {
        ustar.mode.copy_from_slice(b"000755 \0");
    } else {
        ustar.mode.copy_from_slice(b"000644 \0");
    }
    ustar.uid.copy_from_slice(b"000000 \0");
    ustar.gid.copy_from_slice(b"000000 \0");
    write_octal_11(&mut ustar.size, size as u64);
    write_octal_11(
        &mut ustar.mtime,
        std::time::UNIX_EPOCH.elapsed().unwrap().as_secs(),
    );

    ustar.cksum.fill(b' '); // write later

    ustar.typeflag[0] = kind.as_byte();
    if kind.is_hard_link() || kind.is_symlink() {
        if link_target.len() > ustar.linkname.len() {
            return Err(pathname_too_long());
        }
        ustar.linkname[..link_target.len()].copy_from_slice(link_target);
        ustar.linkname[link_target.len()..].fill(0);
    } else {
        ustar.linkname.fill(0);
    }

    // magic must be initialized
    // version must be initialized

    ustar.uname[..b"root".len()].copy_from_slice(b"root");
    ustar.uname[b"root".len()..].fill(0);

    ustar.gname[..b"root".len()].copy_from_slice(b"root");
    ustar.gname[b"root".len()..].fill(0);

    ustar.dev_major.copy_from_slice(b"000000 \0");
    ustar.dev_minor.copy_from_slice(b"000000 \0");

    // version must be initialized

    let mut sum: u32 = 0;
    const CHKSUM_MASK: u32 = (1 << 17) - 1;

    for x in ustar.as_header().as_bytes() {
        sum = (sum + *x as u32) & CHKSUM_MASK;
    }
    write_octal_8(&mut ustar.cksum, (sum & CHKSUM_MASK) as u64);

    Ok(header)
}

fn set_ustar_path(header: &mut Header, path: &[u8], dir: bool) -> io::Result<()> {
    const USTAR_NAME_LEN: usize = 100;
    const USTAR_PREFIX_LEN: usize = 155;

    let ustar = header.as_ustar_mut().unwrap();

    // append '/' so one more byte
    let save_path_len = if dir { path.len() + 1 } else { path.len() };

    if save_path_len >= ustar.name.len() + ustar.prefix.len() {
        return Err(pathname_too_long());
    }

    let mut const_buf = [0 as u8; USTAR_NAME_LEN + USTAR_PREFIX_LEN];

    let path: &[u8] = if dir {
        const_buf[..path.len()].copy_from_slice(path);
        const_buf[path.len()] = b'/';
        &const_buf[..(path.len() + 1)]
    } else {
        path
    };

    if path.len() <= ustar.name.len() {
        // if path is shorter than ustar.name: it's simple; just copy to ustar.name
        ustar.name[..path.len()].copy_from_slice(path);
        ustar.name[path.len()..].fill(0);
        ustar.prefix.fill(0);
        return Ok(());
    } else if let Some(slash_idx) = (0..min(ustar.prefix.len(), path.len()))
        .rev()
        .find(|i| path[*i] == b'/')
    {
        // if we can see '/' in uster.prefix, split at it and write to both.
        let name_part_len = path.len() - slash_idx + 1;
        if name_part_len >= ustar.name.len() {
            // the part will be in ustar.name is longer than limit,
            return Err(pathname_too_long());
        }
        ustar.prefix[..slash_idx].copy_from_slice(&path[..slash_idx]);
        ustar.prefix[slash_idx..].fill(0);
        ustar.name[..name_part_len].copy_from_slice(&path[(slash_idx + 1)..]);
        ustar.prefix[name_part_len..].fill(0);
        Ok(())
    } else {
        // if we can't file/dir name too long
        Err(pathname_too_long())
    }
}
