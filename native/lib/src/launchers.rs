use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::{Command, exit};
use cfg_if::cfg_if;
use jni::JNIEnv;
use jni::objects::{JObject, JThrowable, JValue};
use jni::signature::{JavaType, Primitive};
use crate::jni::{InitArgsBuilder, JvmLib, new_java_vm};
use jni::errors::{Result as JniResult, Error as JniError};
use jni::sys::jsize;

// TODO: change the java_home: PathBuf parameter to entrypoint and get executable/library path from searcher

pub trait JvmParamStr : AsRef<str> + AsRef<OsStr> {}

impl <T> JvmParamStr for T where T : AsRef<str> + AsRef<OsStr> {}

#[cfg(unix)]
pub fn launch_via_exec(
    mut java_home: PathBuf,
    vm_args: impl IntoIterator<Item = impl JvmParamStr>,
    main_class: impl JvmParamStr,
    args: impl IntoIterator<Item = impl JvmParamStr>,
) -> ! {
    use std::os::unix::process::CommandExt;
    java_home.push("bin/java");
    let err = Command::new(java_home)
        .args(vm_args)
        .arg(main_class)
        .args(args)
        .exec();
    panic!("launching java failed: {}", err);
}

pub fn launch_via_spawn(
    mut java_home: PathBuf,
    vm_args: impl IntoIterator<Item = impl JvmParamStr>,
    main_class: impl JvmParamStr,
    args: impl IntoIterator<Item = impl JvmParamStr>,
) -> ! {
    java_home.push("bin/java");
    let output = Command::new(java_home)
        .args(vm_args)
        .arg(main_class)
        .args(args)
        .spawn()
        .expect("launching java failed")
        .wait()
        .expect("waiting for java failed");
    cfg_if! {
        if #[cfg(unix)] {
            use std::os::unix::process::ExitStatusExt;
            exit(output.into_raw());
        } else if #[cfg(windows)] {
            // on windows, (currently) code will never be fail
            exit(output.code().unwrap())
        } else {
            compile_error!("non unix/windows")
        }
    }
}

pub fn launch_via_jni(
    mut java_home: PathBuf,
    vm_args: impl IntoIterator<Item = impl JvmParamStr>,
    main_class: impl JvmParamStr,
    args: impl IntoIterator<Item = impl JvmParamStr>,
) -> ! {
    cfg_if! {
        if #[cfg(target_os = "macos")] {
            java_home.push("lib/libjli.dylib");
        } else if #[cfg(target_os = "linux")] {
            java_home.push("lib/server/libjvm.so");
        } else if #[cfg(target_os = "windows")] {
            java_home.push("bin/server/jvm.dll");
        } else {
            compile_error!("unsupported platform")
        }
    }
    let library = JvmLib::load(java_home)
        .expect("loading jvm dynamic library");
    let jvm_args = {
        let mut builder = InitArgsBuilder::new();

        for vm_arg in vm_args {
            builder = builder.option(vm_arg.as_ref());
        }

        builder.build()
    }.unwrap();

    let jvm = new_java_vm(&library, jvm_args).expect("launching jvm");
    let env = jvm.attach_current_thread().expect("launching jvm");


    fn run_main(
        env: &JNIEnv,
        main_class: impl JvmParamStr,
        args: impl IntoIterator<Item = impl JvmParamStr>,
    ) -> JniResult<()> {
        let string_class = env.find_class("java/lang/String")?;
        let main_class = env.find_class(AsRef::<str>::as_ref(&main_class))?;
        let main_method = env.get_static_method_id(main_class, "main", "([Ljava/lang/String;)V")?;
        let strings = args.into_iter().map(|arg| env.new_string(AsRef::<str>::as_ref(&arg))).collect::<Result<Vec<_>, _>>()?;
        let args = env.new_object_array(strings.len() as jsize, string_class, JObject::null())?;

        for (i, s) in strings.into_iter().enumerate() {
            env.set_object_array_element(args, i as jsize, s)?;
        }

        env.call_static_method_unchecked(
            main_class,
            main_method,
            JavaType::Primitive(Primitive::Void),
            &[JValue::Object(args.into())]
        )?;

        Ok(())
    }

    match run_main(&env, main_class, args) {
        Ok(_) => exit(0),
        Err(JniError::JavaException) => {
            let throwable = env.exception_occurred().expect("getting occurred exception");
            if throwable.is_null() {
                panic!("no exception occurred but jni reports there's exception.")
            }
            env.exception_clear().ok();
            fn call_print_stack_trace(env: &JNIEnv, exception: JThrowable) -> JniResult<()> {
                env.call_method(exception, "printStackTrace", "()V", &[])
                    .map(|_| ())
            }
            call_print_stack_trace(&env, throwable).expect("printing stack trace");
            exit(1);
        }
        Err(e) => panic!("internal error occurred: {}", e)
    }
}
