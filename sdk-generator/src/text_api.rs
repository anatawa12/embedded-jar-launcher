use multimap::MultiMap;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Formatter;
use std::str::FromStr;
use std::{io, str};

#[derive(Debug)]
pub(crate) enum TextApiGenErr {
    Io(io::Error),
    NoPlatform(),
}

impl ::std::error::Error for TextApiGenErr {}

impl ::core::fmt::Display for TextApiGenErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TextApiGenErr::Io(e) => ::core::fmt::Display::fmt(e, f),
            TextApiGenErr::NoPlatform() => f.write_str(
                "platform is not specified by mach-o and no default platform is specified",
            ),
        }
    }
}

impl From<io::Error> for TextApiGenErr {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

pub(crate) fn generate_text_api<'a>(
    default_platform: Option<Platform>,
    dylib: &'a DylibInfo<'a>,
    mut file: impl io::Write,
) -> Result<(), TextApiGenErr> {
    let mut all_targets = HashSet::new();
    let mut targets_by_symbol = MultiMap::new();

    for x in &dylib.symbols {
        let target = Target(
            x.arch,
            x.platform
                .or(default_platform)
                .ok_or(TextApiGenErr::NoPlatform())?,
        );
        all_targets.insert(target);
        targets_by_symbol.insert(x.name, target);
    }

    let mut symbols_by_targets = MultiMap::new();

    for (symbol, targets) in targets_by_symbol {
        symbols_by_targets.insert(BTreeSet::from_iter(targets), symbol);
    }

    for (arch, platform) in &dylib.targets {
        let target = Target(
            *arch,
            platform
                .or(default_platform)
                .ok_or(TextApiGenErr::NoPlatform())?,
        );
        all_targets.insert(target);
    }

    writeln!(file, "--- !tapi-tbd")?;
    writeln!(file, "tbd-version:     4")?;
    writeln!(file, "targets: [")?;
    for target in all_targets {
        writeln!(file, "    {},", target)?;
    }
    writeln!(file, "]")?;
    writeln!(file, "install-name:    '{}'", dylib.name)?;
    writeln!(file, "current-version: {}", dylib.current_version >> 16)?;
    writeln!(file, "exports:")?;
    for (targets, symbols) in symbols_by_targets {
        writeln!(file, "  - targets: [")?;
        for target in &targets {
            writeln!(file, "        {},", target)?;
        }
        writeln!(file, "    ]")?;
        writeln!(file, "    symbols: [")?;
        for symbol in symbols {
            writeln!(file, "        {},", symbol)?;
        }
        writeln!(file, "    ]")?;
    }
    writeln!(file, "...")?;
    Ok(())
}

#[derive(Debug)]
pub(crate) struct DylibInfo<'data> {
    pub name: &'data str,
    pub targets: Vec<(CpuArch, Option<Platform>)>,
    pub symbols: Vec<SymbolInfo<'data>>,
    #[allow(dead_code)]
    pub timestamp: u32,
    pub current_version: u32,
    #[allow(dead_code)]
    pub compatibility_version: u32,
}

impl<'data> DylibInfo<'data> {
    pub(crate) fn unique_dylib(dylib_list: Vec<DylibInfo>) -> Vec<DylibInfo> {
        let mut dylib_table = BTreeMap::<&str, DylibInfo>::new();

        for mut dylib in dylib_list {
            if let Some(info) = dylib_table.get_mut(dylib.name) {
                info.symbols.append(&mut dylib.symbols);
            } else {
                dylib_table.insert(dylib.name, dylib);
            }
        }

        dylib_table.into_values().collect()
    }
}

#[derive(Debug)]
pub(crate) struct SymbolInfo<'data> {
    pub name: &'data str,
    pub arch: CpuArch,
    pub platform: Option<Platform>,
}

str_enum! {
    pub(crate) enum CpuArch {
        pub(crate) type Err = UnknownCpuArchErr("unknown cpu arch: {}");
        I386("i386"),
        X86_64("x86_64"),
        X86_64H("x86_64h"),
        ArmV4t("armv4t"),
        ArmV6("armv6"),
        ArmV5("armv5"),
        ArmV7("armv7"),
        ArmV7s("armv7s"),
        ArmV7k("armv7k"),
        ArmV6m("armv6m"),
        ArmV7m("armv7m"),
        ArmV7em("armv7em"),
        Arm64("arm64"),
        Arm64E("arm64e"),
        Arm64_32("arm64_32"),
    }
}

