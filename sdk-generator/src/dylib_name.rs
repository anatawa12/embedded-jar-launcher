// A/Foo
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn guess_library_short_name(name: &str) -> &str {
    // Pull off the last component and make Foo point to it
    let (pre, dylib_name) = match name.rsplit_once('/') {
        None | Some(("", _)) => return guess_library(name),
        Some(p) => p,
    };

    // Look for a suffix starting with a '_'
    let dylib_name = strip_suffix(dylib_name);

    // First look for the form Foo.framework/Foo
    if name.ends_with(&format!("{0}.framework/{0}", dylib_name)) {
        return dylib_name;
    }

    // Next look for the form Foo.framework/Versions/A/Foo
    let (pre, _version) = match pre.rsplit_once('/') {
        None => return guess_library(name),
        Some(p) => p,
    };
    if pre.ends_with(&format!("{0}.framework/Versions", dylib_name)) {
        return dylib_name;
    }

    return guess_library(name);
}

fn strip_suffix(name: &str) -> &str {
    match name.rsplit_once('_') {
        Some((pre, "debug")) | Some((pre, "profile")) => pre,
        _ => name,
    }
}

fn strip_version_number(name: &str) -> &str {
    if name.len() >= 3 && name.as_bytes()[name.len() - 2] == b'.' {
        &name[..(name.len() - 2)]
    } else {
        name
    }
}

fn guess_library(name: &str) -> &str {
    // pull off the suffix after the "." and make a point to it
    match name.strip_suffix(".dylib") {
        None => guess_qtx(name),
        Some(name) => {
            // First pull off the version letter for the form Foo.A.dylib if any.
            let name = strip_version_number(name);

            let name = name.rsplit_once('/').unwrap_or(("", name)).1;
            let name = strip_suffix(name);
            // There are incorrect library names of the form:
            // libATS.A_profile.dylib so check for these.
            let name = strip_version_number(name);

            name
        }
    }
}

fn guess_qtx(name: &str) -> &str {
    match name.strip_suffix(".qtx") {
        None => "",
        Some(name) => {
            let name = name.rsplit_once('/').unwrap_or(("", name)).1;
            let name = strip_version_number(name);
            name
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_strip_version_number() {
        assert_eq!(strip_version_number("hello.B"), "hello");
        assert_eq!(strip_version_number("hello.1"), "hello");
        assert_eq!(strip_version_number("libavformat.58"), "libavformat.58");
    }

    #[test]
    fn test_guess_library_short_name() {
        assert_eq!(
            guess_library_short_name("/usr/lib/libSystem.B.dylib"),
            "libSystem"
        );
        assert_eq!(
            guess_library_short_name(
                "/System/Library/Frameworks/CoreFoundation.framework/Versions/A/CoreFoundation"
            ),
            "CoreFoundation"
        );
        assert_eq!(
            guess_library_short_name(
                "/opt/homebrew/Cellar/ffmpeg/4.4.1_5/lib/libavformat.58.dylib"
            ),
            "libavformat.58"
        );
    }
}
