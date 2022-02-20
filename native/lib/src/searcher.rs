use crate::env::{exe_dir, home_dir, working_dir};
use crate::{JvmFilter, JvmFilterResult};
use cfg_if::cfg_if;
use std::borrow::Cow;
use std::env::var_os as env_var_os;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

const MAIN_SEPARATOR_BYTE: u8 = ::std::path::MAIN_SEPARATOR as u8;

//region basic

/// check if the directories matches [glob] is jre or not.
#[allow(dead_code)]
pub fn glob(finder: &mut impl JvmFilter, glob: &str) -> JvmFilterResult {
    log!("finding glob: {}", glob);
    ::glob::glob(make_full_path(glob)?.as_ref())
        .unwrap()
        .flat_map(Result::ok)
        .try_fold((), |_, v| finder.try_add(v))
}

/// check if [path] is jre or not
#[allow(dead_code)]
pub fn check(finder: &mut impl JvmFilter, path: &str) -> JvmFilterResult {
    log!("trying: {}", path);
    finder.try_add(to_path_buf(make_full_path(path)?))
}

/// check `JAVA_HOME` environment variable
#[allow(dead_code)]
pub fn java_home(finder: &mut impl JvmFilter) -> JvmFilterResult {
    log!("trying $JAVA_HOME");
    finder.try_add(PathBuf::from(env_var_os("JAVA_HOME")?))
}

/// Extract and use embed jre zip file.
///
/// [Temurin]: https://adoptium.net/
#[allow(dead_code)]
pub fn embed_zip(finder: &mut impl JvmFilter, bytes: &[u8], in_archive: &str) -> JvmFilterResult {
    log!("extracting zip");
    extract_archive(
        finder,
        ZipArchive::new(Cursor::new(bytes)).ok()?,
        in_archive,
        |mut archive, path| archive.extract(&path).ok(),
    )
}

/// Extract and use embed jre tar file.
///
/// [Temurin]: https://adoptium.net/
#[allow(dead_code)]
pub fn embed_tar(finder: &mut impl JvmFilter, bytes: &[u8], in_archive: &str) -> JvmFilterResult {
    log!("extracting tar");
    extract_archive(
        finder,
        tar::Archive::new(Cursor::new(bytes)),
        in_archive,
        |mut archive, path| archive.unpack(&path).ok(),
    )
}

/// Extract and use embed jre tgz(tar.gz) file.
///
/// [Temurin]: https://adoptium.net/
#[allow(dead_code)]
pub fn embed_tgz(finder: &mut impl JvmFilter, bytes: &[u8], in_archive: &str) -> JvmFilterResult {
    log!("extracting tgz");
    extract_archive(
        finder,
        tar::Archive::new(flate2::read::GzDecoder::new(Cursor::new(bytes))),
        in_archive,
        |mut archive, path| archive.unpack(&path).ok(),
    )
}

//endregion

//region package mangers (sorted by name)

//noinspection SpellCheckingInspection
/// support for [asdf]
///
/// [asdf]: http://asdf-vm.com/
#[allow(dead_code)]
pub fn asdf(finder: &mut impl JvmFilter) -> JvmFilterResult {
    log!("finding asdf");
    find_jdk_in_dir(
        finder,
        env_var_os("ASDF_DATA_DIR")
            .map(PathBuf::from)
            .or_else(|| Some(Path::new(home_dir()?).join(".asdf")))?
            .join("installs/java"),
    )
}

//noinspection SpellCheckingInspection
/// support for [jabba]
///
/// [jabba]: https://github.com/shyiko/jabba
#[allow(dead_code)]
pub fn jabba(finder: &mut impl JvmFilter) -> JvmFilterResult {
    log!("finding jabba");
    find_jdk_in_dir(
        finder,
        env_var_os("JABBA_HOME")
            .map(PathBuf::from)
            .or_else(|| Some(Path::new(home_dir()?).join(".jabba")))?
            .join("jdk"),
    )
}

/// support for [sdkman]
///
/// [sdkman]: https://sdkman.io/
#[allow(dead_code)]
pub fn sdkman(finder: &mut impl JvmFilter) -> JvmFilterResult {
    log!("finding sdkman");
    let sdkman_path = env_var_os("SDKMAN_CANDIDATES_DIR")
        .map(PathBuf::from)
        .or_else(|| Some(Path::new(home_dir()?).join(".sdkman/candidates")))?
        .join("java");

    // first, try current version
    let current = sdkman_path.join("current");
    finder.try_add(current)?;

    // otherwise try all installed versions
    find_jdk_in_dir(finder, sdkman_path)
}

// endregion

// region os-specific finder

#[allow(dead_code)]
pub fn os_specific(finder: &mut impl JvmFilter) -> JvmFilterResult {
    cfg_if! {
        if #[cfg(target_os = "linux")] {
            linux_specific(finder)
        } else if #[cfg(target_os = "macos")] {
            macos_specific(finder)
        } else if #[cfg(target_os = "windows")] {
            windows_specific(finder)
        } else {
            None
        }
    }
}

#[cfg(target_os = "linux")]
pub fn linux_specific(finder: &mut impl JvmFilter) -> JvmFilterResult {
    log!("finding linux locations");
    ["/usr/lib/jvm", "/usr/java"]
        .into_iter()
        .try_fold((), |_, v| find_jdk_in_dir(finder, v))
}

