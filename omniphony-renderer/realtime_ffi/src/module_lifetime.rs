//! Process-lifetime module ownership for detached realtime workers.
//!
//! Stereo Current and authored native-bed teardown intentionally request worker
//! stop without joining. A worker may therefore finish initialization or an
//! in-flight render after its processor object has been destroyed. On Windows,
//! the code and Rust allocator backing that worker must remain mapped until the
//! host process exits.

#[cfg(windows)]
mod windows {
    use core::ffi::c_void;
    use std::ptr;
    use std::sync::OnceLock;

    const GET_MODULE_HANDLE_EX_FLAG_PIN: u32 = 0x0000_0001;
    const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: u32 = 0x0000_0004;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleExW(
            flags: u32,
            module_name_or_address: *const u16,
            module: *mut *mut c_void,
        ) -> i32;
    }

    static PIN_RESULT: OnceLock<bool> = OnceLock::new();

    pub(super) fn pin() -> bool {
        *PIN_RESULT.get_or_init(|| {
            let mut module = ptr::null_mut();
            // FROM_ADDRESS makes this identify the exact loaded cdylib rather
            // than relying on a basename/path lookup. The ABI export is a
            // stable address inside omniphony_realtime.dll.
            let address = crate::omniphony_realtime_abi_major as *const () as *const u16;
            let ok = unsafe {
                GetModuleHandleExW(
                    GET_MODULE_HANDLE_EX_FLAG_PIN | GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
                    address,
                    &mut module,
                )
            };
            ok != 0 && !module.is_null()
        })
    }
}

/// Ensure detached realtime worker code remains resident for the host process.
///
/// Non-Windows hosts do not dynamically unload this cdylib from AudioDG-style
/// graph lifetimes, so the portability build treats the invariant as satisfied.
#[cfg(windows)]
pub(crate) fn pin_for_process_lifetime() -> bool {
    windows::pin()
}

#[cfg(not(windows))]
pub(crate) fn pin_for_process_lifetime() -> bool {
    true
}
