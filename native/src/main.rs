//! The embed java launcher
//! This is a example, for-test project.
//! The `main.rs` will be used in the production will be automatically generated.
//!
//! Copyright (c) anatawa12 and other contributors 2022

extern crate core;

use launcher_lib::{log, JvmFilter, JvmFilterResult};
use std::env::args;
use std::path::{Path, PathBuf};

launcher_lib::check_exclusive_cfg! {
    launch = "exec", "spawn", "jni",
    : "multiple launch method specified",
    "no launch method specified"
}

// TODO: extract jars to some place

fn main() {
    let mut finder = JvmFilterImpl(None);
    search_jre(&mut finder);
    let path = finder.0.expect("jre not found");
    log!("found sdk: {:?}", path);

    #[cfg(launch = "exec")]
    use launcher_lib::launchers::launch_via_exec as launch;
    #[cfg(launch = "spawn")]
    use launcher_lib::launchers::launch_via_spawn as launch;
    #[cfg(launch = "jni")]
    use launcher_lib::launchers::launch_via_jni as launch;

    launch(
        path,
        &["-Xmx1G"],
        "com.anatawa12.embed.Test",
        args().skip(1)
    )
}

struct JvmFilterImpl(Option<PathBuf>);

impl JvmFilterImpl {
    fn is_jre(path: &Path) -> bool {
        #[inline]
        fn is_jre(path: &Path) -> Option<()> {
            // TODO: improve search method for each launch method.
            // TODO: support for minimum java version requirements
            let exec = path.join("bin/java");
            if exec.is_file() {
                Some(())
            } else {
                None
            }
        }
        is_jre(path).is_some()
    }
}

impl JvmFilter for JvmFilterImpl {
    fn try_add(&mut self, path: PathBuf) -> JvmFilterResult {
        match &self.0 {
            Some(_) => None,
            None => {
                if JvmFilterImpl::is_jre(&path) {
                    log!("{} is jre", path.display());
                    self.0 = Some(path);
                    None
                } else {
                    log!("{} is not jre", path.display());
                    Some(())
                }
            }
        }
    }
}

fn search_jre(filter: &mut impl JvmFilter) -> Option<()> {
    #[allow(unused)]
    #[macro_export]
    macro_rules! search {
        ($searcher: ident: $($expr: expr),* $(,)?) => {
            ::launcher_lib::searcher::$searcher(filter, $($expr),*)?
        };
        ($searcher: ident) => {
            ::launcher_lib::searcher::$searcher(filter)?
        };
    }

    //search!(glob: "~/Library/Application Support/minecraft/runtime/*/*/*/*/Contents/Home");
    search!(check: "/Library/Java/JavaVirtualMachines/jdk-17.0.1.jdk/Contents/Home");
    search!(sdkman);
    search!(asdf);
    search!(os_specific);
    #[cfg(target_os = "windows")]
    search!(
        embed_zip:
            include_bytes!(
                "/Users/anatawa12/Downloads/OpenJDK8U-jre_x64_windows_hotspot_8u322b06.zip"
            ),
        "jdk8u322-b06-jre",
    );
    #[cfg(target_os = "macos")]
    search!(
        embed_tgz:
            include_bytes!(
                "/Users/anatawa12/Downloads/OpenJDK8U-jre_x64_mac_hotspot_8u322b06.tar.gz"
            ),
        "jdk8u322-b06-jre/Contents/Home"
    );

    Some(())
}

#[no_mangle]
fn _launcher_lib_print(_args: core::fmt::Arguments<'_>) {
    eprintln!("embedded-jar-launcher: {}", _args);
}
