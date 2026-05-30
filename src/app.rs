use std::time::{Duration, Instant};

use iced::mouse;
use iced::widget::{
    Space, button, canvas, checkbox, column, container, mouse_area, pick_list, radio, row,
    scrollable, stack, text, text_input,
};
use iced::{Element, Length, Subscription, Task, Theme, time, window};

use crate::autostart;
use crate::confine::{CageMode, ClipController, ScreenRect};
use crate::hotkey::{self, Captured, HotkeyService};
use crate::icon::{self, IconState};
use crate::monitor::{self, MonitorInfo};
use crate::monitor_preview::MonitorPreview;
use crate::settings::{self, Settings};
use crate::single_instance;
use crate::target::{self, WindowInfo};
use crate::tray::{TrayEvent, TrayService};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Window,
    Monitor,
    Custom,
}

impl Mode {
    #[allow(dead_code)]
    pub fn label(self) -> &'static str {
        match self {
            Mode::Window => "App window",
            Mode::Monitor => "Monitor",
            Mode::Custom => "Custom rect",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseBehavior {
    ToTray,
    Exit,
}

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

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ModeSelected(mode) => {
                self.mode = mode;
                self.refresh_controller_mode();
                self.persist();
            }
            Message::MonitorSelectedByIndex(i) => {
                if i < self.monitors.len() {
                    self.selected_monitor = i;
                    if matches!(self.mode, Mode::Monitor) {
                        self.refresh_controller_mode();
                    }
                    self.persist();
                }
            }
            Message::FieldChanged(field, value) => {
                // Only persist a complete, parseable value so a mid-edit string
                // like "-" or "" never overwrites the saved geometry. Padding may
                // be empty (treated as 0).
                let valid = match field {
                    Field::Padding => {
                        value.trim().is_empty() || value.trim().parse::<i32>().is_ok()
                    }
                    _ => value.trim().parse::<i32>().is_ok(),
                };
                let target = match field {
                    Field::Left => &mut self.custom_left,
                    Field::Top => &mut self.custom_top,
                    Field::Width => &mut self.custom_width,
                    Field::Height => &mut self.custom_height,
                    Field::Padding => &mut self.padding,
                };
                *target = value;
                if matches!(field, Field::Padding) {
                    self.refresh_padding();
                } else if matches!(self.mode, Mode::Custom) {
                    self.refresh_controller_mode();
                }
                if valid {
                    self.persist();
                }
            }
            Message::ToggleActive => {
                return self.toggle();
            }
            Message::RefreshMonitors => {
                self.monitors = monitor::enumerate();
                if self.selected_monitor >= self.monitors.len() {
                    self.selected_monitor = 0;
                }
                self.status = format!("monitors refreshed ({} found)", self.monitors.len());
                if matches!(self.mode, Mode::Monitor) {
                    self.refresh_controller_mode();
                }
            }
            Message::RefreshWindows => {
                self.external_windows = target::enumerate();
                self.status = format!("windows refreshed ({} found)", self.external_windows.len());
                if matches!(self.mode, Mode::Window) {
                    self.refresh_controller_mode();
                }
            }
            Message::WindowTargetSelected(choice) => {
                self.selected_window_hwnd = Some(choice.hwnd);
                if matches!(self.mode, Mode::Window) {
                    self.refresh_controller_mode();
                }
            }
            Message::Tick => {
                self.poll_monitor_changes();
                if single_instance::poll_show_request() {
                    return self.restore_from_tray();
                }
                if let Some(tray) = self.tray.as_ref() {
                    if matches!(tray.poll(), Some(TrayEvent::RestoreRequested)) {
                        return self.restore_from_tray();
                    }
                }
                if let Some(reason) = self.controller.take_auto_release() {
                    self.is_active = false;
                    self.status = format!("auto-released: {reason}");
                    self.set_tray_active(false);
                    if let Some(id) = self.window_id {
                        return Task::batch([
                            window::set_mode::<Message>(id, window::Mode::Windowed),
                            window::gain_focus::<Message>(id),
                            self.window_icon_task(IconState::Idle),
                        ]);
                    }
                    return self.window_icon_task(IconState::Idle);
                }
                if let Some(svc) = self.hotkey.as_ref() {
                    if self.recording_hotkey {
                        svc.drain();
                    } else if svc.poll_toggle() {
                        return self.toggle();
                    }
                }
                if !self.hidden_to_tray && !self.checking_minimized {
                    if let Some(id) = self.window_id {
                        self.checking_minimized = true;
                        return window::is_minimized(id).map(Message::MinimizeStateChecked);
                    }
                }
            }
            Message::WindowOpened(id) => {
                self.window_id = Some(id);
                let hwnd_task = window::raw_id::<Message>(id)
                    .map(|raw| Message::HwndCaptured(Some(raw as isize)));
                if let Some(icon) = icon::window_icon(IconState::Idle) {
                    return Task::batch([hwnd_task, window::set_icon::<Message>(id, icon)]);
                }
                return hwnd_task;
            }
            Message::MinimizeOnActivateToggled(v) => {
                self.minimize_on_activate = v;
                self.persist();
            }
            Message::LaunchOnStartupToggled(v) => match autostart::set_enabled(v) {
                Ok(state) => {
                    self.launch_on_startup = state;
                    self.status = if state {
                        "launch on startup enabled".into()
                    } else {
                        "launch on startup disabled".into()
                    };
                }
                Err(e) => {
                    self.status = format!("startup setting failed: {e}");
                }
            },
            Message::CloseBehaviorSelected(behavior) => {
                self.close_behavior = behavior;
                self.persist();
            }
            Message::StartHotkeyRecord => {
                if self.hotkey.is_some() {
                    self.recording_hotkey = true;
                    self.pending_hotkey = None;
                    self.pending_hotkey_mods = iced::keyboard::Modifiers::empty();
                    self.status = "press combo, then Confirm (esc to cancel)".into();
                }
            }
            Message::CancelHotkeyRecord => {
                self.recording_hotkey = false;
                self.pending_hotkey = None;
                self.pending_hotkey_mods = iced::keyboard::Modifiers::empty();
                self.status = "hotkey unchanged".into();
            }
            Message::ConfirmHotkeyRecord => {
                if let Some(captured) = self.pending_hotkey {
                    if let Some(svc) = self.hotkey.as_mut() {
                        match svc.rebind(captured.0, captured.1) {
                            Ok(()) => {
                                self.recording_hotkey = false;
                                self.pending_hotkey = None;
                                self.pending_hotkey_mods = iced::keyboard::Modifiers::empty();
                                self.status = format!("hotkey set to {}", svc.describe());
                                self.persist();
                            }
                            Err(e) => {
                                // keep recording mode open so the user can try another combo
                                self.pending_hotkey = None;
                                self.pending_hotkey_mods =
                                    iced::keyboard::Modifiers::empty();
                                self.status = format!("{e} — try another combo");
                            }
                        }
                    }
                }
            }
            Message::KeyCaptured(physical, modifiers) => {
                if hotkey::is_cancel_key(physical) {
                    self.recording_hotkey = false;
                    self.pending_hotkey = None;
                    self.pending_hotkey_mods = iced::keyboard::Modifiers::empty();
                    self.status = "hotkey unchanged".into();
                    return Task::none();
                }
                self.pending_hotkey_mods = modifiers;
                if let Some(captured) = hotkey::from_iced(physical, modifiers) {
                    self.pending_hotkey = Some(captured);
                    self.status = format!(
                        "preview: {} — click Confirm",
                        hotkey::describe_captured(captured)
                    );
                } else {
                    let mods_str = hotkey::describe_modifiers(modifiers);
                    if mods_str.is_empty() {
                        self.status = "press combo, then Confirm".into();
                    } else {
                        self.status = format!("preview: {mods_str}+_  press a key");
                    }
                }
            }
            Message::HotkeyModifiersChanged(modifiers) => {
                self.pending_hotkey_mods = modifiers;
            }
            Message::HwndCaptured(hwnd) => {
                self.window_hwnd = hwnd;
                if hwnd.is_none() {
                    self.status = "HWND capture failed".into();
                } else if matches!(self.mode, Mode::Window) {
                    self.refresh_controller_mode();
                }
            }
            Message::DragWindow => {
                if let Some(id) = self.window_id {
                    return window::drag::<Message>(id);
                }
            }
            Message::MinimizeApp => {
                return self.hide_to_tray();
            }
            Message::CloseApp => {
                return self.close_app();
            }
            Message::CloseRequested(id) => {
                if Some(id) == self.rect_window_id {
                    self.status = "draw cancelled".into();
                    return self.exit_draw_mode();
                }
                if Some(id) == self.window_id {
                    return self.close_app();
                }
                return window::close::<Message>(id);
            }
            Message::StartDrawRect => {
                if self.drawing_rect {
                    return Task::none();
                }
                self.drawing_rect = true;
                let settings = window::Settings {
                    size: iced::Size::new(800.0, 600.0),
                    position: window::Position::Centered,
                    decorations: false,
                    resizable: true,
                    transparent: true,
                    level: window::Level::AlwaysOnTop,
                    ..Default::default()
                };
                let (rect_id, open_task) = window::open(settings);
                self.rect_window_id = Some(rect_id);
                let mut tasks = vec![open_task.map(|_| Message::Noop)];
                if let Some(main_id) = self.window_id {
                    tasks.push(window::set_mode::<Message>(main_id, window::Mode::Hidden));
                }
                return Task::batch(tasks);
            }
            Message::ConfirmRect => {
                if let Some(rect_id) = self.rect_window_id {
                    return window::position(rect_id).then(move |pos| {
                        window::size(rect_id)
                            .map(move |size| Message::RectGeometryFetched { pos, size })
                    });
                }
            }
            Message::CancelRect => {
                self.status = "draw cancelled".into();
                return self.exit_draw_mode();
            }
            Message::RectWindowDrag => {
                if let Some(id) = self.rect_window_id {
                    return window::drag::<Message>(id);
                }
            }
            Message::RectWindowResize(dir) => {
                if let Some(id) = self.rect_window_id {
                    return window::drag_resize::<Message>(id, dir);
                }
            }
            Message::RectGeometryFetched { pos, size } => {
                if let Some(p) = pos {
                    let left = p.x.round() as i32;
                    let top = p.y.round() as i32;
                    let w = size.width.round().max(1.0) as i32;
                    let h = size.height.round().max(1.0) as i32;
                    self.custom_left = left.to_string();
                    self.custom_top = top.to_string();
                    self.custom_width = w.to_string();
                    self.custom_height = h.to_string();
                    self.status = format!("rect set: {}×{} at ({},{})", w, h, left, top);
                    if matches!(self.mode, Mode::Custom) {
                        self.refresh_controller_mode();
                    }
                    self.persist();
                } else {
                    self.status = "failed to read rect window position".into();
                }
                return self.exit_draw_mode();
            }
            Message::MinimizeStateChecked(is_minimized) => {
                self.checking_minimized = false;
                if matches!(is_minimized, Some(true)) {
                    return self.hide_to_tray();
                }
            }
            Message::ResetSettings => {
                if self.reset_pending {
                    self.apply_defaults();
                    self.status = "settings reset to defaults".into();
                    self.persist();
                    if self.is_active {
                        self.refresh_controller_mode();
                    }
                } else {
                    self.reset_pending = true;
                    self.status = "click Reset again to confirm".into();
                }
            }
            Message::Noop => {}
        }
        Task::none()
    }

    fn toggle(&mut self) -> Task<Message> {
        if self.is_active {
            self.is_active = false;
            self.controller.deactivate();
            self.status = "released".into();
            self.set_tray_active(false);
            if self.minimize_on_activate {
                if let Some(id) = self.window_id {
                    return Task::batch([
                        window::minimize::<Message>(id, false),
                        self.window_icon_task(IconState::Idle),
                    ]);
                }
            }
            return self.window_icon_task(IconState::Idle);
        }
        let (mode, padding) = match (self.build_cage_mode(), self.parse_padding()) {
            (Ok(m), Ok(p)) => (m, p),
            (Err(e), _) | (_, Err(e)) => {
                self.status = format!("cannot activate: {e}");
                return Task::none();
            }
        };
        match self.controller.activate(mode, padding) {
            Ok(()) => {
                self.is_active = true;
                self.set_tray_active(true);
                self.status = match mode {
                    CageMode::Window { .. } => "active — app window".into(),
                    CageMode::Fixed(r) => format!(
                        "active — {}×{} at ({},{})",
                        r.width(),
                        r.height(),
                        r.left,
                        r.top
                    ),
                };
                let target_is_self = matches!(
                    mode,
                    CageMode::Window { hwnd } if Some(hwnd) == self.window_hwnd
                );
                if self.minimize_on_activate && !target_is_self {
                    if let Some(id) = self.window_id {
                        return Task::batch([
                            window::minimize::<Message>(id, true),
                            self.window_icon_task(IconState::Active),
                        ]);
                    }
                }
                self.window_icon_task(IconState::Active)
            }
            Err(e) => {
                self.status = format!("activate error: {e}");
                Task::none()
            }
        }
    }

    fn persist(&mut self) {
        let settings = Settings {
            mode: self.mode,
            selected_monitor: self.selected_monitor,
            custom_left: self.custom_left.trim().parse().unwrap_or(100),
            custom_top: self.custom_top.trim().parse().unwrap_or(100),
            custom_width: self.custom_width.trim().parse().unwrap_or(800),
            custom_height: self.custom_height.trim().parse().unwrap_or(600),
            padding: self.padding.trim().parse().unwrap_or(0),
            minimize_on_activate: self.minimize_on_activate,
            close_behavior: self.close_behavior,
            hotkey: self.hotkey.as_ref().and_then(|h| h.serialize()),
        };
        if let Err(e) = settings::save(&settings) {
            self.status = e;
        }
        self.reset_pending = false;
    }

    fn apply_defaults(&mut self) {
        let d = Settings::default();
        self.mode = d.mode;
        self.selected_monitor = if self.monitors.is_empty() {
            0
        } else {
            d.selected_monitor.min(self.monitors.len() - 1)
        };
        self.custom_left = d.custom_left.to_string();
        self.custom_top = d.custom_top.to_string();
        self.custom_width = d.custom_width.to_string();
        self.custom_height = d.custom_height.to_string();
        self.padding = d.padding.to_string();
        self.minimize_on_activate = d.minimize_on_activate;
        self.close_behavior = d.close_behavior;
        if let Some(svc) = self.hotkey.as_mut() {
            let _ = svc.reset_default();
        }
        self.reset_pending = false;
    }

    fn poll_monitor_changes(&mut self) {
        if self.last_monitor_check.elapsed() < MONITOR_POLL_INTERVAL {
            return;
        }
        self.last_monitor_check = Instant::now();
        let fresh = monitor::enumerate();
        if fresh == self.monitors {
            return;
        }
        self.monitors = fresh;
        if self.selected_monitor >= self.monitors.len() {
            self.selected_monitor = 0;
        }
        self.status = format!("display changed ({} monitor(s))", self.monitors.len());
        if matches!(self.mode, Mode::Monitor) {
            self.refresh_controller_mode();
        }
    }

    fn refresh_controller_mode(&mut self) {
        if !self.is_active {
            return;
        }
        match self.build_cage_mode() {
            Ok(mode) => {
                if let Err(e) = self.controller.set_mode(mode) {
                    self.status = format!("update error: {e}");
                }
            }
            Err(e) => {
                self.is_active = false;
                self.controller.deactivate();
                self.set_tray_active(false);
                self.status = format!("auto-released: {e}");
            }
        }
    }

    fn refresh_padding(&mut self) {
        if !self.is_active {
            return;
        }
        match self.parse_padding() {
            Ok(p) => {
                if let Err(e) = self.controller.set_padding(p) {
                    self.status = format!("padding error: {e}");
                }
            }
            Err(e) => {
                self.status = format!("padding: {e}");
            }
        }
    }

    fn build_cage_mode(&self) -> Result<CageMode, String> {
        match self.mode {
            Mode::Window => {
                let hwnd = self
                    .selected_window_hwnd
                    .or(self.window_hwnd)
                    .ok_or("no target window selected")?;
                Ok(CageMode::Window { hwnd })
            }
            Mode::Monitor => {
                if self.monitors.is_empty() {
                    return Err("no monitors enumerated".into());
                }
                let idx = self.selected_monitor.min(self.monitors.len() - 1);
                Ok(CageMode::Fixed(self.monitors[idx].bounds))
            }
            Mode::Custom => {
                let left: i32 = self
                    .custom_left
                    .trim()
                    .parse()
                    .map_err(|_| "left not a number")?;
                let top: i32 = self
                    .custom_top
                    .trim()
                    .parse()
                    .map_err(|_| "top not a number")?;
                let width: i32 = self
                    .custom_width
                    .trim()
                    .parse()
                    .map_err(|_| "width not a number")?;
                let height: i32 = self
                    .custom_height
                    .trim()
                    .parse()
                    .map_err(|_| "height not a number")?;
                if width <= 0 || height <= 0 {
                    return Err("width/height must be positive".into());
                }
                Ok(CageMode::Fixed(ScreenRect::from_xywh(
                    left, top, width, height,
                )))
            }
        }
    }

    fn exit_draw_mode(&mut self) -> Task<Message> {
        self.drawing_rect = false;
        let rect_id = self.rect_window_id.take();
        let main_id = self.window_id;
        let mut tasks: Vec<Task<Message>> = Vec::new();
        if let Some(rid) = rect_id {
            tasks.push(window::close::<Message>(rid));
        }
        if let Some(mid) = main_id {
            tasks.push(window::set_mode::<Message>(mid, window::Mode::Windowed));
            tasks.push(window::gain_focus::<Message>(mid));
        }
        Task::batch(tasks)
    }

    fn hide_to_tray(&mut self) -> Task<Message> {
        self.checking_minimized = false;
        if self.tray.is_none() {
            self.status = "tray unavailable — minimized".into();
            if let Some(id) = self.window_id {
                return window::minimize::<Message>(id, true);
            }
            return Task::none();
        }

        self.hidden_to_tray = true;
        self.status = "hidden to tray — double-click tray icon to restore".into();
        if let Some(id) = self.window_id {
            return Task::batch([
                window::minimize::<Message>(id, false),
                window::set_mode::<Message>(id, window::Mode::Hidden),
            ]);
        }
        Task::none()
    }

    fn restore_from_tray(&mut self) -> Task<Message> {
        self.hidden_to_tray = false;
        self.status = "restored".into();
        if let Some(id) = self.window_id {
            return Task::batch([
                window::set_mode::<Message>(id, window::Mode::Windowed),
                window::minimize::<Message>(id, false),
                window::gain_focus::<Message>(id),
            ]);
        }
        Task::none()
    }

    fn close_app(&mut self) -> Task<Message> {
        match self.close_behavior {
            CloseBehavior::ToTray => self.hide_to_tray(),
            CloseBehavior::Exit => {
                if let Some(id) = self.window_id {
                    window::close::<Message>(id)
                } else {
                    Task::none()
                }
            }
        }
    }

    fn set_tray_active(&self, active: bool) {
        if let Some(tray) = self.tray.as_ref() {
            tray.set_active(active);
        }
    }

    fn window_icon_task(&self, state: IconState) -> Task<Message> {
        match (self.window_id, icon::window_icon(state)) {
            (Some(id), Some(icon)) => window::set_icon::<Message>(id, icon),
            _ => Task::none(),
        }
    }

    fn parse_padding(&self) -> Result<i32, String> {
        if self.padding.trim().is_empty() {
            return Ok(0);
        }
        self.padding
            .trim()
            .parse::<i32>()
            .map_err(|_| "padding not a number".to_string())
    }

    pub fn view(&self) -> Element<'_, Message> {
        if self.drawing_rect {
            return self.draw_view();
        }
        let titlebar = self.titlebar();
        let content = column![
            section_label("MODE"),
            section_panel(self.mode_picker()),
            Space::new().height(Length::Fixed(4.0)),
            section_label("TARGET"),
            section_panel(self.target_panel()),
            Space::new().height(Length::Fixed(4.0)),
            section_label("SETTINGS"),
            section_panel(self.settings_panel()),
        ]
        .spacing(7)
        .padding([0, 16]);

        let bottom_bar = container(
            column![
                status_description(&self.status),
                Space::new().height(Length::Fixed(8.0)),
                self.action_button(),
            ]
            .spacing(0),
        )
        .padding([0, 16])
        .width(Length::Fill);

        // Only the middle section scrolls; the titlebar and the bottom action
        // bar stay pinned so the ACTIVATE button never gets squished when the
        // content is taller than the fixed window.
        let scroll_area = scrollable(content)
            .height(Length::Fill)
            .width(Length::Fill);

        let stack = column![
            titlebar,
            scroll_area,
            Space::new().height(Length::Fixed(10.0)),
            bottom_bar,
            Space::new().height(Length::Fixed(14.0))
        ];

        container(stack)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(root_style)
            .into()
    }

    fn draw_view(&self) -> Element<'_, Message> {
        const EDGE: f32 = 8.0;

        let confirm = button(text("Confirm").size(14))
            .padding([8, 22])
            .on_press(Message::ConfirmRect)
            .style(|theme: &Theme, status: button::Status| {
                let palette = theme.extended_palette();
                let (bg, tc) = match status {
                    button::Status::Hovered => {
                        (palette.primary.strong.color, palette.primary.strong.text)
                    }
                    button::Status::Pressed => {
                        (palette.primary.weak.color, palette.primary.weak.text)
                    }
                    _ => (palette.primary.base.color, palette.primary.base.text),
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: tc,
                    border: iced::Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            });

        let cancel = button(text("×").size(16))
            .padding([2, 8])
            .on_press(Message::CancelRect)
            .style(|theme: &Theme, status: button::Status| {
                let palette = theme.extended_palette();
                let (bg, tc) = match status {
                    button::Status::Hovered => (
                        iced::Background::Color(iced::Color::from_rgb(0.85, 0.25, 0.30)),
                        iced::Color::WHITE,
                    ),
                    _ => (
                        iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.45)),
                        palette.background.base.text,
                    ),
                };
                button::Style {
                    background: Some(bg),
                    text_color: tc,
                    border: iced::Border {
                        radius: 4.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            });

        let edge = |dir: window::Direction, cursor: mouse::Interaction| {
            mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
                .interaction(cursor)
                .on_press(Message::RectWindowResize(dir))
        };

        let n_edge: Element<'_, Message> = container(edge(
            window::Direction::North,
            mouse::Interaction::ResizingVertically,
        ))
        .width(Length::Fill)
        .height(Length::Fixed(EDGE))
        .into();
        let s_edge: Element<'_, Message> = container(edge(
            window::Direction::South,
            mouse::Interaction::ResizingVertically,
        ))
        .width(Length::Fill)
        .height(Length::Fixed(EDGE))
        .into();
        let w_edge: Element<'_, Message> = container(edge(
            window::Direction::West,
            mouse::Interaction::ResizingHorizontally,
        ))
        .width(Length::Fixed(EDGE))
        .height(Length::Fill)
        .into();
        let e_edge: Element<'_, Message> = container(edge(
            window::Direction::East,
            mouse::Interaction::ResizingHorizontally,
        ))
        .width(Length::Fixed(EDGE))
        .height(Length::Fill)
        .into();

        // corners (size EDGE x EDGE in stack overlay)
        let corner = |dir: window::Direction, cursor: mouse::Interaction| {
            mouse_area(
                Space::new()
                    .width(Length::Fixed(EDGE * 2.0))
                    .height(Length::Fixed(EDGE * 2.0)),
            )
            .interaction(cursor)
            .on_press(Message::RectWindowResize(dir))
        };

        let center_zone = mouse_area(
            container(column![
                row![Space::new().width(Length::Fill), cancel].padding([4, 4]),
                Space::new().height(Length::Fill),
                container(confirm).center_x(Length::Fill),
                Space::new().height(Length::Fill),
            ])
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .interaction(mouse::Interaction::Grab)
        .on_press(Message::RectWindowDrag);

        let middle_row = row![w_edge, center_zone, e_edge];

        let edges_layout = column![n_edge, middle_row, s_edge];

        // Stack: base = edges_layout, overlays = 4 corners positioned absolutely
        let corners_overlay = column![
            row![
                container(corner(
                    window::Direction::NorthWest,
                    mouse::Interaction::ResizingDiagonallyDown
                ))
                .align_left(Length::Shrink),
                Space::new().width(Length::Fill),
                container(corner(
                    window::Direction::NorthEast,
                    mouse::Interaction::ResizingDiagonallyUp
                ))
                .align_right(Length::Shrink),
            ],
            Space::new().height(Length::Fill),
            row![
                container(corner(
                    window::Direction::SouthWest,
                    mouse::Interaction::ResizingDiagonallyUp
                ))
                .align_left(Length::Shrink),
                Space::new().width(Length::Fill),
                container(corner(
                    window::Direction::SouthEast,
                    mouse::Interaction::ResizingDiagonallyDown
                ))
                .align_right(Length::Shrink),
            ],
        ];

        let stacked = stack![edges_layout, corners_overlay];

        container(stacked)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.25).into()),
                border: iced::Border {
                    color: iced::Color::from_rgb(0.95, 0.25, 0.30),
                    width: 2.0,
                    radius: 0.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn titlebar(&self) -> Element<'_, Message> {
        let badge: Element<'_, Message> = if self.is_active {
            container(text("ACTIVE").size(10).color(ColorToken::MintText.color()))
                .padding([3, 8])
                .style(active_badge_style)
                .into()
        } else {
            container(text("IDLE").size(10).color(ColorToken::Muted.color()))
                .padding([3, 8])
                .style(idle_badge_style)
                .into()
        };
        let drag_zone = mouse_area(
            container(
                row![
                    text("Cursory").size(16).color(ColorToken::Ink.color()),
                    Space::new().width(Length::Fixed(10.0)),
                    badge,
                    Space::new().width(Length::Fill),
                ]
                .align_y(iced::Alignment::Center),
            )
            .padding([10, 14])
            .width(Length::Fill),
        )
        .on_press(Message::DragWindow);

        let close = button(text("").size(1))
            .width(Length::Fixed(13.0))
            .height(Length::Fixed(13.0))
            .padding(0)
            .on_press(Message::CloseApp)
            .style(|_theme: &Theme, status| traffic_light_style(ColorToken::Coral.color(), status));
        let mini = button(text("").size(1))
            .width(Length::Fixed(13.0))
            .height(Length::Fixed(13.0))
            .padding(0)
            .on_press(Message::MinimizeApp)
            .style(|_theme: &Theme, status| traffic_light_style(ColorToken::Sun.color(), status));

        row![
            Space::new().width(Length::Fixed(14.0)),
            close,
            Space::new().width(Length::Fixed(7.0)),
            mini,
            Space::new().width(Length::Fixed(8.0)),
            drag_zone,
        ]
        .height(Length::Fixed(42.0))
        .align_y(iced::Alignment::Center)
        .into()
    }

    fn mode_picker(&self) -> Element<'_, Message> {
        column![
            radio(
                "App window",
                Mode::Window,
                Some(self.mode),
                Message::ModeSelected
            )
            .size(14),
            radio(
                "Monitor",
                Mode::Monitor,
                Some(self.mode),
                Message::ModeSelected
            )
            .size(14),
            radio(
                "Custom rect",
                Mode::Custom,
                Some(self.mode),
                Message::ModeSelected
            )
            .size(14),
        ]
        .spacing(4)
        .into()
    }

    fn target_panel(&self) -> Element<'_, Message> {
        match self.mode {
            Mode::Window => self.window_panel(),
            Mode::Monitor => self.monitor_panel(),
            Mode::Custom => self.custom_panel(),
        }
    }

    fn settings_panel(&self) -> Element<'_, Message> {
        let padding_row = row![
            text("Padding")
                .width(Length::Fixed(92.0))
                .size(13)
                .color(ColorToken::Ink.color()),
            text_input("0", &self.padding)
                .on_input(|s| Message::FieldChanged(Field::Padding, s))
                .width(Length::Fixed(80.0))
                .size(13),
            text("px").size(12),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        let mini = checkbox(self.minimize_on_activate)
            .label("Minimize this window on activate")
            .on_toggle(Message::MinimizeOnActivateToggled)
            .size(15)
            .text_size(13);

        let startup = checkbox(self.launch_on_startup)
            .label("Launch on Windows startup")
            .on_toggle(Message::LaunchOnStartupToggled)
            .size(15)
            .text_size(13);

        let label_text = if self.recording_hotkey {
            if let Some(captured) = self.pending_hotkey {
                format!("Preview  {}", hotkey::describe_captured(captured))
            } else {
                let mods_str = hotkey::describe_modifiers(self.pending_hotkey_mods);
                if mods_str.is_empty() {
                    "Recording  press a combo...".to_string()
                } else {
                    format!("Recording  {mods_str}+_")
                }
            }
        } else if let Some(svc) = self.hotkey.as_ref() {
            format!("Hotkey  {}", svc.describe())
        } else {
            "Hotkey unavailable".into()
        };

        let action: Element<'_, Message> = if self.recording_hotkey {
            let mut confirm_btn = button(text("Confirm").size(11))
                .padding([4, 10])
                .style(confirm_button_style);
            if self.pending_hotkey.is_some() {
                confirm_btn = confirm_btn.on_press(Message::ConfirmHotkeyRecord);
            }
            let cancel_btn = button(text("Cancel").size(11))
                .padding([4, 10])
                .on_press(Message::CancelHotkeyRecord)
                .style(secondary_button_style);
            row![confirm_btn, cancel_btn]
                .spacing(6)
                .align_y(iced::Alignment::Center)
                .into()
        } else if self.hotkey.is_some() {
            button(text("Change").size(11))
                .padding([4, 10])
                .on_press(Message::StartHotkeyRecord)
                .style(secondary_button_style)
                .into()
        } else {
            Space::new().width(Length::Shrink).into()
        };

        let hotkey_row = row![text(label_text).width(Length::Fill).size(12), action]
            .spacing(8)
            .align_y(iced::Alignment::Center);

        let close_behavior = column![
            text("Close button").size(13).color(ColorToken::Ink.color()),
            row![
                radio(
                    "Send to tray",
                    CloseBehavior::ToTray,
                    Some(self.close_behavior),
                    Message::CloseBehaviorSelected
                )
                .size(14),
                radio(
                    "Exit app",
                    CloseBehavior::Exit,
                    Some(self.close_behavior),
                    Message::CloseBehaviorSelected
                )
                .size(14),
            ]
            .spacing(12)
        ]
        .spacing(4);

        let reset_label = if self.reset_pending {
            "Click again to confirm reset"
        } else {
            "Reset to defaults"
        };
        let reset_btn: Element<'_, Message> = if self.reset_pending {
            button(text(reset_label).size(11))
                .padding([4, 10])
                .on_press(Message::ResetSettings)
                .style(danger_button_style)
                .into()
        } else {
            button(text(reset_label).size(11))
                .padding([4, 10])
                .on_press(Message::ResetSettings)
                .style(secondary_button_style)
                .into()
        };
        let reset_row = row![Space::new().width(Length::Fill), reset_btn]
            .align_y(iced::Alignment::Center);

        column![
            padding_row,
            mini,
            startup,
            close_behavior,
            hotkey_row,
            reset_row
        ]
        .spacing(8)
        .into()
    }

    fn action_button(&self) -> Element<'_, Message> {
        let is_active = self.is_active;
        let label = if is_active { "RELEASE" } else { "ACTIVATE" };
        button(
            text(label)
                .size(15)
                .color(iced::Color::WHITE)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center),
        )
        .padding([11, 0])
        .width(Length::Fill)
        .on_press(Message::ToggleActive)
        .style(move |_theme: &Theme, status: button::Status| {
            let base = if is_active {
                ColorToken::Coral.color()
            } else {
                ColorToken::Blue.color()
            };
            let bg_color = match status {
                button::Status::Hovered => lighten(base, 0.08),
                button::Status::Pressed => darken(base, 0.07),
                _ => base,
            };
            button::Style {
                background: Some(bg_color.into()),
                text_color: iced::Color::WHITE,
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
    }

    fn window_panel(&self) -> Element<'_, Message> {
        let self_hwnd = self.window_hwnd.unwrap_or(0);
        let mut options: Vec<WindowChoice> = Vec::new();
        if self_hwnd != 0 {
            options.push(WindowChoice {
                hwnd: self_hwnd,
                label: "(this app) Cursory".into(),
            });
        }
        for w in &self.external_windows {
            if w.hwnd == self_hwnd {
                continue;
            }
            options.push(WindowChoice {
                hwnd: w.hwnd,
                label: w.title.clone(),
            });
        }
        let selected_hwnd = self.selected_window_hwnd.unwrap_or(self_hwnd);
        let current = options.iter().find(|c| c.hwnd == selected_hwnd).cloned();
        column![
            pick_list(options, current, Message::WindowTargetSelected).width(Length::Fill),
            button(
                text("Refresh")
                    .size(13)
                    .color(ColorToken::Ink.color())
                    .width(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Center)
            )
            .padding([8, 16])
            .width(Length::Fill)
            .on_press(Message::RefreshWindows)
            .style(secondary_button_style),
        ]
        .spacing(8)
        .into()
    }

    fn monitor_panel(&self) -> Element<'_, Message> {
        if self.monitors.is_empty() {
            return column![
                text("No monitors. Try Refresh.").size(12),
                button(text("Refresh").size(11))
                    .padding([4, 12])
                    .on_press(Message::RefreshMonitors)
                    .style(secondary_button_style),
            ]
            .spacing(6)
            .into();
        }
        let preview = canvas(MonitorPreview {
            monitors: &self.monitors,
            selected: self.selected_monitor,
            on_select: Box::new(Message::MonitorSelectedByIndex),
        })
        .width(Length::Fill)
        .height(Length::Fixed(170.0));

        let idx = self.selected_monitor.min(self.monitors.len() - 1);
        let m = &self.monitors[idx];
        let primary = if m.is_primary { " · primary" } else { "" };
        let info = format!(
            "#{}  {}×{} at ({},{}){}",
            idx + 1,
            m.bounds.width(),
            m.bounds.height(),
            m.bounds.left,
            m.bounds.top,
            primary
        );

        column![
            preview,
            text(info).size(12),
            button(
                text("Refresh")
                    .size(13)
                    .color(ColorToken::Ink.color())
                    .width(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Center)
            )
            .padding([8, 16])
            .width(Length::Fill)
            .on_press(Message::RefreshMonitors)
            .style(secondary_button_style),
        ]
        .spacing(8)
        .into()
    }

    fn custom_panel(&self) -> Element<'_, Message> {
        let field = |label: &'static str, value: &str, f: Field| {
            row![
                text(label).size(12),
                Space::new().width(Length::Fill),
                text_input("0", value)
                    .on_input(move |s| Message::FieldChanged(f, s))
                    .width(Length::Fixed(140.0))
                    .size(13),
                text("px").size(12),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center)
        };
        column![
            field("Left", &self.custom_left, Field::Left),
            field("Top", &self.custom_top, Field::Top),
            field("Width", &self.custom_width, Field::Width),
            field("Height", &self.custom_height, Field::Height),
            button(
                text("Draw on screen")
                    .size(13)
                    .color(ColorToken::Ink.color())
                    .width(Length::Fill)
                    .align_x(iced::alignment::Horizontal::Center)
            )
            .padding([8, 16])
            .width(Length::Fill)
            .on_press(Message::StartDrawRect)
            .style(secondary_button_style),
        ]
        .spacing(6)
        .into()
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

