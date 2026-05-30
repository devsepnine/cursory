use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use thiserror::Error;
use windows::Win32::Foundation::{GetLastError, HWND, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};
use windows::Win32::UI::WindowsAndMessaging::{
    ClipCursor, GetWindowRect, IsIconic, IsWindow, IsWindowVisible,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl ScreenRect {
    pub fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn from_xywh(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        }
    }

    pub fn width(self) -> i32 {
        self.right - self.left
    }
    pub fn height(self) -> i32 {
        self.bottom - self.top
    }

    pub fn is_valid(self) -> bool {
        self.right > self.left && self.bottom > self.top
    }

    pub fn inset(self, px: i32) -> Self {
        Self {
            left: self.left + px,
            top: self.top + px,
            right: self.right - px,
            bottom: self.bottom - px,
        }
    }

    fn to_win32(self) -> RECT {
        RECT {
            left: self.left,
            top: self.top,
            right: self.right,
            bottom: self.bottom,
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfineError {
    #[error("invalid rect: {0:?}")]
    InvalidRect(ScreenRect),
    #[error("ClipCursor failed with Win32 error code {0}")]
    Win32(u32),
    #[error("window handle not available")]
    NoHandle,
}

#[derive(Debug, Clone, Copy)]
pub enum CageMode {
    Window { hwnd: isize },
    Fixed(ScreenRect),
}

#[derive(Clone)]
struct ClipState {
    active: bool,
    mode: Option<CageMode>,
    padding: i32,
    auto_release_reason: Option<String>,
}

pub struct ClipController {
    state: Arc<Mutex<ClipState>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl ClipController {
    pub fn new() -> Self {
        unsafe {
            let _ = timeBeginPeriod(1);
        }
        let state = Arc::new(Mutex::new(ClipState {
            active: false,
            mode: None,
            padding: 0,
            auto_release_reason: None,
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let handle = {
            let state = Arc::clone(&state);
            let stop = Arc::clone(&stop);
            thread::spawn(move || run_loop(state, stop))
        };
        Self {
            state,
            stop,
            handle: Some(handle),
        }
    }

    pub fn activate(&self, mode: CageMode, padding: i32) -> Result<(), ConfineError> {
        let rect = resolve_rect(mode, padding)?;
        {
            let mut s = self.state.lock().unwrap();
            s.active = true;
            s.mode = Some(mode);
            s.padding = padding;
            s.auto_release_reason = None;
        }
        apply(rect)
    }

    pub fn deactivate(&self) {
        {
            let mut s = self.state.lock().unwrap();
            s.active = false;
            s.auto_release_reason = None;
        }
        let _ = release();
    }

    pub fn take_auto_release(&self) -> Option<String> {
        self.state.lock().unwrap().auto_release_reason.take()
    }

    pub fn set_mode(&self, mode: CageMode) -> Result<(), ConfineError> {
        let snapshot = self.state.lock().unwrap().clone();
        if snapshot.active {
            let rect = resolve_rect(mode, snapshot.padding)?;
            self.state.lock().unwrap().mode = Some(mode);
            apply(rect)
        } else {
            self.state.lock().unwrap().mode = Some(mode);
            Ok(())
        }
    }

    pub fn set_padding(&self, padding: i32) -> Result<(), ConfineError> {
        let snapshot = self.state.lock().unwrap().clone();
        if snapshot.active {
            if let Some(mode) = snapshot.mode {
                let rect = resolve_rect(mode, padding)?;
                self.state.lock().unwrap().padding = padding;
                return apply(rect);
            }
        }
        self.state.lock().unwrap().padding = padding;
        Ok(())
    }
}

impl Default for ClipController {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ClipController {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        unsafe {
            let _ = timeEndPeriod(1);
        }
    }
}

fn run_loop(state: Arc<Mutex<ClipState>>, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Relaxed) {
        let snapshot = state.lock().unwrap().clone();
        if snapshot.active {
            if let Some(mode) = snapshot.mode {
                match resolve_rect(mode, snapshot.padding) {
                    Ok(rect) => {
                        let _ = apply(rect);
                    }
                    Err(ConfineError::NoHandle) => {
                        let mut s = state.lock().unwrap();
                        s.active = false;
                        s.auto_release_reason = Some("target window closed".into());
                        drop(s);
                        let _ = release();
                    }
                    Err(_) => {
                        // transient (e.g. minimized target) — keep last clip, retry
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(1));
    }
    let _ = release();
}

fn resolve_rect(mode: CageMode, padding: i32) -> Result<ScreenRect, ConfineError> {
    let base = match mode {
        CageMode::Window { hwnd } => read_window_rect(hwnd)?,
        CageMode::Fixed(rect) => rect,
    };
    let rect = if padding == 0 {
        base
    } else {
        base.inset(padding)
    };
    if !rect.is_valid() {
        return Err(ConfineError::InvalidRect(rect));
    }
    Ok(rect)
}

fn read_window_rect(hwnd_raw: isize) -> Result<ScreenRect, ConfineError> {
    if hwnd_raw == 0 {
        return Err(ConfineError::NoHandle);
    }
    let hwnd = HWND(hwnd_raw as *mut std::ffi::c_void);

    if !unsafe { IsWindow(hwnd) }.as_bool() {
        return Err(ConfineError::NoHandle);
    }
    if !unsafe { IsWindowVisible(hwnd) }.as_bool() {
        return Err(ConfineError::NoHandle);
    }
    if unsafe { IsIconic(hwnd) }.as_bool() {
        return Err(ConfineError::InvalidRect(ScreenRect::new(0, 0, 0, 0)));
    }

    let mut frame = RECT::default();
    let dwm_ok = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut frame as *mut _ as *mut std::ffi::c_void,
            std::mem::size_of::<RECT>() as u32,
        )
    }
    .is_ok();

    let rect = if dwm_ok && frame.right > frame.left && frame.bottom > frame.top {
        frame
    } else {
        let mut r = RECT::default();
        unsafe {
            GetWindowRect(hwnd, &mut r).map_err(|_| ConfineError::Win32(GetLastError().0))?;
        }
        r
    };

    let r = ScreenRect::new(rect.left, rect.top, rect.right, rect.bottom);
    if r.is_valid() {
        Ok(r)
    } else {
        Err(ConfineError::InvalidRect(r))
    }
}

fn apply(rect: ScreenRect) -> Result<(), ConfineError> {
    if !rect.is_valid() {
        return Err(ConfineError::InvalidRect(rect));
    }
    let win_rect = rect.to_win32();
    unsafe {
        ClipCursor(Some(&win_rect)).map_err(|_| {
            let code = GetLastError().0;
            ConfineError::Win32(code)
        })
    }
}

fn release() -> Result<(), ConfineError> {
    unsafe {
        ClipCursor(None).map_err(|_| {
            let code = GetLastError().0;
            ConfineError::Win32(code)
        })
    }
}
