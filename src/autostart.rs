//! Manage the "launch on Windows startup" behavior via the per-user Run key:
//! `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
//!
//! Per-user (HKCU) requires no administrator rights, and works the same whether
//! Cursory was installed via the MSI or is run as a portable exe. The registry
//! value is the single source of truth — the UI reads it back on launch rather
//! than caching the state in the settings file.

use std::os::windows::ffi::OsStrExt;

use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_SZ, RegCloseKey, RegDeleteValueW,
    RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::core::PCWSTR;

const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "Cursory";

/// Returns `true` if Cursory is registered to launch at startup.
pub fn is_enabled() -> bool {
    let subkey = wide(RUN_SUBKEY);
    let name = wide(VALUE_NAME);
    unsafe {
        let mut hkey = HKEY::default();
        let opened = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        );
        if opened != ERROR_SUCCESS {
            return false;
        }
        let result = RegQueryValueExW(
            hkey,
            PCWSTR(name.as_ptr()),
            None,
            None,
            None,
            None,
        );
        let _ = RegCloseKey(hkey);
        result == ERROR_SUCCESS
    }
}

/// Enables or disables launch-at-startup. Returns the resulting state on
/// success, or a human-readable error.
pub fn set_enabled(enable: bool) -> Result<bool, String> {
    if enable {
        let exe = std::env::current_exe().map_err(|e| format!("cannot resolve exe path: {e}"))?;
        let command = format!("\"{}\"", exe.display());
        write_value(&command).map(|()| true)
    } else {
        delete_value().map(|()| false)
    }
}

fn write_value(command: &str) -> Result<(), String> {
    let subkey = wide(RUN_SUBKEY);
    let name = wide(VALUE_NAME);
    let data = wide(command);
    let bytes = as_byte_slice(&data);
    unsafe {
        let mut hkey = HKEY::default();
        let opened = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        );
        if opened != ERROR_SUCCESS {
            return Err(format!("cannot open Run key (code {})", opened.0));
        }
        let set = RegSetValueExW(hkey, PCWSTR(name.as_ptr()), 0, REG_SZ, Some(bytes));
        let _ = RegCloseKey(hkey);
        if set != ERROR_SUCCESS {
            return Err(format!("cannot write Run value (code {})", set.0));
        }
    }
    Ok(())
}

fn delete_value() -> Result<(), String> {
    let subkey = wide(RUN_SUBKEY);
    let name = wide(VALUE_NAME);
    unsafe {
        let mut hkey = HKEY::default();
        let opened = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        );
        if opened == ERROR_FILE_NOT_FOUND {
            // Run key itself is absent => nothing to remove.
            return Ok(());
        }
        if opened != ERROR_SUCCESS {
            return Err(format!("cannot open Run key (code {})", opened.0));
        }
        let deleted = RegDeleteValueW(hkey, PCWSTR(name.as_ptr()));
        let _ = RegCloseKey(hkey);
        // ERROR_FILE_NOT_FOUND means the value was already absent — treat as success.
        if deleted != ERROR_SUCCESS && deleted != ERROR_FILE_NOT_FOUND {
            return Err(format!("cannot delete Run value (code {})", deleted.0));
        }
    }
    Ok(())
}

fn wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn as_byte_slice(data: &[u16]) -> &[u8] {
    // REG_SZ data is the UTF-16 string including its terminating null, as bytes.
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, std::mem::size_of_val(data)) }
}
