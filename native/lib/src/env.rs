
use crate::util::MayUninitOrNone;
use std::path::Path;

macro_rules! define_cached {
    ($name: ident -> $ty: ty = $expr: expr) => {
        #[inline]
        pub(crate) fn $name() -> Option<&'static $ty> {
            static mut CACHE: MayUninitOrNone<$ty> = MayUninitOrNone::<$ty>::uninitialized();
            unsafe { CACHE.ref_or_init(|| $expr) }
        }
    };
}

define_cached!(home_dir -> String = ::home::home_dir().map(path_to_dir_path_string));
define_cached!(working_dir -> String = ::std::env::current_dir().ok().map(path_to_dir_path_string));
define_cached!(exe_dir -> String = ::std::env::current_exe().ok()
            .as_ref()
            .and_then(|x| x.parent())
            .map(path_to_dir_path_string));

fn path_to_dir_path_string(absolute: impl AsRef<Path>) -> String {
    _path_to_dir_path_string(absolute.as_ref())
}

fn _path_to_dir_path_string(absolute: &Path) -> String {
    assert!(absolute.is_absolute());
    let mut owned = absolute.to_string_lossy().into_owned();
    if !owned.ends_with('/') {
        owned.push('/');
    }
    owned
}
