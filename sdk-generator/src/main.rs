// I can't use the macros in include!d file in intellij-rust.
// So I write macro directly. If this problem is resolved,
// I want to move macro definition to macros.rs later.
// See https://github.com/intellij-rust/intellij-rust/issues/8559,
//include!("macros.rs");
// region macros.rs
macro_rules! define_to_string {
    (
        $fn_name: ident <= $t: ty; $format: expr,
        $(use $uses0:ident :: $($uses: ident ::)* * ;)?
        [ $($name: ident,)* ]
    ) => {
        fn $fn_name(value: $t) -> String {
            $(use $uses0 :: $($uses ::)* * ;)?
            match value {
                $( $name => format!($format, stringify!($name)),)*
                i => format!($format, i),
            }
        }
    };
    (@ $($_1: ident)+ $name: ident) => {
        stringify($name)
    };
}

macro_rules! str_enum {
    (
        $(#[$attr: meta])*
        $enum_access: vis enum $enum_name: ident {
            $(#[$error_attr: meta])*
            $error_access: vis type Err = $error_name: ident ($error_format: literal);
            $( $variant_name: ident ( $variant_str: literal ) ),* $(,)?
        }
    ) => {
        $(#[$attr])*
        #[derive(::core::marker::Copy, ::core::clone::Clone)]
        #[derive(::core::cmp::Eq, ::core::cmp::PartialEq, ::core::hash::Hash)]
        #[derive(::core::cmp::Ord, ::core::cmp::PartialOrd)]
        $enum_access enum $enum_name {
            $($variant_name,)*
        }

        impl ::core::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    $(Self::$variant_name => f.write_str($variant_str),)*
                }
            }
        }

        impl ::core::fmt::Debug for $enum_name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Display::fmt(self, f)
            }
        }

        impl ::core::str::FromStr for $enum_name {
            type Err = $error_name;

            fn from_str(s: &str) -> ::core::result::Result<Self, Self::Err> {
                match s {
                    $($variant_str => Ok(Self::$variant_name),)*
                    other => Err($error_name(other.to_owned()))
                }
            }
        }

        simple_str_err! {
            $(#[$error_attr])*
            $error_access $error_name($error_format)
        }
    };
}

macro_rules! simple_str_err {
    ($(#[$attr: meta])* $access: vis $name: ident($format: literal)) => {
        #[derive(::core::fmt::Debug)]
        $(#[$attr])*
        $access struct $name(String);

        impl std::error::Error for $name {}

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                write!(f, $format, &self.0)
            }
        }
    };
}