#[allow(dead_code)]
fn virtual_screen(monitors: &[MonitorInfo]) -> Option<ScreenRect> {
    if monitors.is_empty() {
        return None;
    }
    let mut left = i32::MAX;
    let mut top = i32::MAX;
    let mut right = i32::MIN;
    let mut bottom = i32::MIN;
    for m in monitors {
        left = left.min(m.bounds.left);
        top = top.min(m.bounds.top);
        right = right.max(m.bounds.right);
        bottom = bottom.max(m.bounds.bottom);
    }
    Some(ScreenRect::new(left, top, right, bottom))
}

fn section_label(label: &'static str) -> Element<'static, Message> {
    text(label).size(10).color(ColorToken::Muted.color()).into()
}

fn section_panel<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content)
        .padding(11)
        .width(Length::Fill)
        .style(panel_style)
        .into()
}

fn status_description(status: &str) -> Element<'_, Message> {
    container(
        column![
            text("STATUS").size(10).color(ColorToken::Muted.color()),
            text(status).size(12).color(ColorToken::Ink.color()),
        ]
        .spacing(3),
    )
    .padding([8, 10])
    .width(Length::Fill)
    .style(status_style)
    .into()
}

#[derive(Debug, Clone, Copy)]
enum ColorToken {
    Ink,
    Muted,
    Border,
    Panel,
    Root,
    Blue,
    Coral,
    Sun,
    Mint,
    MintText,
}

