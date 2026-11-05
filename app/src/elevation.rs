//! Elevate the interactive editor before it opens files or starts sensor workers.
use std::{ffi::OsString, io, os::windows::ffi::OsStrExt};
use windows_sys::Win32::{
    Foundation::{CloseHandle, ERROR_CANCELLED},
    Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
    System::Threading::{GetCurrentProcess, OpenProcessToken},
    UI::{
        Shell::{ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW},
        WindowsAndMessaging::SW_SHOWNORMAL,
    },
};

fn is_elevated() -> io::Result<bool> {
    let mut token = std::ptr::null_mut();
    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut size = 0;
    // SAFETY: all output buffers are valid for their declared sizes. The token
    // is owned here and closed after querying, including on query failure.
    unsafe {
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(io::Error::last_os_error());
        }
        let success = GetTokenInformation(
            token,
            TokenElevation,
            (&mut elevation as *mut TOKEN_ELEVATION).cast(),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        );
        let error = (success == 0).then(io::Error::last_os_error);
        CloseHandle(token);
        if let Some(error) = error {
            return Err(error);
        }
    }
    Ok(elevation.TokenIsElevated != 0)
}

// ShellExecuteEx takes one Windows command line, rather than an argument array.
// Quote every argument and double backslashes before quotes and the closing quote.
fn parameters(args: impl Iterator<Item = OsString>) -> Vec<u16> {
    let mut command = Vec::new();
    for arg in args {
        if !command.is_empty() {
            command.push(b' ' as u16);
        }
        command.push(b'"' as u16);
        let mut backslashes = 0;
        for ch in arg.encode_wide() {
            if ch == b'\\' as u16 {
                backslashes += 1;
                continue;
            }
            let count = if ch == b'"' as u16 {
                backslashes * 2 + 1
            } else {
                backslashes
            };
            command.extend(std::iter::repeat_n(b'\\' as u16, count));
            command.push(ch);
            backslashes = 0;
        }
        command.extend(std::iter::repeat_n(b'\\' as u16, backslashes * 2));
        command.push(b'"' as u16);
    }
    command.push(0);
    command
}

/// Continue only in the elevated process. A successful handoff or canceled UAC
/// prompt ends the unelevated launcher without opening another editor window.
pub fn ensure_administrator() -> io::Result<bool> {
    if is_elevated()? {
        return Ok(true);
    }
    let executable: Vec<u16> = std::env::current_exe()?
        .as_os_str()
        .encode_wide()
        .chain([0])
        .collect();
    let directory: Vec<u16> = std::env::current_dir()?
        .as_os_str()
        .encode_wide()
        .chain([0])
        .collect();
    let args = parameters(std::env::args_os().skip(1));
    let verb: Vec<u16> = "runas\0".encode_utf16().collect();
    // SAFETY: zero is valid for unused SHELLEXECUTEINFO fields; the referenced
    // NUL-terminated UTF-16 buffers remain alive until the synchronous call ends.
    unsafe {
        let initialized = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED
                | windows::Win32::System::Com::COINIT_DISABLE_OLE1DDE,
        )
        .is_ok();
        let mut info: SHELLEXECUTEINFOW = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        // The launcher exits immediately after this call, so wait for handoff.
        info.fMask = SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI;
        info.lpVerb = verb.as_ptr();
        info.lpFile = executable.as_ptr();
        info.lpParameters = args.as_ptr();
        info.lpDirectory = directory.as_ptr();
        info.nShow = SW_SHOWNORMAL;
        let success = ShellExecuteExW(&mut info);
        let error = (success == 0).then(io::Error::last_os_error);
        if initialized {
            windows::Win32::System::Com::CoUninitialize();
        }
        if let Some(error) = error {
            if error.raw_os_error() != Some(ERROR_CANCELLED as i32) {
                return Err(error);
            }
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elevation_arguments_round_trip_through_windows_parser() {
        use std::os::windows::ffi::OsStringExt;
        use windows_sys::Win32::{Foundation::LocalFree, UI::Shell::CommandLineToArgvW};

        // Exercise the Windows startup flag, spaces, Unicode, empty arguments,
        // quotes and trailing backslashes so a relaunch cannot alter arguments.
        let expected = [
            OsString::from("--tray"),
            OsString::from(r"C:\folder with spaces\design.json"),
            OsString::from("温度"),
            OsString::from(""),
            OsString::from("a\\\"b"),
            OsString::from("C:\\trailing space\\"),
        ];
        let mut command: Vec<u16> = "uMonitor.exe ".encode_utf16().collect();
        command.extend(parameters(expected.iter().cloned()));
        let mut count = 0;
        // SAFETY: CommandLineToArgvW returns count valid NUL-terminated strings
        // in one allocation, which is released after copying their contents.
        unsafe {
            let argv = CommandLineToArgvW(command.as_ptr(), &mut count);
            assert!(!argv.is_null());
            let actual: Vec<_> = std::slice::from_raw_parts(argv, count as usize)
                .iter()
                .skip(1)
                .map(|&arg| {
                    let mut length = 0;
                    while *arg.add(length) != 0 {
                        length += 1;
                    }
                    OsString::from_wide(std::slice::from_raw_parts(arg, length))
                })
                .collect();
            LocalFree(argv.cast());
            assert_eq!(actual, expected);
        }
        assert_eq!(parameters(std::iter::empty()), [0]);
    }
}
