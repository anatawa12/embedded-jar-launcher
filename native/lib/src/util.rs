use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) struct MayUninitOrNone<T> {
    // 0: none
    // 1: uninitialized
    // other: initialized, pointer to T
    ptr: AtomicUsize,
    _ph: PhantomData<T>,
}

#[allow(dead_code)]
impl<T> MayUninitOrNone<T> {
    #[inline]
    pub const fn uninitialized() -> Self {
        Self::check_req();
        Self {
            ptr: AtomicUsize::new(1),
            _ph: PhantomData,
        }
    }

    #[inline]
    pub const fn none() -> Self {
        Self::check_req();
        Self {
            ptr: AtomicUsize::new(0),
            _ph: PhantomData,
        }
    }

    #[inline]
    pub fn initialized(value: T) -> Self {
        Self::check_req();
        let ptr = Box::into_raw(Box::new(value)) as usize;
        Self {
            ptr: AtomicUsize::new(ptr),
            _ph: PhantomData,
        }
    }

    const fn check_req() {
        #[cfg(debug_assertions)]
        if std::mem::align_of::<T>() % 2 != 0 {
            ["align assert failed: requires 2 bytes"][std::mem::align_of::<T>()];
        }
    }
}

impl<T> MayUninitOrNone<T> {
    pub(crate) fn ref_or_init<'a>(&'a mut self, init: impl FnOnce() -> Option<T>) -> Option<&'a T> {
        match self.ptr.load(Ordering::Acquire) {
            0 => None,
            1 => {
                if let Some(value) = init() {
                    // initialize
                    let new_ptr = Box::into_raw(Box::new(value)) as usize;
                    match self
                        .ptr
                        .compare_exchange(1, new_ptr, Ordering::AcqRel, Ordering::Acquire)
                    {
                        Ok(_) => unsafe { Some(&*(new_ptr as *const T)) },
                        Err(ptr) => unsafe {
                            // drop the value
                            Box::from_raw(new_ptr as *mut T);
                            Self::ptr_to_value::<'a>(ptr)
                        },
                    }
                } else {
                    match self
                        .ptr
                        .compare_exchange(1, 0, Ordering::AcqRel, Ordering::Acquire)
                    {
                        Ok(_) => None,
                        Err(ptr) => Self::ptr_to_value::<'a>(ptr),
                    }
                }
            }
            ptr => unsafe { Some(&*(ptr as *const T)) },
        }
    }

    fn ptr_to_value<'a>(ptr: usize) -> Option<&'a T> {
        if ptr == 0 {
            None
        } else {
            unsafe { Some(&*(ptr as *const T)) }
        }
    }
}

impl<T> Drop for MayUninitOrNone<T> {
    fn drop(&mut self) {
        match self.ptr.load(Ordering::Acquire) {
            0 => {}
            1 => {}
            ptr => unsafe {
                Box::from_raw(ptr as *mut T);
            },
        }
    }
}

extern "Rust" {
    fn _launcher_lib_print(args: core::fmt::Arguments<'_>);
}

pub fn launcher_lib_print(args: core::fmt::Arguments<'_>) {
    unsafe { _launcher_lib_print(args) }
}