impl ColorToken {
    fn color(self) -> iced::Color {
        match self {
            Self::Ink => iced::Color::from_rgb(0.90, 0.92, 0.98),
            Self::Muted => iced::Color::from_rgb(0.62, 0.66, 0.78),
            Self::Border => iced::Color::from_rgba(0.56, 0.60, 0.78, 0.38),
            Self::Panel => iced::Color::from_rgba(0.13, 0.15, 0.23, 0.82),
            Self::Root => iced::Color::from_rgba(0.08, 0.09, 0.14, 0.97),
            Self::Blue => iced::Color::from_rgb(0.43, 0.59, 0.96),
            Self::Coral => iced::Color::from_rgb(0.96, 0.36, 0.36),
            Self::Sun => iced::Color::from_rgb(0.96, 0.70, 0.25),
            Self::Mint => iced::Color::from_rgba(0.24, 0.58, 0.39, 0.24),
            Self::MintText => iced::Color::from_rgb(0.53, 0.92, 0.68),
        }
    }
}

fn root_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(ColorToken::Root.color().into()),
        border: iced::Border {
            radius: 14.0.into(),
            color: ColorToken::Border.color(),
            width: 1.0,
        },
        ..Default::default()
    }
}

fn panel_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(ColorToken::Panel.color().into()),
        border: iced::Border {
            radius: 8.0.into(),
            color: ColorToken::Border.color(),
            width: 1.0,
        },
        ..Default::default()
    }
}