macro_rules! common_impl {
    (@variants [ $( ( $($variant: tt)+ ) )* ] [ ] for $head: ident $($tt:tt)* ) => {
        common_impl!{@variants [ $( ( $($variant)+ ) )* ] [ $head ] for $($tt)* }
    };

    (@variants [ $( ( $($variant: tt)+ ) )* ] [ $($heading: tt)* ] for :: $tail: ident $($tt:tt)* ) => {
        common_impl!{@variants [ $( ( $($variant)+ ) )* ] [ $($heading)* :: $tail ] for $($tt)* }
    };

    (@variants [ $( ( $($variant: tt)+ ) )* ] [ $($new_variant: tt)+ ] for, $($tt:tt)* ) => {
        common_impl!{@variants [ $( ( $($variant)+ ) )* ( $($new_variant)+ ) ] [ ] for $($tt)* }
    };

    (@variants [ $( ( $($variant: tt)+ ) )* ] [ $($new_variant: tt)+ ] for { $($tt:tt)* } ) => {
        common_impl!{@fn [ $( ( $($variant)+ ) )* ( $($new_variant)+ ) ] [] $($tt)* }
    };

    (@variants [ $( ( $($variant: tt)+ ) )* ] [ ] for { $($tt:tt)* } ) => {
        common_impl!{@fn [ $( ( $($variant)+ ) )*] [] $($tt)* }
    };



    (
        @fn
        [ $( ( $($variant: tt)+ ) )* ]
        [$($fn_mod: tt)*]
        $access: vis unsafe $($tt:tt)*
    ) => {
        common_impl!{ @fn [ $( ( $($variant)+ ) )* ] [unsafe] $access $($tt)* }
    };
    (
        @fn
        [ $( ( $($variant: tt)+ ) )* ]
        [ $($fn_mod: tt)* ]
        $access: vis fn $fn_name: ident
            $(<
                $($lf: lifetime $(: $lb0: lifetime $(+ $lb: lifetime)*)?),*
                $(,)?
                $($tp: ident $(: $tb0: path  /* $(+ $tb: ty)* */)?),*
            >)?
            (& $($self_life: lifetime)? $($self_mod: ident)+ $(. $self_suf: tt)* $(, $arg_name: ident: $arg_type: ty )* $(,)?)
            -> $returns: ty
            $(where $($attr: ident),+ $(,)?)?
            ;
        $($tt:tt)*
    ) => {
        #[allow(dead_code)]
        $access $($fn_mod)* fn $fn_name
            $(<
                $( $lf $(: $lb0 $(+ $lb)*)?, )*
                $( $tp $(: $tb0 /* $(+ $tb)* */)?, )*
            >)?
            ( self: common_impl!( @self Self $($self_life)? $($self_mod)+ ) , $( $arg_name: $arg_type),*) -> $returns {
            common_impl!{ @match
                [self $(. $self_suf)*]
                [ $($self_mod)+ ]
                [ $( ( $($variant)+ ) )*]
                [ $($($attr)+)? ]
                { $fn_name ($($arg_name)*) }
            }
        }

        common_impl!{ @fn [ $( ( $($variant)+ ) )* ] [] $($tt)* }
    };
    (@fn [$($variant: tt)*] [$($fn_mod: tt)*]) => {
    };

    (@match
        [ $s: expr ]
        [ $($self_mod: ident)+ ]
        [ ( $simple_variant: ident ) $($variants: tt)* ]
        [ $($attr: ident)* ]
        { $fn_name:ident ($($arg_name: ident)*) }
        $($rest: tt)*
    ) => {
        common_impl!{@match
            [$s]
            [$($self_mod)+]
            [$($variants)*]
            [ $($attr)* ]
            { $fn_name ($($arg_name)*) }
            ( [$($self_mod)+] [ $($attr)* ] ( Self::$simple_variant ) $fn_name ($($arg_name)*) )
            $($rest)*
        }
    };
    (@match
        [ $s: expr ]
        [ $($self_mod: ident)+ ]
        [ ( $($complex_variant: tt)+ ) $($variants: tt)* ]
        [ $($attr: ident)* ]
        { $fn_name:ident ($($arg_name: ident)*) }
        $($rest: tt)*
    ) => {
        common_impl!{@match
            [$s]
            [$($self_mod)+]
            [$($variants)*]
            [ $($attr)* ]
            { $fn_name ($($arg_name)*) }
            ( [$($self_mod)+] [ $($attr)* ] ( $($complex_variant)+ ) $fn_name ($($arg_name)*) )
            $($rest)*
        }
    };
    (@match
        [$s: expr]
        [$($_self_mod: ident)+]
        []
        [ $($_attr: ident)* ]
        { $($_def:tt)* }
        $(
            ( [$($self_mod: ident)+] [ $($attr:ident)* ] ( $($variant: tt)+ ) $fn_name: ident ($($arg_name: ident)*))
        )*
    ) => {
        match $s {
            $($($variant)+ (common_impl!( @ref v $($self_mod)+ )) => common_impl!(@where v.$fn_name($($arg_name),*); $($attr)*),)*
        }
    };

    (@self $self: ident $($life: lifetime)? mut self) => {
        &$($life)? mut Self
    };
    (@self $self: ident $($life: lifetime)? self) => {
        &$($life)? Self
    };

    (@ref $v: ident mut self) => {
        ref mut $v
    };
    (@ref $v: ident self) => {
        ref $v
    };

    (@where $v: expr; ) => {
        $v.into()
    };
    (@where $v: expr; result $($attr:ident)*) => {
        common_impl!(@where $v.map(::core::convert::Into::into); $($attr)*)
    };
    (@where $v: expr; $attr:ident $($_attr:ident)*) => {
        ::core::compile_error!(::core::concat!("unknown attribute: ", ::core::stringify!($attr)))
    };


    (@$kind:ident $($tt:tt)* ) => {
        ::core::compile_error!(::core::concat!("internal macro error ", ::core::stringify!($kind)))
    };

    ( $($tt:tt)* ) => {
        common_impl!{@variants [] [] $($tt)* }
    };
}

