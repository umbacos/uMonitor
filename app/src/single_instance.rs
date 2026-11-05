//! Keep one interactive editor per Windows session, including while in the tray.
use std::io;
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, GetLastError, ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, ERROR_FILE_NOT_FOUND,
        HANDLE,
    },
    System::Threading::{CreateMutexExW, OpenMutexW, SYNCHRONIZATION_SYNCHRONIZE},
};

// Stable across executable names, locations and versions. Helpers and isolated
// screenshots do not open the interactive editor or share its saved session.
const NAME: &str = "Local\\uMonitor.SingleInstance.{6FBB87A3-2DA7-456E-9334-DCD8B238AE6D}";

pub struct SingleInstance(HANDLE);

impl Drop for SingleInstance {
    fn drop(&mut self) {
        // SAFETY: this guard owns exactly one valid, non-inheritable handle.
        unsafe { CloseHandle(self.0) };
    }
}

fn wide(name: &str) -> Vec<u16> {
    name.encode_utf16().chain([0]).collect()
}

fn is_running(name: &str) -> io::Result<bool> {
    let name = wide(name);
    // SAFETY: name is NUL-terminated and lives through the call. Only request
    // synchronization access so an unelevated launcher can see an elevated app.
    let handle = unsafe { OpenMutexW(SYNCHRONIZATION_SYNCHRONIZE, 0, name.as_ptr()) };
    if !handle.is_null() {
        drop(SingleInstance(handle));
        return Ok(true);
    }
    let error = io::Error::last_os_error();
    match error.raw_os_error().map(|code| code as u32) {
        Some(ERROR_FILE_NOT_FOUND) => Ok(false),
        // An existing instance with a more restrictive token still blocks launch.
        Some(ERROR_ACCESS_DENIED) => Ok(true),
        _ => Err(error),
    }
}

fn try_acquire(name: &str) -> io::Result<Option<SingleInstance>> {
    let name = wide(name);
    // SAFETY: null attributes create a non-inheritable handle. Creation and the
    // already-exists check are atomic, even when two launches race after UAC.
    // We use the object's lifetime, never take ownership or wait on the mutex.
    let (handle, error) = unsafe {
        let handle = CreateMutexExW(
            std::ptr::null(),
            name.as_ptr(),
            0,
            SYNCHRONIZATION_SYNCHRONIZE,
        );
        (handle, GetLastError())
    };
    if handle.is_null() {
        return if error == ERROR_ACCESS_DENIED {
            Ok(None)
        } else {
            Err(io::Error::from_raw_os_error(error as i32))
        };
    }
    let guard = SingleInstance(handle);
    if error == ERROR_ALREADY_EXISTS {
        return Ok(None);
    }
    Ok(Some(guard))
}

fn notify_already_running() {
    rfd::MessageDialog::new()
        .set_title("uMonitor")
        .set_description("uMonitor is already running. Check its window or the system tray.")
        .set_level(rfd::MessageLevel::Info)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

/// Reject duplicates before UAC, then acquire atomically in the elevated process.
/// Hold the returned guard until the editor and its background workers exit.
pub fn start() -> io::Result<Option<SingleInstance>> {
    if is_running(NAME)? {
        notify_already_running();
        return Ok(None);
    }
    // Holding a guard during elevation would block our own elevated replacement.
    if !crate::elevation::ensure_administrator()? {
        return Ok(None);
    }
    let instance = try_acquire(NAME)?;
    if instance.is_none() {
        notify_already_running();
    }
    Ok(instance)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Barrier,
    };

    fn test_name(suffix: &str) -> String {
        format!("{NAME}.test.{}.{suffix}", std::process::id())
    }

    #[test]
    fn duplicate_launch_is_blocked_until_the_last_handle_closes() {
        let name = test_name("lifetime");
        assert!(!is_running(&name).unwrap());
        let first = try_acquire(&name).unwrap().unwrap();
        assert!(is_running(&name).unwrap());
        assert!(try_acquire(&name).unwrap().is_none());
        // Rejected launches must close their extra handles, too.
        drop(first);
        assert!(!is_running(&name).unwrap());
        assert!(try_acquire(&name).unwrap().is_some());
    }

    #[test]
    fn simultaneous_launches_allow_exactly_one_instance() {
        let name = test_name("race");
        let barrier = Arc::new(Barrier::new(8));
        let winners = Arc::new(AtomicUsize::new(0));
        std::thread::scope(|scope| {
            for _ in 0..8 {
                let (name, barrier, winners) = (&name, &barrier, &winners);
                scope.spawn(move || {
                    barrier.wait();
                    let guard = try_acquire(name);
                    if matches!(guard, Ok(Some(_))) {
                        winners.fetch_add(1, Ordering::SeqCst);
                    }
                    // Keep the winner alive until every contender has tried.
                    barrier.wait();
                    drop(guard.unwrap());
                });
            }
        });
        assert_eq!(winners.load(Ordering::SeqCst), 1);
        assert!(!is_running(&name).unwrap());
    }

    #[test]
    fn subprocess_instance_probe() {
        let Ok(name) = std::env::var("UMONITOR_INSTANCE_TEST_NAME") else {
            return;
        };
        let mode = std::env::var("UMONITOR_INSTANCE_TEST_MODE").unwrap();
        let guard = try_acquire(&name).unwrap();
        assert_eq!(guard.is_none(), mode == "duplicate");
        if mode == "exit_without_drop" {
            // Skip Rust destructors to verify Windows releases the handle itself.
            std::process::exit(0);
        }
    }

    #[test]
    fn instance_lock_is_shared_between_processes_and_clears_on_process_exit() {
        use std::{os::windows::process::CommandExt, process::Command};
        let name = test_name("processes");
        let probe = |mode: &str| {
            let output = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "single_instance::tests::subprocess_instance_probe",
                ])
                .env("UMONITOR_INSTANCE_TEST_NAME", &name)
                .env("UMONITOR_INSTANCE_TEST_MODE", mode)
                .creation_flags(windows_sys::Win32::System::Threading::CREATE_NO_WINDOW)
                .output()
                .unwrap();
            assert!(output.status.success(), "{output:?}");
        };
        let guard = try_acquire(&name).unwrap().unwrap();
        probe("duplicate");
        assert!(is_running(&name).unwrap());
        drop(guard);
        probe("first");
        assert!(!is_running(&name).unwrap());
        probe("exit_without_drop");
        assert!(!is_running(&name).unwrap());
        assert!(try_acquire(&name).unwrap().is_some());
    }
}
