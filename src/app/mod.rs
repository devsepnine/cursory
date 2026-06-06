use std::time::{Duration, Instant};

use iced::{Subscription, Task, Theme, time, window};

use crate::autostart;
use crate::confine::{CageMode, ClipController, ScreenRect};
pub use crate::domain::{CloseBehavior, Mode};
use crate::hotkey::{self, Captured, HotkeyService};
use crate::icon::{self, IconState};
use crate::monitor::{self, MonitorInfo};
use crate::settings::{self, Settings};
use crate::single_instance;
use crate::target::{self, WindowInfo};
use crate::tray::{TrayEvent, TrayService};

mod draw;
mod logic;
mod panels;
mod settings_view;
mod style;
mod update;
mod update_draw;
mod update_hotkey;
mod view;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Left,
    Top,
    Width,
    Height,
    Padding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowChoice {
    pub hwnd: isize,
    pub label: String,
}

impl std::fmt::Display for WindowChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

pub struct App {
    mode: Mode,
    is_active: bool,
    monitors: Vec<MonitorInfo>,
    selected_monitor: usize,
    custom_left: String,
    custom_top: String,
    custom_width: String,
    custom_height: String,
    padding: String,
    window_id: Option<window::Id>,
    window_hwnd: Option<isize>,
    external_windows: Vec<WindowInfo>,
    selected_window_hwnd: Option<isize>,
    minimize_on_activate: bool,
    launch_on_startup: bool,
    start_in_tray: bool,
    close_behavior: CloseBehavior,
    recording_hotkey: bool,
    pending_hotkey: Option<Captured>,
    pending_hotkey_mods: iced::keyboard::Modifiers,
    reset_pending: bool,
    drawing_rect: bool,
    rect_window_id: Option<window::Id>,
    hidden_to_tray: bool,
    checking_minimized: bool,
    hotkey: Option<HotkeyService>,
    tray: Option<TrayService>,
    controller: ClipController,
    status: String,
    last_monitor_check: Instant,
}

const MONITOR_POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Clone)]
pub enum Message {
    ModeSelected(Mode),
    MonitorSelectedByIndex(usize),
    WindowTargetSelected(WindowChoice),
    FieldChanged(Field, String),
    ToggleActive,
    RefreshMonitors,
    RefreshWindows,
    Tick,
    WindowOpened(window::Id),
    HwndCaptured(Option<isize>),
    MinimizeOnActivateToggled(bool),
    LaunchOnStartupToggled(bool),
    StartInTrayToggled(bool),
    CloseBehaviorSelected(CloseBehavior),
    StartHotkeyRecord,
    CancelHotkeyRecord,
    ConfirmHotkeyRecord,
    KeyCaptured(iced::keyboard::key::Physical, iced::keyboard::Modifiers),
    HotkeyModifiersChanged(iced::keyboard::Modifiers),
    DragWindow,
    MinimizeApp,
    CloseApp,
    CloseRequested(window::Id),
    StartDrawRect,
    ConfirmRect,
    CancelRect,
    RectWindowDrag,
    RectWindowResize(window::Direction),
    RectGeometryFetched {
        pos: Option<iced::Point>,
        size: iced::Size,
    },
    MinimizeStateChecked(Option<bool>),
    ResetSettings,
    Noop,
}

impl Default for App {
    fn default() -> Self {
        let saved = settings::load();
        let monitors = monitor::enumerate();
        let hotkey = HotkeyService::try_new(saved.hotkey.as_deref());
        let tray = TrayService::try_new();
        let status = match &hotkey {
            Some(svc) => format!("ready — global hotkey {}", svc.describe()),
            None => "ready — global hotkey unavailable".into(),
        };
        let selected_monitor = if monitors.is_empty() {
            0
        } else {
            saved.selected_monitor.min(monitors.len() - 1)
        };
        Self {
            mode: saved.mode,
            is_active: false,
            selected_monitor,
            custom_left: saved.custom_left.to_string(),
            custom_top: saved.custom_top.to_string(),
            custom_width: saved.custom_width.to_string(),
            custom_height: saved.custom_height.to_string(),
            padding: saved.padding.to_string(),
            window_id: None,
            window_hwnd: None,
            external_windows: target::enumerate(),
            selected_window_hwnd: None,
            minimize_on_activate: saved.minimize_on_activate,
            launch_on_startup: autostart::is_enabled(),
            start_in_tray: saved.start_in_tray,
            close_behavior: saved.close_behavior,
            recording_hotkey: false,
            pending_hotkey: None,
            pending_hotkey_mods: iced::keyboard::Modifiers::empty(),
            reset_pending: false,
            drawing_rect: false,
            rect_window_id: None,
            hidden_to_tray: false,
            checking_minimized: false,
            hotkey,
            tray,
            controller: ClipController::new(),
            monitors,
            status,
            last_monitor_check: Instant::now(),
        }
    }
}

impl App {
    pub fn title(&self) -> String {
        let suffix = if self.is_active { "active" } else { "idle" };
        format!("Cursory — {suffix}")
    }

    pub fn theme(&self) -> Theme {
        Theme::TokyoNightStorm
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let tick = time::every(Duration::from_millis(50)).map(|_| Message::Tick);
        let mut subs: Vec<Subscription<Message>> = vec![tick];
        if self.window_hwnd.is_none() {
            subs.push(iced::window::events().map(|(id, event)| match event {
                iced::window::Event::Opened { .. } => Message::WindowOpened(id),
                _ => Message::Noop,
            }));
        }
        subs.push(window::close_requests().map(Message::CloseRequested));
        if self.recording_hotkey {
            subs.push(iced::event::listen_with(
                |event, _status, _id| match event {
                    iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                        physical_key,
                        modifiers,
                        ..
                    }) => Some(Message::KeyCaptured(physical_key, modifiers)),
                    iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(
                        modifiers,
                    )) => Some(Message::HotkeyModifiersChanged(modifiers)),
                    _ => None,
                },
            ));
        }
        if self.drawing_rect {
            subs.push(iced::event::listen_with(
                |event, _status, _id| match event {
                    iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }) => {
                        use iced::keyboard::Key;
                        use iced::keyboard::key::Named;
                        match key {
                            Key::Named(Named::Escape) => Some(Message::CancelRect),
                            Key::Named(Named::Enter) => Some(Message::ConfirmRect),
                            _ => None,
                        }
                    }
                    _ => None,
                },
            ));
        }
        Subscription::batch(subs)
    }
}
