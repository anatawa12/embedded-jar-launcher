use std::ffi::{c_void, CString};
use std::fmt::Debug;
use std::path::PathBuf;
use jni::*;
use jni::errors::jni_error_code_to_result;

macro_rules! java_vm_unchecked {
    ( $java_vm:expr, $name:tt $(, $args:expr )* ) => ({
        log::trace!("calling unchecked JavaVM method: {}", stringify!($name));
        java_vm_method!($java_vm, $name)($java_vm, $($args),*)
    })
}

macro_rules! java_vm_method {
    ( $jnienv:expr, $name:tt ) => {{
        log::trace!("looking up JavaVM method {}", stringify!($name));
        let env = $jnienv;
        match deref!(deref!(env, "JavaVM"), "*JavaVM").$name {
            Some(meth) => {
                log::trace!("found JavaVM method");
                meth
            }
            None => {
                log::trace!("JavaVM method not defined, returning error");
                return Err(::jni::errors::Error::JavaVMMethodNotFound(stringify!(
                    $name
                )));
            }
        }
    }};
}

macro_rules! deref {
    ( $obj:expr, $ctx:expr ) => {
        if $obj.is_null() {
            return Err(::jni::errors::Error::NullDeref($ctx));
        } else {
            #[allow(unused_unsafe)]
            unsafe {
                *$obj
            }
        }
    };
}

#[allow(non_camel_case_types)]
type JNI_CreateJavaVM_t = extern "system" fn(
    pvm: *mut *mut sys::JavaVM,
    penv: *mut *mut c_void,
    args: *mut c_void,
) -> sys::jint;

#[allow(non_snake_case)]
pub struct JvmLib {
    _lib: libloading::Library,
    JNI_CreateJavaVM: JNI_CreateJavaVM_t,
}

impl JvmLib {
    // expects libjvm.dylib, libjli.so, or jvm.dll
    pub fn load(path: PathBuf) -> Result<Self, libloading::Error> {
        unsafe {
            let lib = libloading::Library::new(path)?;
            let f: libloading::Symbol<JNI_CreateJavaVM_t> = lib.get(b"JNI_CreateJavaVM")?;
            Ok(Self {
                JNI_CreateJavaVM: std::mem::transmute(f.into_raw().into_raw()),
                _lib: lib,
            })
        }
    }
}

pub fn new_java_vm(
    lib: &JvmLib,
    args: InitArgs,
) -> jni::errors::Result<jni::JavaVM> {
    let mut ptr: *mut sys::JavaVM = ::std::ptr::null_mut();
    let mut env: *mut sys::JNIEnv = ::std::ptr::null_mut();

    unsafe {
        jni_error_code_to_result((lib.JNI_CreateJavaVM)(
            &mut ptr as *mut _,
            &mut env as *mut *mut sys::JNIEnv as *mut *mut c_void,
            args.inner_ptr(),
        ))?;

        let vm = jni::JavaVM::from_raw(ptr)?;
        java_vm_unchecked!(vm.get_java_vm_pointer(), DetachCurrentThread);

        Ok(vm)
    }
}

/// JavaVM InitArgs.
///
/// *This API requires "invocation" feature to be enabled,
/// see ["Launching JVM from Rust"](struct.JavaVM.html#launching-jvm-from-rust).*
pub struct InitArgs {
    inner: sys::JavaVMInitArgs,
    opts: Vec<sys::JavaVMOption>,
}

impl InitArgs {
    pub(crate) fn inner_ptr(&self) -> *mut c_void {
        &self.inner as *const _ as _
    }
}

impl Drop for InitArgs {
    fn drop(&mut self) {
        for opt in self.opts.iter() {
            unsafe { CString::from_raw(opt.optionString) };
        }
    }
}

/// Builder for JavaVM InitArgs.
///
/// *This API requires "invocation" feature to be enabled,
/// see ["Launching JVM from Rust"](struct.JavaVM.html#launching-jvm-from-rust).*
#[derive(Debug)]
pub struct InitArgsBuilder {
    opts: Vec<String>,
    ignore_unrecognized: bool,
    version: JNIVersion,
}

impl Default for InitArgsBuilder {
    fn default() -> Self {
        InitArgsBuilder {
            opts: vec![],
            ignore_unrecognized: false,
            version: JNIVersion::V8,
        }
    }
}

impl InitArgsBuilder {
    /// Create a new default InitArgsBuilder
    pub fn new() -> Self {
        Default::default()
    }

    /// Add an option to the init args
    ///
    /// The `vfprintf`, `abort`, and `exit` options are not supported yet.
    pub fn option(self, opt_string: &str) -> Self {
        let mut s = self;

        match opt_string {
            "vfprintf" | "abort" | "exit" => return s,
            _ => {}
        }

        s.opts.push(opt_string.into());

        s
    }

    /// Set JNI version for the init args
    ///
    /// Default: V8
    pub fn version(self, version: JNIVersion) -> Self {
        let mut s = self;
        s.version = version;
        s
    }

    /// Set the `ignoreUnrecognized` init arg flag
    ///
    /// If ignoreUnrecognized is true, JavaVM::new ignores all unrecognized option strings that
    /// begin with "-X" or "_". If ignoreUnrecognized is false, JavaVM::new returns Err as soon as
    /// it encounters any unrecognized option strings.
    ///
    /// Default: `false`
    pub fn ignore_unrecognized(self, ignore: bool) -> Self {
        let mut s = self;
        s.ignore_unrecognized = ignore;
        s
    }

    /// Build the `InitArgs`
    ///
    /// This will check for internal nulls in the option strings and will return
    /// an error if one is found.
    pub fn build(self) -> Result<InitArgs, JvmError> {
        let mut opts = Vec::with_capacity(self.opts.len());
        for opt in self.opts {
            let option_string =
                CString::new(opt.as_str()).map_err(|_| JvmError::NullOptString(opt))?;
            let jvm_opt = sys::JavaVMOption {
                optionString: option_string.into_raw(),
                extraInfo: ::std::ptr::null_mut(),
            };
            opts.push(jvm_opt);
        }

        Ok(InitArgs {
            inner: sys::JavaVMInitArgs {
                version: self.version.into(),
                ignoreUnrecognized: self.ignore_unrecognized as _,
                options: opts.as_ptr() as _,
                nOptions: opts.len() as _,
            },
            opts,
        })
    }

    /// Returns collected options
    pub fn options(&self) -> Vec<String> {
        self.opts.clone()
    }
}

/// Errors that can occur when invoking a [`JavaVM`](super::vm::JavaVM) with the
/// [Invocation API](https://docs.oracle.com/en/java/javase/12/docs/specs/jni/invocation.html).
#[derive(Debug)]
pub enum JvmError {
    /// An internal `0` byte was found when constructing a string.
    NullOptString(String),
}

impl std::error::Error for JvmError {
}
impl std::fmt::Display for JvmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JvmError::NullOptString(e) => write!(f, "internal null in option: {0}", e)
        }
    }
}
