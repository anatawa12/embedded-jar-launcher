// SPDX-License-Identifier: Unlicense OR CC0-1.0
// (c) anatawa12 2021
// JVM Launcher

mod tempfile;

use std::env;
use std::path::PathBuf;
use is_executable::is_executable;
use std::process::{exit, Command};
use std::io::Write;
use std::env::consts::EXE_SUFFIX;
use std::fs::{File, create_dir_all};
use std::env::args_os;
use tempfile::create_temp_jar;

macro_rules! debug {
    ($debug:expr, $($pat:expr),*) => {
        if ($debug) {
            eprintln!($($pat),*);
        }
    };
}

#[cfg(windows)]
const PATH_SEPARATOR: char = ';';
#[cfg(any(target_os = "redox", unix, target_os = "vxworks", target_os = "hermit"))]
const PATH_SEPARATOR: char = ':';

const JAR_FILE_BODY: &'static [u8] = include_bytes!("../resources/all.jar");

const MAIN_CLASS_NAME: &'static str = include_str!("../resources/main_class_name.txt");

fn main() {
    let debug = env::var_os("JAVA_WRAPPER_DEBUG").is_some();
    let java_command = match infer_java_command(debug) {
        Some(path) => path,
        None => {
            eprintln!("java command not found.");
            eprintln!("make sure valid JAVA_HOME is set or put java command in PATH.");
            exit(101);
        },
    };
    debug!(debug, "java command: {}", &java_command.display());
    let jar_path = create_temp_jar(debug).unwrap();
    debug!(debug, "jar file: {}", &jar_path.display());
    {
        create_dir_all(jar_path.parent().unwrap()).unwrap();
        save_jar(&mut File::create(&jar_path).unwrap());
    }

    let options = if debug { "-debug " } else { "" };
    let mut command = Command::new(java_command);
    command.arg("-jar")
        .arg(&jar_path)
        .arg(&options)
        .arg(&jar_path)
        .arg(MAIN_CLASS_NAME)
        .args(args_os())
        ;


    #[cfg(not(any(target_os = "redox", unix, target_os = "vxworks", target_os = "hermit")))]
    {
        debug!(debug, "launching java command: {:?}", &command);
        let status = command.status().unwrap();
        exit(status.code().unwrap());
    }

    #[cfg(any(target_os = "redox", unix, target_os = "vxworks", target_os = "hermit"))]
    {
        use std::os::unix::process::CommandExt;
        debug!(debug, "launching java command with exec: {:?}", &command);
        let err = command.exec();
        panic!("{}", err);
    }
}

fn save_jar(file: &mut dyn Write) {
    file.write(JAR_FILE_BODY).unwrap();
    file.flush().unwrap();
}

fn infer_java_command(debug: bool) -> Option<PathBuf> {
    match infer_java_command_from_java_home(debug) {
        Some(java_command) => {
            return Some(java_command);
        }
        None => {}
    };
    match infer_java_command_from_path(debug) {
        Some(java_command) => {
            return Some(java_command);
        }
        None => {}
    };
    debug!(debug, "java command not found");
    return None;
}

fn infer_java_command_from_java_home(debug: bool) -> Option<PathBuf> {
    let java_home = PathBuf::from(env::var_os("JAVA_HOME")?);
    debug!(debug, "JAVA_HOME found!: {}", &java_home.display());

    let java_cmd = java_home.join("bin").join(format!("java{}", EXE_SUFFIX));
    debug!(debug, "finding java command: {}", java_cmd.display());

    if is_executable(&java_cmd) {
        debug!(debug, "executable java: {}", java_cmd.display());
        return Some(java_cmd);
    }

    debug!(debug, "not a java: {}", java_cmd.display());
    return None;
}

fn infer_java_command_from_path(debug: bool) -> Option<PathBuf> {
    let path = env::var("PATH").ok()?;
    debug!(debug, "PATH found!: {}", path);

    for path_component_slice in path.split(PATH_SEPARATOR) {
        debug!(debug, "finding java command in PATH: {}", path_component_slice);

        let mut path_component = PathBuf::from(path_component_slice);
        path_component.push(format!("java{}", EXE_SUFFIX));

        if is_executable(&path_component) {
            debug!(debug, "executable java: {}", path_component.display());
            return Some(path_component)
        }
        debug!(debug, "not a java: {}", path_component.display());
    }
    return None;
}
