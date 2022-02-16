use object::macho::*;
use object::read::macho::*;
use object::*;
use std::fmt::Debug;
use std::slice::Iter;

#[derive(Debug, Clone, Copy)]
pub(crate) enum CommonMachHeader<'data, E: Endian> {
    B32(&'data MachHeader32<E>),
    B64(&'data MachHeader64<E>),
}

impl<'d, E: Endian> CommonMachHeader<'d, E> {
    common_impl! {
        for B32, B64 {
            pub fn is_type_64(&self) -> bool;
            pub fn is_big_endian(&self) -> bool;
            pub fn is_little_endian(&self) -> bool;
            pub fn magic(&self) -> u32;
            pub fn cputype(&self, endian: E) -> u32;
            pub fn cpusubtype(&self, endian: E) -> u32;
            pub fn filetype(&self, endian: E) -> u32;
            pub fn ncmds(&self, endian: E) -> u32;
            pub fn sizeofcmds(&self, endian: E) -> u32;
            pub fn flags(&self, endian: E) -> u32;
            pub fn is_supported(&self) -> bool;
            pub fn endian(&self) -> Result<E>;
            pub fn load_commands<'data, R: ReadRef<'data>>(
                &self,
                endian: E,
                data: R,
                header_offset: u64,
            ) -> Result<LoadCommandIterator<'data, E>>;
            pub fn uuid<'data, R: ReadRef<'data>>(
                &self,
                endian: E,
                data: R,
                header_offset: u64,
            ) -> Result<Option<[u8; 16]>>;
        }
    }

    pub fn parse<R: ReadRef<'d>>(data: R, offset: u64) -> read::Result<Self> {
        let magic = data
            .read::<u32>(&mut 0)
            .read_error("Invalid Mach-O header size or alignment")?;
        match *magic {
            macho::MH_MAGIC | macho::MH_CIGAM => {
                Ok(CommonMachHeader::B32(MachHeader32::parse(data, offset)?))
            }
            macho::MH_MAGIC_64 | macho::MH_CIGAM_64 => {
                Ok(CommonMachHeader::B64(MachHeader64::parse(data, offset)?))
            }
            _ => Err(new_err("Unsupported Mach-O header")),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum CommonFatHeader<'data> {
    B32(&'data [FatArch32]),
    B64(&'data [FatArch64]),
}

impl<'data> CommonFatHeader<'data> {
    #[allow(dead_code)]
    pub fn at(&'data self, index: usize) -> CommonFatArch<'data> {
        match self {
            Self::B32(slice) => CommonFatArch::B32(&slice[index]),
            Self::B64(slice) => CommonFatArch::B64(&slice[index]),
        }
    }

    pub fn parse<R: ReadRef<'data>>(data: R) -> read::Result<Self> {
        let magic = data
            .read::<U32<BigEndian>>(&mut 0)
            .read_error("Invalid Mach-O header size or alignment")?
            .get(BigEndian);
        match magic {
            macho::FAT_MAGIC => Ok(CommonFatHeader::B32(FatHeader::parse_arch32(data)?)),
            macho::FAT_MAGIC_64 => Ok(CommonFatHeader::B64(FatHeader::parse_arch64(data)?)),
            _ => Err(new_err("Unsupported Mach-O header")),
        }
    }
}

impl<'data> IntoIterator for CommonFatHeader<'data> {
    type Item = CommonFatArch<'data>;
    type IntoIter = CommonFatArchIter<'data>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            CommonFatHeader::B32(s) => CommonFatArchIter::B32(s.into_iter()),
            CommonFatHeader::B64(s) => CommonFatArchIter::B64(s.into_iter()),
        }
    }
}

pub(crate) enum CommonFatArchIter<'data> {
    B32(Iter<'data, FatArch32>),
    B64(Iter<'data, FatArch64>),
}

impl<'data> Iterator for CommonFatArchIter<'data> {
    type Item = CommonFatArch<'data>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            CommonFatArchIter::B32(i) => i.next().map(CommonFatArch::B32),
            CommonFatArchIter::B64(i) => i.next().map(CommonFatArch::B64),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum CommonFatArch<'data> {
    B32(&'data FatArch32),
    B64(&'data FatArch64),
}

impl<'data> CommonFatArch<'data> {
    common_impl! {
        for B32, B64 {
            pub fn cputype(&self) -> u32;
            pub fn cpusubtype(&self) -> u32;
            pub fn offset(&self) -> u64;
            pub fn size(&self) -> u64;
            pub fn align(&self) -> u32;
            pub fn architecture(&self) -> Architecture;
            pub fn file_range(&self) -> (u64, u64);
            pub fn data<'data1, R: ReadRef<'data1>>(&self, file: R) -> Result<&'data1 [u8]>;
        }
    }
}
// utils

fn new_err(value: &'static str) -> Error {
    pub struct Error0(&'static str);
    unsafe { std::mem::transmute(Error0(value)) }
}

trait ReadError<T> {
    fn read_error(self, error: &'static str) -> Result<T>;
}

impl<T> ReadError<T> for ::std::result::Result<T, ()> {
    fn read_error(self, error: &'static str) -> Result<T> {
        self.map_err(|()| new_err(error))
    }
}

impl<T> ReadError<T> for ::std::result::Result<T, Error> {
    fn read_error(self, error: &'static str) -> Result<T> {
        self.map_err(|_| new_err(error))
    }
}

impl<T> ReadError<T> for Option<T> {
    fn read_error(self, error: &'static str) -> Result<T> {
        self.ok_or(new_err(error))
    }
}
