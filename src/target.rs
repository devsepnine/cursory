use windows::Win32::Foundation::{BOOL, HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
};

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub hwnd: isize,
    pub title: String,
}

pub fn enumerate() -> Vec<WindowInfo> {
    let mut wins: Vec<WindowInfo> = Vec::new();
    unsafe {
        let lparam = LPARAM(&mut wins as *mut Vec<WindowInfo> as isize);
        let _ = EnumWindows(Some(enum_proc), lparam);
    }
    wins.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    wins
}

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let wins = unsafe { &mut *(lparam.0 as *mut Vec<WindowInfo>) };
    if unsafe { !IsWindowVisible(hwnd).as_bool() } {
        return BOOL(1);
    }
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return BOOL(1);
    }
    let mut buf = vec![0u16; (len + 1) as usize];
    let actual = unsafe { GetWindowTextW(hwnd, &mut buf) };
    if actual <= 0 {
        return BOOL(1);
    }
    let title = String::from_utf16_lossy(&buf[..actual as usize]);
    if title.trim().is_empty() {
        return BOOL(1);
    }
    wins.push(WindowInfo {
        hwnd: hwnd.0 as isize,
        title,
    });
    BOOL(1)
}
