use std::time::{Duration, Instant};

use iced::{Subscription, Task, Theme, time, window};

use crate::autostart;
use crate::confine::{CageMode, ClipController};
pub use crate::domain::{CloseBehavior, Mode};
use crate::hotkey::{self, HotkeyService};
use custom_rect::CustomRect;
use draw_session::DrawSession;
use hotkey_recorder::HotkeyRecorder;
use tray_state::TrayState;
use crate::icon::{self, IconState};
use crate::monitor::{self, MonitorInfo};
use crate::settings::{self, Settings};
use crate::single_instance;
use crate::target::{self, WindowInfo};
use crate::tray::{TrayEvent, TrayService};

mod about;
mod custom_rect;
mod draw;
mod draw_session;
mod hotkey_recorder;
mod logic;
mod panels;
mod settings_view;
mod style;
mod tray_state;
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
    custom: CustomRect,
    padding: String,
    window_id: Option<window::Id>,
    window_hwnd: Option<isize>,
    external_windows: Vec<WindowInfo>,
    selected_window_hwnd: Option<isize>,
    minimize_on_activate: bool,
    launch_on_startup: bool,
    start_in_tray: bool,
    close_behavior: CloseBehavior,
    recorder: HotkeyRecorder,
    reset_pending: bool,
    draw: DrawSession,
    about_window_id: Option<window::Id>,
    tray_state: TrayState,
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
    CloseAbout,
    OpenUrl(&'static str),
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
            custom: CustomRect::new(
                saved.custom_left,
                saved.custom_top,
                saved.custom_width,
                saved.custom_height,
            ),
            padding: saved.padding.to_string(),
            window_id: None,
            window_hwnd: None,
            external_windows: target::enumerate(),
            selected_window_hwnd: None,
            minimize_on_activate: saved.minimize_on_activate,
            launch_on_startup: autostart::is_enabled(),
            start_in_tray: saved.start_in_tray,
            close_behavior: saved.close_behavior,
            recorder: HotkeyRecorder::default(),
            reset_pending: false,
            draw: DrawSession::default(),
            about_window_id: None,
            tray_state: TrayState::default(),
            hotkey,
            tray,
            controller: ClipController::new(),
            monitors,
            status,
            last_monitor_check: Instant::now(),
        }
    }
}

fn main_window_settings() -> window::Settings {
    window::Settings {
        size: iced::Size::new(460.0, 880.0),
        resizable: false,
        decorations: false,
        transparent: true,
        exit_on_close_request: false,
        ..Default::default()
    }
}

impl App {
    /// Daemon boot: build state and open the main window. The window id flows
    /// back through `WindowOpened`, where HWND capture and the icon are wired up.
    pub fn boot() -> (Self, Task<Message>) {
        let app = Self::default();
        let (_id, open) = window::open(main_window_settings());
        (app, open.map(Message::WindowOpened))
    }

    pub fn title(&self, window: window::Id) -> String {
        if Some(window) == self.about_window_id {
            return "About Cursory".to_string();
        }
        let suffix = if self.is_active { "active" } else { "idle" };
        format!("Cursory — {suffix}")
    }

    pub fn theme(&self, _window: window::Id) -> Theme {
        Theme::TokyoNightStorm
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let tick = time::every(Duration::from_millis(50)).map(|_| Message::Tick);
        let mut subs: Vec<Subscription<Message>> = vec![tick];
        subs.push(window::close_requests().map(Message::CloseRequested));
        if self.recorder.is_recording() {
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
        if self.draw.is_active() {
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