impl CpuArch {
    pub(crate) fn from_mach_o(cputype: u32, cpusubtype: u32) -> Result<Self, UnknownCpuArchErr> {
        use object::macho::*;
        match cputype {
            CPU_TYPE_X86 => Ok(Self::I386),
            CPU_TYPE_X86_64 => match cpusubtype {
                CPU_SUBTYPE_X86_64_H => Ok(Self::X86_64H),
                _ => Ok(Self::X86_64),
            },
            CPU_TYPE_ARM => match cpusubtype {
                CPU_SUBTYPE_ARM_V4T => Ok(Self::ArmV4t),
                CPU_SUBTYPE_ARM_V6 => Ok(Self::ArmV6),
                CPU_SUBTYPE_ARM_V5TEJ => Ok(Self::ArmV5),
                CPU_SUBTYPE_ARM_V7 => Ok(Self::ArmV7),
                CPU_SUBTYPE_ARM_V7S => Ok(Self::ArmV7s),
                CPU_SUBTYPE_ARM_V7K => Ok(Self::ArmV7k),
                CPU_SUBTYPE_ARM_V6M => Ok(Self::ArmV6m),
                CPU_SUBTYPE_ARM_V7M => Ok(Self::ArmV7m),
                CPU_SUBTYPE_ARM_V7EM => Ok(Self::ArmV7em),
                _ => Err(UnknownCpuArchErr(format!("{}:{}", cputype, cpusubtype))),
            },
            CPU_TYPE_ARM64 => match cpusubtype {
                CPU_SUBTYPE_ARM64E => Ok(Self::Arm64E),
                _ => Ok(Self::Arm64),
            },
            CPU_TYPE_ARM64_32 => match cpusubtype {
                CPU_SUBTYPE_ARM64_32_V8 => Ok(Self::Arm64_32),
                _ => Err(UnknownCpuArchErr(format!("{}:{}", cputype, cpusubtype))),
            },
            _ => Err(UnknownCpuArchErr(format!("{}:{}", cputype, cpusubtype))),
        }
    }
}

str_enum! {
    pub(crate) enum Platform {
        pub(crate) type Err = UnknownPlatformErr("unknown platform: {}");
        MacOS("macos"),
        IOS("ios"),
        TvOS("tvos"),
        WatchOS("watchos"),
        BridgeOS("bridgeos"),
        MacCatalyst("maccatalyst"),
        IOSSimulator("ios-simulator"),
        TvOSSimulator("tvos-simulator"),
        WatchOSSimulator("watchos-simulator"),
        DriverKit("driverkit"),
    }
}

impl Platform {
    pub(crate) fn from_mach_o(platform: u32) -> Result<Self, UnknownPlatformErr> {
        use object::macho::*;

        match platform {
            PLATFORM_MACOS => Ok(Self::MacOS),
            PLATFORM_IOS => Ok(Self::IOS),
            PLATFORM_TVOS => Ok(Self::TvOS),
            PLATFORM_WATCHOS => Ok(Self::WatchOS),
            PLATFORM_BRIDGEOS => Ok(Self::BridgeOS),
            PLATFORM_MACCATALYST => Ok(Self::MacCatalyst),
            PLATFORM_IOSSIMULATOR => Ok(Self::IOSSimulator),
            PLATFORM_TVOSSIMULATOR => Ok(Self::TvOSSimulator),
            PLATFORM_WATCHOSSIMULATOR => Ok(Self::WatchOSSimulator),
            PLATFORM_DRIVERKIT => Ok(Self::DriverKit),
            _ => Err(UnknownPlatformErr(format!("{}", platform))),
        }
    }
}

#[derive(Debug)]
pub(crate) enum UnknownTargetErr {
    CpuArch(UnknownCpuArchErr),
    Platform(UnknownPlatformErr),
    Total(String),
}

impl std::error::Error for UnknownTargetErr {}

impl ::core::fmt::Display for UnknownTargetErr {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        match self {
            UnknownTargetErr::CpuArch(e) => ::core::fmt::Display::fmt(e, f),
            UnknownTargetErr::Platform(e) => ::core::fmt::Display::fmt(e, f),
            UnknownTargetErr::Total(name) => write!(f, "unknwon target: {}", name),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash, Ord, PartialOrd)]
pub(crate) struct Target(CpuArch, Platform);

impl FromStr for Target {
    type Err = UnknownTargetErr;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.split_once('-') {
            None => Err(UnknownTargetErr::Total(s.to_owned())),
            Some((arch, platform)) => Ok(Target(
                arch.parse().map_err(UnknownTargetErr::CpuArch)?,
                platform.parse().map_err(UnknownTargetErr::Platform)?,
            )),
        }
    }
}

impl ::core::fmt::Display for Target {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> core::fmt::Result {
        ::core::fmt::Display::fmt(&self.0, f)?;
        f.write_str("-")?;
        ::core::fmt::Display::fmt(&self.1, f)
    }
}
