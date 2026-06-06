use crate::confine::ScreenRect;
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
};

const MONITORINFOF_PRIMARY: u32 = 0x0000_0001;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitorInfo {
    pub bounds: ScreenRect,
    pub is_primary: bool,
}

pub fn enumerate() -> Vec<MonitorInfo> {
    let mut monitors: Vec<MonitorInfo> = Vec::new();
    unsafe {
        let lparam = LPARAM(&mut monitors as *mut Vec<MonitorInfo> as isize);
        let _ = EnumDisplayMonitors(None, None, Some(enum_proc), lparam);
    }
    monitors
}

unsafe extern "system" fn enum_proc(
    hmon: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    lparam: LPARAM,
) -> BOOL {
    let monitors = unsafe { &mut *(lparam.0 as *mut Vec<MonitorInfo>) };
    let mut info = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    let ok = unsafe { GetMonitorInfoW(hmon, &mut info as *mut _ as *mut MONITORINFO).as_bool() };
    if !ok {
        return BOOL(1);
    }
    let bounds = info.monitorInfo.rcMonitor;
    monitors.push(MonitorInfo {
        bounds: ScreenRect::new(bounds.left, bounds.top, bounds.right, bounds.bottom),
        is_primary: (info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY) != 0,
    });
    BOOL(1)
}