fn status_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Color::from_rgba(0.10, 0.12, 0.19, 0.86).into()),
        border: iced::Border {
            radius: 8.0.into(),
            color: ColorToken::Border.color(),
            width: 1.0,
        },
        ..Default::default()
    }
}

fn active_badge_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(ColorToken::Mint.color().into()),
        border: iced::Border {
            radius: 8.0.into(),
            color: iced::Color::from_rgba(0.48, 0.88, 0.62, 0.50),
            width: 1.0,
        },
        ..Default::default()
    }
}

fn idle_badge_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Color::from_rgba(0.18, 0.20, 0.29, 0.78).into()),
        border: iced::Border {
            radius: 8.0.into(),
            color: ColorToken::Border.color(),
            width: 1.0,
        },
        ..Default::default()
    }
}

fn traffic_light_style(color: iced::Color, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => lighten(color, 0.10),
        button::Status::Pressed => darken(color, 0.08),
        _ => color,
    };
    button::Style {
        background: Some(background.into()),
        text_color: iced::Color::TRANSPARENT,
        border: iced::Border {
            radius: 8.0.into(),
            color: darken(color, 0.12),
            width: 1.0,
        },
        ..Default::default()
    }
}

fn danger_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let base = ColorToken::Coral.color();
    let background = match status {
        button::Status::Hovered => lighten(base, 0.08),
        button::Status::Pressed => darken(base, 0.07),
        _ => base,
    };
    button::Style {
        background: Some(background.into()),
        text_color: iced::Color::WHITE,
        border: iced::Border {
            radius: 8.0.into(),
            color: darken(ColorToken::Coral.color(), 0.12),
            width: 1.0,
        },
        ..Default::default()
    }
}

