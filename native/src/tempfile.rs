use std::{env, io};
use std::ffi::{OsStr, OsString};
use std::path::{PathBuf};
use rand::Rng;
use rand::distributions::Alphanumeric;

macro_rules! debug {
    ($debug:expr, $($pat:expr),*) => {
        if ($debug) {
            eprintln!($($pat),*);
        }
    };
}

/// Create a new temporary file or directory with custom parameters.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Builder<'a, 'b> {
    random_len: usize,
    prefix: &'a OsStr,
    suffix: &'b OsStr,
    append: bool,
}

fn temp_name(suffix: &OsStr, rand_len: usize) -> OsString {
    let mut buf = OsString::with_capacity(suffix.len() + rand_len);

    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(rand_len)
        // Alphanumeric so must be safe
        .for_each(|b| unsafe { buf.push(std::str::from_utf8_unchecked(&[b as u8])) });

    buf.push(suffix);
    buf
}

pub fn create_temp_jar(debug: bool) -> io::Result<PathBuf> {
    let base = &env::temp_dir();

    for _ in 0..(1 << 31 - 1) {
        let path = base.join(temp_name(OsStr::new(".jar"), 6));
        if !path.exists() {
            return Ok(path);
        }
        debug!(debug, "already exists: {}", path.display())
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("too many temporary files exist at: {}", base.display()),
    ))
}
