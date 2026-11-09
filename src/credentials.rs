//! Secrets are stored only in Windows Credential Manager, never in design/session files.
use crate::model::Source;

pub fn ensure_id(source: &mut Source) {
    if source.credential_id.is_empty() {
        source.credential_id = format!(
            "{:x}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            source.id
        );
    }
}

#[cfg(windows)]
mod vault {
    use super::*;
    use std::{ffi::c_void, ptr};
    #[repr(C)]
    struct Credential {
        flags: u32,
        kind: u32,
        target: *mut u16,
        comment: *mut u16,
        written: [u32; 2],
        size: u32,
        blob: *mut u8,
        persist: u32,
        count: u32,
        attributes: *mut c_void,
        alias: *mut u16,
        username: *mut u16,
    }
    #[link(name = "Advapi32")]
    extern "system" {
        fn CredWriteW(credential: *const Credential, flags: u32) -> i32;
        fn CredReadW(
            target: *const u16,
            kind: u32,
            flags: u32,
            credential: *mut *mut Credential,
        ) -> i32;
        fn CredDeleteW(target: *const u16, kind: u32, flags: u32) -> i32;
        fn CredFree(buffer: *mut c_void);
    }
    fn target(s: &Source) -> Vec<u16> {
        // Keep existing JSON/HA credentials, but never reuse a Finnhub key for Alpha Vantage.
        format!(
            "uCockpitStudio/{}{}",
            s.credential_id,
            if s.kind == crate::model::SourceKind::Stock {
                "/AlphaVantage"
            } else {
                ""
            }
        )
        .encode_utf16()
        .chain(Some(0))
        .collect()
    }
    fn error() -> String {
        format!(
            "Windows Credential Manager: {}",
            std::io::Error::last_os_error()
        )
    }
    pub fn save(s: &Source) -> Result<(), String> {
        if s.credential_id.is_empty() {
            return Err("Missing credential reference".into());
        }
        if s.token.is_empty() && s.password.is_empty() && s.username.is_empty() {
            return delete(s);
        }
        let mut bytes =
            serde_json::to_vec(&(&s.token, &s.username, &s.password)).map_err(|e| e.to_string())?;
        if bytes.len() > 2560 {
            return Err("Credentials exceed the Windows vault limit (2560 bytes)".into());
        }
        let mut name = target(s);
        let c = Credential {
            flags: 0,
            kind: 1,
            target: name.as_mut_ptr(),
            comment: ptr::null_mut(),
            written: [0; 2],
            size: bytes.len() as u32,
            blob: bytes.as_mut_ptr(),
            persist: 2,
            count: 0,
            attributes: ptr::null_mut(),
            alias: ptr::null_mut(),
            username: ptr::null_mut(),
        };
        let result = unsafe { CredWriteW(&c, 0) };
        let err = if result == 0 { Some(error()) } else { None };
        bytes.fill(0);
        err.map_or(Ok(()), Err)
    }
    pub fn restore(s: &mut Source) -> Result<(), String> {
        if s.credential_id.is_empty() {
            return Ok(());
        }
        let mut c = ptr::null_mut();
        if unsafe { CredReadW(target(s).as_ptr(), 1, 0, &mut c) } == 0 {
            return if std::io::Error::last_os_error().raw_os_error() == Some(1168) {
                Ok(())
            } else {
                Err(error())
            };
        }
        let data = unsafe { std::slice::from_raw_parts((*c).blob, (*c).size as usize) };
        let result = serde_json::from_slice::<(String, String, String)>(data)
            .map_err(|_| "Stored credentials could not be read".to_owned());
        unsafe { CredFree(c.cast()) };
        let (token, username, password) = result?;
        s.token = token;
        s.username = username;
        s.password = password;
        Ok(())
    }
    pub fn delete(s: &Source) -> Result<(), String> {
        if s.credential_id.is_empty() {
            return Ok(());
        }
        if unsafe { CredDeleteW(target(s).as_ptr(), 1, 0) } != 0
            || std::io::Error::last_os_error().raw_os_error() == Some(1168)
        {
            Ok(())
        } else {
            Err(error())
        }
    }
}
#[cfg(windows)]
pub use vault::{delete, restore, save};
#[cfg(not(windows))]
pub fn save(_: &Source) -> Result<(), String> {
    Err("Secure credential storage requires Windows".into())
}
#[cfg(not(windows))]
pub fn restore(_: &mut Source) -> Result<(), String> {
    Ok(())
}
#[cfg(not(windows))]
pub fn delete(_: &Source) -> Result<(), String> {
    Ok(())
}
