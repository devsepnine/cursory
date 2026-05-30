//! Single-instance enforcement.
//!
//! A named mutex guarantees only one Cursory process runs per user session. A
//! second launch finds the mutex already owned, signals the first instance via
//! a named auto-reset event (so it can pop out of the tray instead of leaving
//! the user wondering why "nothing happened"), and then exits.
//!
//! The first instance keeps the mutex handle alive for its whole lifetime and
//! polls the event from its UI tick via [`poll_show_request`].

use std::ffi::c_void;
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicIsize, Ordering};

use windows::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, WAIT_OBJECT_0,
};
use windows::Win32::System::Threading::{
    CreateEventW, CreateMutexW, EVENT_MODIFY_STATE, OpenEventW, SetEvent, WaitForSingleObject,
};
use windows::core::PCWSTR;

const MUTEX_NAME: &str = r"Local\Cursory-SingleInstance-Mutex";
const EVENT_NAME: &str = r"Local\Cursory-SingleInstance-Show";

/// Handle to the show-event, stored as a raw value so the UI layer can poll it
/// from the main thread without threading the guard through `App`.
static SHOW_EVENT: AtomicIsize = AtomicIsize::new(0);

/// Held by the first instance for the process lifetime; releasing it (on exit)
/// frees the single-instance lock.
pub struct SingleInstance {
    mutex: HANDLE,
    event: HANDLE,
}

impl SingleInstance {
    /// Returns `Some` if this is the first instance, `None` if another is
    /// already running.
    pub fn acquire() -> Option<Self> {
        let name = wide(MUTEX_NAME);
        let mutex = unsafe { CreateMutexW(None, true, PCWSTR(name.as_ptr())) };
        // Capture immediately: ERROR_ALREADY_EXISTS is only meaningful right
        // after the call, and CreateMutexW succeeds even when the mutex exists.
        let already_exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        let mutex = mutex.ok()?;

        if already_exists {
            unsafe {
                let _ = CloseHandle(mutex);
            }
            return None;
        }

        let event_name = wide(EVENT_NAME);
        let event = unsafe { CreateEventW(None, false, false, PCWSTR(event_name.as_ptr())) };
        let event = match event {
            Ok(h) => h,
            Err(_) => {
                // No event => degrade gracefully to "single instance, no signal".
                HANDLE(std::ptr::null_mut())
            }
        };
        if !event.is_invalid() {
            SHOW_EVENT.store(event.0 as isize, Ordering::Relaxed);
        }

        Some(Self { mutex, event })
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        unsafe {
            if !self.event.is_invalid() {
                let _ = CloseHandle(self.event);
            }
            let _ = CloseHandle(self.mutex);
        }
    }
}

/// Called from the running instance's poll loop. Returns `true` once when a
/// second launch has asked this instance to surface its window.
pub fn poll_show_request() -> bool {
    let raw = SHOW_EVENT.load(Ordering::Relaxed);
    if raw == 0 {
        return false;
    }
    let handle = HANDLE(raw as *mut c_void);
    unsafe { WaitForSingleObject(handle, 0) == WAIT_OBJECT_0 }
}

/// Called by a second instance to wake the first one, then exit.
pub fn signal_existing() {
    let event_name = wide(EVENT_NAME);
    unsafe {
        if let Ok(h) = OpenEventW(EVENT_MODIFY_STATE, false, PCWSTR(event_name.as_ptr())) {
            let _ = SetEvent(h);
            let _ = CloseHandle(h);
        }
    }
}

fn wide(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