#[cfg(target_os = "macos")]
pub fn macos_specific(finder: &mut impl JvmFilter) -> JvmFilterResult {
    use std::io::BufRead;
    use std::process::{Command, Stdio};

    log!("finding using /usr/libexec/java_home");

    let output = Command::new("/usr/libexec/java_home")
        .arg("-V")
        .stdout(Stdio::null())
        .stdin(Stdio::null())
        .stdin(Stdio::piped())
        .spawn()
        .ok()?
        .wait_with_output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    std::io::BufReader::new(&mut output.stderr.as_slice())
        .lines()
        .filter_map(|x| x.ok())
        .by_ref()
        .filter_map(|x| {
            x.split_once(" /")
                .map(|(_, path)| PathBuf::from(format!("/{}", path)))
        })
        .try_fold((), |_, v| finder.try_add(v))
}

#[cfg(target_os = "windows")]
pub fn windows_specific(finder: &mut impl JvmFilter) -> JvmFilterResult {
    log!("finding using windows registry");

    use winreg::enums::*;
    use winreg::*;
    use std::ffi::OsString;

    struct SdkFinder<'com> {
        keys: EnumKeys<'com>,
        key: &'com RegKey,
        path: &'com str,
    }

    impl<'key: 'common, 'path: 'common, 'common> SdkFinder<'common> {
        fn new(key: &'key RegKey, path: &'path str) -> Self {
            Self {
                keys: key.enum_keys(),
                key,
                path,
            }
        }
    }

    impl<'com> Iterator for SdkFinder<'com> {
        type Item = PathBuf;

        fn next(&mut self) -> Option<Self::Item> {
            while let Some(x) = self.keys.next() {
                if let Some(mut version) = x.ok() {
                    version.push_str(&self.path);
                    if let Some(value) = self.key.get_value::<OsString, _>(version).ok() {
                        return Some(PathBuf::from(value));
                    }
                }
            }
            None
        }
    }

    let hjlm = RegKey::predef(HKEY_LOCAL_MACHINE);

    let keys = [
        (
            new_key(&hjlm, "SOFTWARE\\AdoptOpenJDK\\JDK"),
            "\\hotspot\\MSI\\Path",
        ),
        (new_key(&hjlm, "SOFTWARE\\JavaSoft\\JDK"), "\\JavaHome"),
        (
            new_key(&hjlm, "SOFTWARE\\JavaSoft\\Java Development Kit"),
            "\\JavaHome",
        ),
        (
            new_key(&hjlm, "SOFTWARE\\JavaSoft\\Java Runtime Environment"),
            "\\JavaHome",
        ),
        (
            new_key(
                &hjlm,
                "SOFTWARE\\Wow6432Node\\JavaSoft\\Java Development Kit",
            ),
            "\\JavaHome",
        ),
        (
            new_key(
                &hjlm,
                "SOFTWARE\\Wow6432Node\\JavaSoft\\Java Runtime Environment",
            ),
            "\\JavaHome",
        ),
    ];

    return keys
        .iter()
        .filter_map(|(k, v)| k.as_ref().map(|k| (k, *v)))
        .flat_map(|(k, v)| SdkFinder::new(k, v))
        .try_fold((), |_, v| finder.try_add(v));

    fn new_key(hjlm: &RegKey, key: &str) -> Option<RegKey> {
        hjlm.open_subkey_with_flags(key, KEY_QUERY_VALUE | KEY_ENUMERATE_SUB_KEYS)
            .ok()
    }
}

// endregion

// region utilities

fn find_jdk_in_dir(finder: &mut impl JvmFilter, dir: impl AsRef<Path>) -> JvmFilterResult {
    let dir = dir.as_ref();
    log!("finding in {}", dir.display());
    fs::read_dir(dir)
        .ok()
        .into_iter()
        .flat_map(|x| x.into_iter())
        .filter_map(|x| x.ok())
        // skip symlink but keep unknown
        .filter(|x| x.file_type().map(|x| !x.is_symlink()).unwrap_or(true))
        .map(|x| x.path())
        .try_fold((), |_, v| finder.try_add(v))
}

fn make_full_path(path: &str) -> Option<Cow<str>> {
    debug_assert!(path.len() >= 2);
    if path.as_bytes()[1] == MAIN_SEPARATOR_BYTE {
        match path.as_bytes()[0] {
            b'~' => home_dir().map(|x| Cow::Owned(format!("{}{}", x, path.split_at(2).1))),
            b'.' => working_dir().map(|x| Cow::Owned(format!("{}{}", x, path.split_at(2).1))),
            b'^' => exe_dir().map(|x| Cow::Owned(format!("{}{}", x, path.split_at(2).1))),
            _ => panic!(),
        }
    } else {
        assert!(Path::new(path).is_absolute());
        Some(Cow::Borrowed(path))
    }
}

fn extract_archive<A>(
    finder: &mut impl JvmFilter,
    archive: A,
    in_archive: &str,
    extract: impl FnOnce(A, &Path) -> JvmFilterResult,
) -> JvmFilterResult {
    let dir = tempfile::tempdir().ok()?;
    let mut path = dir.path().to_path_buf();
    std::mem::forget(dir); // do not drop TempDir
    log!("extracting to {}", path.display());
    extract(archive, &path)?;
    path.push(in_archive);
    log!("striped: {}", path.display());
    finder.try_add(path)
}

fn to_path_buf(path: Cow<str>) -> PathBuf {
    match path {
        Cow::Owned(owned) => PathBuf::from(owned),
        Cow::Borrowed(borrowed) => PathBuf::from(borrowed),
    }
}

// endregion
