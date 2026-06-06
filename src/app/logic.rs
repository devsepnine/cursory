//! Non-view state transitions and Win32-backed helpers for `App`.

use super::*;

impl App {
    pub(super) fn toggle(&mut self) -> Task<Message> {
        if self.is_active {
            self.deactivate()
        } else {
            self.activate()
        }
    }

    fn deactivate(&mut self) -> Task<Message> {
        self.is_active = false;
        self.controller.deactivate();
        self.set_status("released");
        self.set_tray_active(false);
        if self.minimize_on_activate {
            if let Some(id) = self.window_id {
                return Task::batch([
                    window::minimize::<Message>(id, false),
                    self.window_icon_task(IconState::Idle),
                ]);
            }
        }
        self.window_icon_task(IconState::Idle)
    }

    fn activate(&mut self) -> Task<Message> {
        let (mode, padding) = match (self.build_cage_mode(), self.parse_padding()) {
            (Ok(m), Ok(p)) => (m, p),
            (Err(e), _) | (_, Err(e)) => {
                self.set_status(format!("cannot activate: {e}"));
                return Task::none();
            }
        };
        if let Err(e) = self.controller.activate(mode, padding) {
            self.set_status(format!("activate error: {e}"));
            return Task::none();
        }
        self.is_active = true;
        self.set_tray_active(true);
        self.set_status(match mode {
            CageMode::Window { .. } => "active — app window".to_string(),
            CageMode::Fixed(r) => {
                format!("active — {}×{} at ({},{})", r.width(), r.height(), r.left, r.top)
            }
        });
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

    pub(super) fn persist(&mut self) {
        let settings = Settings {
            mode: self.mode,
            selected_monitor: self.selected_monitor,
            custom_left: self.custom_left.trim().parse().unwrap_or(100),
            custom_top: self.custom_top.trim().parse().unwrap_or(100),
            custom_width: self.custom_width.trim().parse().unwrap_or(800),
            custom_height: self.custom_height.trim().parse().unwrap_or(600),
            padding: self.padding.trim().parse().unwrap_or(0),
            minimize_on_activate: self.minimize_on_activate,
            start_in_tray: self.start_in_tray,
            close_behavior: self.close_behavior,
            hotkey: self.hotkey.as_ref().and_then(|h| h.serialize()),
        };
        if let Err(e) = settings::save(&settings) {
            self.set_status(e);
        }
        self.reset_pending = false;
    }

    pub(super) fn apply_defaults(&mut self) {
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
        self.start_in_tray = d.start_in_tray;
        self.close_behavior = d.close_behavior;
        if let Some(svc) = self.hotkey.as_mut() {
            let _ = svc.reset_default();
        }
        self.reset_pending = false;
    }

    pub(super) fn poll_monitor_changes(&mut self) {
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
        self.set_status(format!("display changed ({} monitor(s))", self.monitors.len()));
        // `refresh_controller_mode` self-guards on `is_active` and rebuilds from
        // the current mode, so it is safe (and a no-op for unaffected modes) to
        // call unconditionally — the caller need not know which mode cares.
        self.refresh_controller_mode();
    }

    pub(super) fn refresh_controller_mode(&mut self) {
        if !self.is_active {
            return;
        }
        match self.build_cage_mode() {
            Ok(mode) => {
                if let Err(e) = self.controller.set_mode(mode) {
                    self.set_status(format!("update error: {e}"));
                }
            }
            Err(e) => {
                self.is_active = false;
                self.controller.deactivate();
                self.set_tray_active(false);
                self.set_status(format!("auto-released: {e}"));
            }
        }
    }

    pub(super) fn refresh_padding(&mut self) {
        if !self.is_active {
            return;
        }
        match self.parse_padding() {
            Ok(p) => {
                if let Err(e) = self.controller.set_padding(p) {
                    self.set_status(format!("padding error: {e}"));
                }
            }
            Err(e) => {
                self.set_status(format!("padding: {e}"));
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

    pub(super) fn exit_draw_mode(&mut self) -> Task<Message> {
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

    pub(super) fn hide_to_tray(&mut self) -> Task<Message> {
        self.checking_minimized = false;
        if self.tray.is_none() {
            self.set_status("tray unavailable — minimized");
            if let Some(id) = self.window_id {
                return window::minimize::<Message>(id, true);
            }
            return Task::none();
        }

        self.hidden_to_tray = true;
        self.set_status("hidden to tray — double-click tray icon to restore");
        if let Some(id) = self.window_id {
            return Task::batch([
                window::minimize::<Message>(id, false),
                window::set_mode::<Message>(id, window::Mode::Hidden),
            ]);
        }
        Task::none()
    }

    pub(super) fn restore_from_tray(&mut self) -> Task<Message> {
        self.hidden_to_tray = false;
        self.set_status("restored");
        if let Some(id) = self.window_id {
            return Task::batch([
                window::set_mode::<Message>(id, window::Mode::Windowed),
                window::minimize::<Message>(id, false),
                window::gain_focus::<Message>(id),
            ]);
        }
        Task::none()
    }

    pub(super) fn close_app(&mut self) -> Task<Message> {
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

    /// Single entry point for status-line writes. Routing every update through
    /// one method keeps mutation of the shared `status` field in one place.
    pub(super) fn set_status(&mut self, message: impl Into<String>) {
        self.status = message.into();
    }

    pub(super) fn set_tray_active(&self, active: bool) {
        if let Some(tray) = self.tray.as_ref() {
            tray.set_active(active);
        }
    }

    pub(super) fn window_icon_task(&self, state: IconState) -> Task<Message> {
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
}
