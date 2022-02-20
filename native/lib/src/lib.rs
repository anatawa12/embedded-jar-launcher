//! The (precompiled) library for embed java launcher
//!
//! To make it fast to build launcher, most parts of launcher will be implemented in this crate
//! and launcher will mostly have only configuration patterned file
//!
//! Copyright (c) anatawa12 and other contributors 2022

#![recursion_limit = "256"]

use std::path::PathBuf;

#[macro_export]
macro_rules! log {
    ($($tt:tt)*) => {
        $crate::launcher_lib_print(::core::format_args!($($tt)*));
    };
}

mod env;
pub mod searcher;
mod util;
pub mod jni;
pub mod launchers;

pub use util::launcher_lib_print;
pub type JvmFilterResult = Option<()>;

pub trait JvmFilter {
    /// returns None if this found best candidate, no more search is not required.
    fn try_add(&mut self, path: PathBuf) -> JvmFilterResult;
}

#[macro_export]
macro_rules! check_exclusive_cfg {
    (
        $key: ident = $($value: literal),* $(,)?
        : $multi_err: literal, $unknown_err: literal
    ) => {
        // if multiple feature specified
        $crate::check_exclusive_cfg!{
            @multi
            $key
            []
            [$($value),*]
            []
            [$multi_err]
        }

        #[cfg(all($(not($key = $value), )*))]
        ::std::compile_error!($unknown_err);
    };

    (
        @multi
        $key: ident
        [$($pre_value: literal),*]
        [$cur: literal $(, $post_value: literal)*]
        [$($made: meta),*]
        [$multi_err: literal]
    ) => {
        $crate::check_exclusive_cfg!{
            @multi
            $key
            [$($pre_value, )* $cur]
            [$($post_value),*]
            [
                $($made,)*
                all(
                    $key = $cur,
                    any(
                        $($key = $pre_value,)*
                        $($key = $post_value,)*
                    ),
                )
            ]
            [$multi_err]
        }
    };
    (
        @multi
        $key: ident
        [$($_value: literal),*]
        [ ]
        [$($made: meta),*]
        [$multi_err: literal]
    ) => {
        #[cfg(any($($made),*))]
        ::std::compile_error!($multi_err);
    };
}
