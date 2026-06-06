use std::cell::Cell;
use std::ffi::c_void;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::icon::IconState;

use windows::Win32::Foundation::{BOOL, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
    Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CREATESTRUCTW, CreateIconFromResourceEx, CreatePopupMenu, CreateWindowExW,
    DefWindowProcW, DestroyMenu, DestroyWindow, DispatchMessageW, GWLP_USERDATA, GetCursorPos,
    GetMessageW, GetWindowLongPtrW, HICON, HWND_MESSAGE, IMAGE_FLAGS, MF_SEPARATOR, MF_STRING, MSG,
    PostMessageW, PostQuitMessage, RegisterClassW, SetForegroundWindow, SetWindowLongPtrW,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, TranslateMessage, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_APP, WM_CREATE, WM_DESTROY, WM_LBUTTONDBLCLK, WM_NULL, WM_RBUTTONUP, WNDCLASSW,
};
use windows::core::{Error, PCWSTR, w};

const TRAY_UID: u32 = 1;
const WM_TRAY_ICON: u32 = WM_APP + 1;
const WM_TRAY_SHUTDOWN: u32 = WM_APP + 2;
const WM_TRAY_SET_ACTIVE: u32 = WM_APP + 3;
const IDLE_ICON: &[u8] = include_bytes!("../assets/icon.ico");
const ACTIVE_ICON: &[u8] = include_bytes!("../assets/icon-active.ico");

// Context-menu command ids (TrackPopupMenu with TPM_RETURNCMD returns these).
const CMD_SHOW: u32 = 1;
const CMD_TOGGLE: u32 = 2;
const CMD_ABOUT: u32 = 3;
const CMD_EXIT: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayEvent {
    Restore,
    Toggle,
    About,
    Exit,
}

/// Per-window context handed to the tray window via its creation lparam and
/// read back from GWLP_USERDATA. It lives on `run_tray_window`'s stack for the
/// whole message loop and is only touched on the tray thread, so the interior
/// `Cell` needs no synchronization. `active` mirrors the cage state so the
/// context menu can show the right Activate/Release label.
struct TrayContext {
    sender: Sender<TrayEvent>,
    active: Cell<bool>,
}

pub struct TrayService {
    hwnd: isize,
    events: Receiver<TrayEvent>,
    handle: Option<JoinHandle<()>>,
}

impl TrayService {
    pub fn try_new() -> Option<Self> {
        let (event_tx, event_rx) = mpsc::channel();
        let (hwnd_tx, hwnd_rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            if let Err(error) = run_tray_window(event_tx, hwnd_tx) {
                eprintln!("tray icon unavailable: {error}");
            }
        });

        match hwnd_rx.recv().ok().flatten() {
            Some(hwnd) => Some(Self {
                hwnd,
                events: event_rx,
                handle: Some(handle),
            }),
            None => {
                let _ = handle.join();
                None
            }
        }
    }

    pub fn poll(&self) -> Option<TrayEvent> {
        self.events.try_recv().ok()
    }

    pub fn set_active(&self, active: bool) {
        unsafe {
            let _ = PostMessageW(
                HWND(self.hwnd as _),
                WM_TRAY_SET_ACTIVE,
                WPARAM(usize::from(active)),
                LPARAM(0),
            );
        }
    }
}

impl Drop for TrayService {
    fn drop(&mut self) {
        unsafe {
            let _ = PostMessageW(HWND(self.hwnd as _), WM_TRAY_SHUTDOWN, WPARAM(0), LPARAM(0));
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn run_tray_window(
    event_tx: Sender<TrayEvent>,
    hwnd_tx: Sender<Option<isize>>,
) -> windows::core::Result<()> {
    // `ctx` lives on this stack frame for the whole message loop. The window
    // proc reaches it through a borrowed pointer stashed in GWLP_USERDATA (set
    // from the CreateWindowExW lparam), so there is no shared global and no
    // ownership handed to the window. Everything runs on this thread, so the
    // borrow is single-threaded; the pointer is only ever read while this frame
    // — and thus `ctx` — is alive.
    let ctx = TrayContext {
        sender: event_tx,
        active: Cell::new(false),
    };
    let hwnd = match unsafe { create_message_window(&ctx) } {
        Ok(hwnd) => hwnd,
        Err(error) => {
            let _ = hwnd_tx.send(None);
            return Err(error);
        }
    };

    if let Err(error) = add_icon(hwnd) {
        let _ = hwnd_tx.send(None);
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
        return Err(error);
    }

    let _ = hwnd_tx.send(Some(hwnd.0 as isize));

    let mut msg = MSG::default();
    while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    Ok(())
}

unsafe fn create_message_window(ctx: &TrayContext) -> windows::core::Result<HWND> {
    let instance = unsafe { GetModuleHandleW(None)? };
    let class_name = w!("CursoryTrayWindow");

    let class = WNDCLASSW {
        lpfnWndProc: Some(tray_wnd_proc),
        hInstance: instance.into(),
        lpszClassName: class_name,
        ..Default::default()
    };
    unsafe {
        RegisterClassW(&class);
    }

    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            w!("Cursory Tray"),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            instance,
            Some(ctx as *const TrayContext as *const c_void),
        )
    }
}

fn add_icon(hwnd: HWND) -> windows::core::Result<()> {
    let icon = tray_icon(IconState::Idle)?;
    let mut data = notify_data(hwnd);
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = WM_TRAY_ICON;
    data.hIcon = icon;
    write_wide_fixed(&mut data.szTip, "Cursory");

    if !unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool() {
        return Err(Error::from_win32());
    }

    Ok(())
}

fn update_icon(hwnd: HWND, state: IconState) -> windows::core::Result<()> {
    let mut data = notify_data(hwnd);
    data.uFlags = NIF_ICON | NIF_TIP;
    data.hIcon = tray_icon(state)?;
    match state {
        IconState::Idle => write_wide_fixed(&mut data.szTip, "Cursory"),
        IconState::Active => write_wide_fixed(&mut data.szTip, "Cursory - active"),
    }
    if !unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) }.as_bool() {
        return Err(Error::from_win32());
    }
    Ok(())
}

