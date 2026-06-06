//! Open a URL in the user's default browser via the shell.

use std::os::windows::ffi::OsStrExt;

use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{PCWSTR, w};

/// Launch `url` in the default browser. Best-effort: failures are ignored since
/// there is no useful recovery for "couldn't open a link".
pub fn open(url: &str) {
    let wide: Vec<u16> = std::ffi::OsStr::new(url)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}
