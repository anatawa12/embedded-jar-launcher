use relative_path::{RelativePath, RelativePathBuf};

pub(super) trait ErrorExt {
    type Error;
    fn ok_if(self, filter: impl FnOnce(&Self::Error) -> bool) -> Self;
}

impl<E> ErrorExt for Result<(), E> {
    type Error = E;
    fn ok_if(self, filter: impl FnOnce(&Self::Error) -> bool) -> Self {
        match self {
            Ok(_) => Ok(()),
            Err(e) if filter(&e) => Ok(()),
            Err(e) => Err(e),
        }
    }
}

pub(crate) fn safe_normalize(path: impl AsRef<RelativePath>) -> RelativePathBuf {
    let mut buf = RelativePathBuf::new();
    use relative_path::Component::*;

    for c in path.as_ref().components() {
        match c {
            CurDir => (),
            ParentDir => {
                buf.pop();
            }
            Normal(name) => {
                buf.push(name);
            }
        }
    }

    buf
}