fn tray_icon(state: IconState) -> windows::core::Result<HICON> {
    let icon_bytes = match state {
        IconState::Idle => IDLE_ICON,
        IconState::Active => ACTIVE_ICON,
    };
    let image = best_ico_image(icon_bytes).ok_or_else(Error::from_win32)?;
    unsafe { CreateIconFromResourceEx(image, BOOL(1), 0x0003_0000, 64, 64, IMAGE_FLAGS(0)) }
}

fn best_ico_image(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.len() < 6 || u16::from_le_bytes([bytes[2], bytes[3]]) != 1 {
        return None;
    }
    let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
    let mut best: Option<(u32, &[u8])> = None;
    for index in 0..count {
        let entry = 6 + index * 16;
        if entry + 16 > bytes.len() {
            return None;
        }
        let width = match bytes[entry] {
            0 => 256,
            value => value as u32,
        };
        let size = u32::from_le_bytes([
            bytes[entry + 8],
            bytes[entry + 9],
            bytes[entry + 10],
            bytes[entry + 11],
        ]) as usize;
        let offset = u32::from_le_bytes([
            bytes[entry + 12],
            bytes[entry + 13],
            bytes[entry + 14],
            bytes[entry + 15],
        ]) as usize;
        if offset + size > bytes.len() {
            return None;
        }
        let image = &bytes[offset..offset + size];
        if best.is_none_or(|(best_width, _)| width > best_width) {
            best = Some((width, image));
        }
    }
    best.map(|(_, image)| image)
}

fn delete_icon(hwnd: HWND) {
    let data = notify_data(hwnd);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &data);
    }
}

fn notify_data(hwnd: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_UID,
        ..Default::default()
    }
}

fn write_wide_fixed(target: &mut [u16], text: &str) {
    for (slot, value) in target.iter_mut().zip(text.encode_utf16()) {
        *slot = value;
    }
}

unsafe extern "system" fn tray_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            // Stash the borrowed TrayContext pointer (passed as the
            // CreateWindowExW lparam) so later messages can reach it.
            let create = lparam.0 as *const CREATESTRUCTW;
            let context = unsafe { (*create).lpCreateParams };
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, context as isize);
            }
            LRESULT(0)
        }
        WM_TRAY_ICON => {
            if let Some(ctx) = unsafe { tray_context(hwnd) } {
                match lparam.0 as u32 {
                    WM_LBUTTONDBLCLK => {
                        let _ = ctx.sender.send(TrayEvent::Restore);
                    }
                    WM_RBUTTONUP => unsafe { show_context_menu(hwnd, ctx) },
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_TRAY_SHUTDOWN => {
            delete_icon(hwnd);
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            LRESULT(0)
        }
        WM_TRAY_SET_ACTIVE => {
            let active = wparam.0 != 0;
            if let Some(ctx) = unsafe { tray_context(hwnd) } {
                ctx.active.set(active);
            }
            let state = if active {
                IconState::Active
            } else {
                IconState::Idle
            };
            let _ = update_icon(hwnd, state);
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Recover the borrowed `TrayContext` stashed in GWLP_USERDATA on WM_CREATE.
unsafe fn tray_context<'a>(hwnd: HWND) -> Option<&'a TrayContext> {
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const TrayContext;
    unsafe { ptr.as_ref() }
}

/// Build and track the right-click context menu, then forward the chosen
/// command as a `TrayEvent`. Runs on the tray thread inside the WM_RBUTTONUP
/// handler.
unsafe fn show_context_menu(hwnd: HWND, ctx: &TrayContext) {
    let menu = match unsafe { CreatePopupMenu() } {
        Ok(menu) => menu,
        Err(_) => return,
    };
    let toggle_label = if ctx.active.get() {
        w!("Release confine")
    } else {
        w!("Activate confine")
    };
    unsafe {
        let _ = AppendMenuW(menu, MF_STRING, CMD_SHOW as usize, w!("Show Cursory"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, CMD_TOGGLE as usize, toggle_label);
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, CMD_ABOUT as usize, w!("About Cursory"));
        let _ = AppendMenuW(menu, MF_STRING, CMD_EXIT as usize, w!("Exit"));
    }

    let mut point = POINT::default();
    let command = unsafe {
        let _ = GetCursorPos(&mut point);
        // The owner must be foreground or the menu won't dismiss on an outside
        // click; the trailing WM_NULL post is the documented workaround.
        let _ = SetForegroundWindow(hwnd);
        let command = TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON | TPM_RETURNCMD,
            point.x,
            point.y,
            0,
            hwnd,
            None,
        );
        let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        command
    };

    let event = match command.0 as u32 {
        CMD_SHOW => Some(TrayEvent::Restore),
        CMD_TOGGLE => Some(TrayEvent::Toggle),
        CMD_ABOUT => Some(TrayEvent::About),
        CMD_EXIT => Some(TrayEvent::Exit),
        _ => None,
    };
    if let Some(event) = event {
        let _ = ctx.sender.send(event);
    }
}
