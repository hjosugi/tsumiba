//! Windows release builds are GUI-subsystem programs (no console window flashes
//! up when tsumiba starts at login), which means a terminal does not give them
//! its console. When tsumiba is run with arguments from a terminal, this
//! attaches to that terminal so `tsumiba --help` and `tsumiba list` can print.

#[cfg(windows)]
pub fn attach_parent() {
    use windows_sys::Win32::Foundation::{
        GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Console::{
        ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE,
        STD_OUTPUT_HANDLE, SetStdHandle,
    };

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    // SAFETY: plain Win32 calls with valid arguments; the handles created here
    // are intentionally kept open for the life of the process.
    unsafe {
        if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
            // No parent console (launched from Explorer or at login): nothing to do.
            return;
        }
        for (std_handle, device) in [
            (STD_INPUT_HANDLE, "CONIN$"),
            (STD_OUTPUT_HANDLE, "CONOUT$"),
            (STD_ERROR_HANDLE, "CONOUT$"),
        ] {
            let current: HANDLE = GetStdHandle(std_handle);
            // Keep a handle that was redirected to a file or pipe.
            if !current.is_null() && current != INVALID_HANDLE_VALUE {
                continue;
            }
            let name = wide(device);
            let handle = CreateFileW(
                name.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            );
            if handle != INVALID_HANDLE_VALUE {
                SetStdHandle(std_handle, handle);
            }
        }
    }
}

#[cfg(not(windows))]
pub fn attach_parent() {}