fn confirm_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let base = ColorToken::Blue.color();
    let background = match status {
        button::Status::Hovered => lighten(base, 0.08),
        button::Status::Pressed => darken(base, 0.07),
        button::Status::Disabled => iced::Color::from_rgba(0.30, 0.34, 0.46, 0.55),
        _ => base,
    };
    button::Style {
        background: Some(background.into()),
        text_color: iced::Color::WHITE,
        border: iced::Border {
            radius: 8.0.into(),
            color: ColorToken::Border.color(),
            width: 1.0,
        },
        ..Default::default()
    }
}

fn secondary_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let base = iced::Color::from_rgba(0.18, 0.20, 0.30, 0.82);
    let background = match status {
        button::Status::Hovered => iced::Color::from_rgba(0.24, 0.28, 0.40, 0.92),
        button::Status::Pressed => iced::Color::from_rgba(0.15, 0.17, 0.25, 0.96),
        _ => base,
    };
    button::Style {
        background: Some(background.into()),
        text_color: ColorToken::Ink.color(),
        border: iced::Border {
            radius: 8.0.into(),
            color: ColorToken::Border.color(),
            width: 1.0,
        },
        ..Default::default()
    }
}

fn lighten(color: iced::Color, amount: f32) -> iced::Color {
    iced::Color {
        r: (color.r + amount).min(1.0),
        g: (color.g + amount).min(1.0),
        b: (color.b + amount).min(1.0),
        a: color.a,
    }
}

fn darken(color: iced::Color, amount: f32) -> iced::Color {
    iced::Color {
        r: (color.r - amount).max(0.0),
        g: (color.g - amount).max(0.0),
        b: (color.b - amount).max(0.0),
        a: color.a,
    }
}
