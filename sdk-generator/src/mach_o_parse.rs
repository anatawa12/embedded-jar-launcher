use crate::text_api::{CpuArch, DylibInfo, Platform, SymbolInfo};
use crate::{CommonFatHeader, CommonMachHeader};
use anyhow::{bail, Result};
use log::{trace, warn};
use object::macho::*;
use object::read::macho::{LoadCommandVariant, Nlist};
use object::Endianness;
use std::str::from_utf8;

pub(crate) fn collect_dylib_of_macho(data: &[u8]) -> Result<Vec<DylibInfo>> {
    let mut symbols: Vec<DylibInfo> = vec![];

    if let Some(fat) = CommonFatHeader::parse(data).ok() {
        trace!("fat binary found.");
        for found in fat {
            trace!("using {:?}", found.architecture());
            symbols.append(&mut parse_dylib_macho(found.data(data)?)?);
        }
    } else {
        trace!("simple binary found.");
        symbols = parse_dylib_macho(data)?;
    }

    Ok(symbols)
}

fn parse_dylib_macho(data: &[u8]) -> Result<Vec<DylibInfo<'_>>> {
    let object_file = CommonMachHeader::<Endianness>::parse(data, 0)?;
    let endian = object_file.endian()?;
    let two_level = object_file.flags(endian) & MH_TWOLEVEL == MH_TWOLEVEL;

    if !two_level {
        bail!("MH_TWOLEVEL required");
    }

    let arch = CpuArch::from_mach_o(object_file.cputype(endian), object_file.cpusubtype(endian))?;

    let mut commands = object_file.load_commands(endian, data, 0)?;
    let mut dylib_list = vec![];
    let mut undefined_symbols = vec![];
    let mut platform: Option<Platform> = None;

    while let Some(command) = commands.next()? {
        match command.cmd() {
            | LC_LOAD_DYLIB | LC_LOAD_WEAK_DYLIB | LC_REEXPORT_DYLIB | LC_LAZY_LOAD_DYLIB
            | LC_LOAD_UPWARD_DYLIB => {
                let dylib = command.dylib()?.unwrap().dylib;
                dylib_list.push(crate::text_api::DylibInfo {
                    name: from_utf8(command.string(endian, dylib.name)?)?,
                    targets: vec![],
                    symbols: vec![],
                    timestamp: dylib.timestamp.get(endian),
                    current_version: dylib.current_version.get(endian),
                    compatibility_version: dylib.compatibility_version.get(endian),
                });
            }
            LC_SYMTAB => {
                let table = command.symtab()?.unwrap();
                let symbols = table.symbols::<MachHeader64<Endianness>, _>(endian, data)?;
                for symbol in symbols.iter() {
                    if !symbol.is_stab() {
                        undefined_symbols.push((symbols, symbol));
                    }
                }
            }
            LC_BUILD_VERSION => {
                if let LoadCommandVariant::BuildVersion(version) = command.variant()? {
                    platform = Some(Platform::from_mach_o(version.platform.get(endian))?)
                }
            }
            | LC_RPATH
            | LC_DYLD_INFO_ONLY
            | LC_MAIN
            | LC_DYLD_EXPORTS_TRIE
            | LC_DYLD_CHAINED_FIXUPS
            | LC_FILESET_ENTRY => {} // ignored LC_REQ_DYLD load commands
            _ => {
                if command.cmd() & LC_REQ_DYLD != 0 {
                    bail!(
                        "required but unsupported command found: {}",
                        load_command_tostring(command.cmd())
                    );
                }
            }
        }
    }

    for (symbols, symbol) in undefined_symbols {
        let name = from_utf8(symbol.name(endian, symbols.strings())?)?;
        let desc = symbol.n_desc(endian);
        let source = desc >> 8 & 0xff;
        if source == 0 {
            let n_type = symbol.n_type();
            if (n_type & N_TYPE) == N_UNDF || (n_type & N_TYPE) == N_PBUD {
                warn!("undefined external symbol by ZERO found: {}", name);
            }
            continue;
        }
        let dylib = &mut dylib_list[(source - 1) as usize];
        dylib.symbols.push(SymbolInfo {
            name,
            arch,
            platform,
        })
    }

    for x in &mut dylib_list {
        x.targets.push((arch, platform));
    }

    Ok(dylib_list)
}

define_to_string! {
    load_command_tostring <= u32; "{:30}",
    use object::macho::*;
    [
        LC_SEGMENT,
        LC_SYMTAB,
        LC_SYMSEG,
        LC_THREAD,
        LC_UNIXTHREAD,
        LC_LOADFVMLIB,
        LC_IDFVMLIB,
        LC_IDENT,
        LC_FVMFILE,
        LC_PREPAGE,
        LC_DYSYMTAB,
        LC_LOAD_DYLIB,
        LC_ID_DYLIB,
        LC_LOAD_DYLINKER,
        LC_ID_DYLINKER,
        LC_PREBOUND_DYLIB,
        LC_ROUTINES,
        LC_SUB_FRAMEWORK,
        LC_SUB_UMBRELLA,
        LC_SUB_CLIENT,
        LC_SUB_LIBRARY,
        LC_TWOLEVEL_HINTS,
        LC_PREBIND_CKSUM,
        LC_LOAD_WEAK_DYLIB,
        LC_SEGMENT_64,
        LC_ROUTINES_64,
        LC_UUID,
        LC_RPATH,
        LC_CODE_SIGNATURE,
        LC_SEGMENT_SPLIT_INFO,
        LC_REEXPORT_DYLIB,
        LC_LAZY_LOAD_DYLIB,
        LC_ENCRYPTION_INFO,
        LC_DYLD_INFO,
        LC_DYLD_INFO_ONLY,
        LC_LOAD_UPWARD_DYLIB,
        LC_VERSION_MIN_MACOSX,
        LC_VERSION_MIN_IPHONEOS,
        LC_FUNCTION_STARTS,
        LC_DYLD_ENVIRONMENT,
        LC_MAIN,
        LC_DATA_IN_CODE,
        LC_SOURCE_VERSION,
        LC_DYLIB_CODE_SIGN_DRS,
        LC_ENCRYPTION_INFO_64,
        LC_LINKER_OPTION,
        LC_LINKER_OPTIMIZATION_HINT,
        LC_VERSION_MIN_TVOS,
        LC_VERSION_MIN_WATCHOS,
        LC_NOTE,
        LC_BUILD_VERSION,
        LC_DYLD_EXPORTS_TRIE,
        LC_DYLD_CHAINED_FIXUPS,
        LC_FILESET_ENTRY,
    ]
}