macro_rules! impl_from {
    (@generic [$($generic:tt)+] [$($nest: ident)*] < $($tt:tt)*) => {
        impl_from! { @generic [$($generic)+ <] [ $($nest:ident)+ nest] $($tt)* }
    };
    (@generic [$($generic:tt)+] [nest $($nest: ident)*] > $($tt:tt)*) => {
        impl_from! { @generic [$($generic)+ >] [ $($nest:ident)+ ] $($tt)* }
    };
    (@generic [$($generic:tt)+] [] > $($tt:tt)*) => {
        impl_from! { @impl [$($generic)+ >] $($tt)* }
    };
    (@generic [$($generic:tt)+] [$($nest: ident)*] $token: tt $($tt:tt)*) => {
        impl_from! { @generic [$($generic)+ $token] [ $($nest:ident)* ] $($tt)* }
    };
    (@generic [$($generic:tt)+] [$($nest: ident)*] ) => {
        ::core::compile_error!{::core::concat!("unexpected end of macro call")}
    };

    (@impl [$($generic:tt)*] | $arg: ident: $from_t: ty | -> $to_t: ty { $expr: expr }) => {
        impl $($generic)* From<$from_t> for $to_t {
            fn from($arg: $from_t) -> $to_t {
                $expr
            }
        }
    };
    (@impl [$($generic:tt)*] $($tt:tt)*) => {
        ::core::compile_error!{::core::concat!("unexpected macro call")}
    };

    /*
    ($(<$($generic: tt)+>)? | $arg: ident: $from_t: ty | -> $to_t: ty { $expr: expr }) => {
        impl$(<$($generic)+>)? From<$from_t> for $to_t {
            fn from($arg: $from_t) -> to_t {
                $expr
            }
        }
    };
     */

    (@$kind:ident $($tt:tt)* ) => {
        ::core::compile_error!(::core::concat!("internal macro error ", ::core::stringify!($kind)))
    };


    // entrypoint
    (< $($tt:tt)*) => {
        impl_from! { @generic [<] [] $($tt)* }
    };
    ($($tt:tt)*) => {
        impl_from! { @impl [] $($tt)* }
    };
}
// endregion macros.rs

mod dylib_name;
mod mach_common;
mod mach_o_parse;
mod text_api;
mod util;
mod writer;
mod symlinks;

use std::collections::HashSet;
use crate::mach_o_parse::collect_dylib_of_macho;
use crate::text_api::{generate_text_api, DylibInfo, Platform};

use crate::mach_common::{CommonFatHeader, CommonMachHeader};
use anyhow::{Context, Result};
use clap::Parser;
use log::*;
use memmap::MmapOptions;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use relative_path::{RelativePath, RelativePathBuf};
use crate::symlinks::{parse_symlinks_file, SymlinkDescriptor};
use crate::writer::{ClosableWrite, FileTree};

#[derive(Debug, Parser)]
struct Options {
    /// specify default target platform. if it's specified in mach-o,
    /// the platform by mach-o will be used.
    #[clap(short, long)]
    platform: Option<Platform>,
    /// add symlink in sdk.
    /// the original can be relative path from original
    #[clap(short = 'l', long, value_name = "LINK:ORIGINAL")]
    symlink: Vec<SymlinkDescriptor>,
    /// add symlink in sdk from a file
    /// each line of this file should have same format as --symlink.
    /// the line starts with '#' will be ignored as comment.
    #[clap(long = "symlinks-file")]
    symlink_file: Vec<PathBuf>,
    /// the destination directory or file.
    #[clap(long = "dest", short = 'd')]
    destination: PathBuf,
    /// the format of destination sdk
    #[clap(long = "format", short = 'f', value_name = "dir|tar|tgz", default_value_t = SdkFormat::Dir)]
    destination_format: SdkFormat,
    /// the list of mach-o executable wil be parsed.
    executables: Vec<String>,
}

fn main() -> Result<()> {
    ::simple_logger::SimpleLogger::new()
        .with_level(
            LevelFilter::from_str(
                &std::env::var("SDK_GEN_LEVEL")
                    .as_ref()
                    .map(String::as_str)
                    .unwrap_or("WARN"),
            )
            .unwrap(),
        )
        .init()?;

    let options: Options = Options::parse();

    info!("using options: {:#?}", options);

    // open and parse mach-o
    let files = options
        .executables
        .iter()
        .map(|path| {
            let path_ref = AsRef::<Path>::as_ref(&path);
            trace!("opening {}", path_ref.display());
            let file =
                File::open(&path_ref).with_context(|| format!("reading {}", path_ref.display()))?;
            Ok((path, unsafe { MmapOptions::new().map(&file)? }))
        })
        .collect::<Result<Vec<_>>>()
        .context("opening mach-o")?;

    let mut dylib_list = Vec::new();

    for (path, file) in &files {
        trace!("reading {}", AsRef::<Path>::as_ref(&path).display());
        dylib_list.append(&mut collect_dylib_of_macho(file.as_ref())
            .context("parsing mach-o")?);
    }

    let unique_dylib_list = DylibInfo::unique_dylib(dylib_list);

    // parse symlinks file
    let mut symlinks = options.symlink_file
        .into_iter()
        .map(parse_symlinks_file)
        .flat_map(MayErr::new)
        .chain(options.symlink.into_iter().map(Ok))
        .collect::<Result<HashSet<SymlinkDescriptor>, _>>()
        .context("reading symlinks file")?;

    let mut tree = writer::new_tree(&options.destination, options.destination_format)
        .context("opening destination")?;

    for dylib in &unique_dylib_list {
        let path: RelativePathBuf = format!("{}.tbd", dylib.name.trim_end_matches(".dylib")).into();
        if let Some(parent) = path.parent() {
            tree.mkdirp(parent).context("creating dir")?;
        }
        let mut file = tree.new_file(&path).context("creating tbd file")?;
        generate_text_api(options.platform, &dylib, &mut file).context("writing tbd")?;
        file.close().context("writing tbd")?;
        if let Some(no_ver) = lib_name_without_version(&path) {
            symlinks.insert(SymlinkDescriptor::new(no_ver, path));
        }
    }

    for x in symlinks {
        if let Some(parent) = x.link().parent() {
            tree.mkdirp(parent).context("creating dir")?;
        }
        tree.new_file_symlink(x.original(), x.link())
            .context("creating symkink")?;
    }

    tree.finish()?;
    /*
    generate_sdk(Path::new(&sdk_root), &unique_dylib_list)?;
    // */

    #[cfg(only_for_debug)]
    // this is for debugging. you can enable this block via commenting cfg line
    for (_, dylib) in &unique_dylib_list {
        println!("{}: ", dylib.name);
        println!(
            "  version {}; compatible {}",
            dylib.current_version, dylib.compatibility_version
        );
        for x in &dylib.symbols {
            println!("    {}", x.name);
        }
    }

    #[cfg(only_for_debug)]
    // this is for debugging. you can enable this block via commenting cfg line
    for x in symbols {
        println!("symbol {} by {}", x.name, x.dylib_name);
    }

    Ok(())
}

fn lib_name_without_version(path: impl AsRef<RelativePath>) -> Option<RelativePathBuf> {
    let path = path.as_ref().as_str();
    let (parent, filename) = path.rsplit_once('/').unwrap_or(("", path));
    let (stem, ext) = filename.rsplit_once('.')?;
    let (lib_name, _version) = stem.rsplit_once('.')?;
    Some(format!("{}/{}.{}", parent, lib_name, ext).into())
}

/// the struct to handle a function returns Result<Iterator<Item=Result<Item, Err>>, Err>
enum MayErr<I, E> {
    Iterator(I),
    Error(E),
    Finished,
}

impl <I, E> MayErr<I, E> {
    fn new(r: Result<I, E>) -> Self {
        match r {
            Ok(i) => MayErr::Iterator(i),
            Err(e) => MayErr::Error(e),
        }
    }
}

impl <I, E, V> Iterator for MayErr<I, E>
    where I : Iterator<Item = Result<V, E>> {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            MayErr::Iterator(iter) => iter.next(),
            MayErr::Error(_) => {
                match std::mem::replace(self, MayErr::Finished) {
                    MayErr::Iterator(_) => unreachable!(),
                    MayErr::Finished => unreachable!(),
                    MayErr::Error(e) => Some(Err(e))
                }
            }
            MayErr::Finished => None
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum SdkFormat {
    Dir,
    Tar,
    Tgz,
    Zip,
}

#[derive(Debug)]
struct SdkFormatParseErr(());

impl std::error::Error for SdkFormatParseErr {}

impl std::fmt::Display for SdkFormatParseErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("unknown sdk format")
    }
}

impl Default for SdkFormat {
    fn default() -> Self {
        Self::Dir
    }
}

impl FromStr for SdkFormat {
    type Err = SdkFormatParseErr;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "d" | "dir" | "directory" => Ok(Self::Dir),
            "t" | "tar" => Ok(Self::Tar),
            "tg" | "tgz" | "tar.gz" => Ok(Self::Tgz),
            "z" | "zip" => Ok(Self::Zip),
            _ => Err(SdkFormatParseErr(())),
        }
    }
}

impl std::fmt::Display for SdkFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SdkFormat::Dir => f.write_str("dir"),
            SdkFormat::Tar => f.write_str("tar"),
            SdkFormat::Tgz => f.write_str("tgz"),
            SdkFormat::Zip => f.write_str("zip"),
        }
    }
}
