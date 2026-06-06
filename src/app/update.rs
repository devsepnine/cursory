//! Message dispatch. `update` is a thin router; each arm delegates to a small
//! handler so individual behaviors stay testable and within size limits.

use super::*;

impl App {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ModeSelected(mode) => self.on_mode_selected(mode),
            Message::MonitorSelectedByIndex(i) => self.on_monitor_selected(i),
            Message::WindowTargetSelected(choice) => self.on_window_target_selected(choice),
            Message::FieldChanged(field, value) => self.on_field_changed(field, value),
            Message::ToggleActive => self.toggle(),
            Message::RefreshMonitors => self.on_refresh_monitors(),
            Message::RefreshWindows => self.on_refresh_windows(),
            Message::Tick => self.on_tick(),
            Message::WindowOpened(id) => self.on_window_opened(id),
            Message::HwndCaptured(hwnd) => self.on_hwnd_captured(hwnd),
            Message::MinimizeOnActivateToggled(v) => self.on_minimize_on_activate_toggled(v),
            Message::LaunchOnStartupToggled(v) => self.on_launch_on_startup_toggled(v),
            Message::StartInTrayToggled(v) => self.on_start_in_tray_toggled(v),
            Message::CloseBehaviorSelected(behavior) => self.on_close_behavior_selected(behavior),
            Message::StartHotkeyRecord => self.on_start_hotkey_record(),
            Message::CancelHotkeyRecord => self.on_cancel_hotkey_record(),
            Message::ConfirmHotkeyRecord => self.on_confirm_hotkey_record(),
            Message::KeyCaptured(physical, modifiers) => self.on_key_captured(physical, modifiers),
            Message::HotkeyModifiersChanged(modifiers) => self.on_hotkey_modifiers_changed(modifiers),
            Message::DragWindow => self.on_drag_window(),
            Message::MinimizeApp => self.hide_to_tray(),
            Message::CloseApp => self.close_app(),
            Message::CloseRequested(id) => self.on_close_requested(id),
            Message::StartDrawRect => self.on_start_draw_rect(),
            Message::ConfirmRect => self.on_confirm_rect(),
            Message::CancelRect => self.on_cancel_rect(),
            Message::RectWindowDrag => self.on_rect_window_drag(),
            Message::RectWindowResize(dir) => self.on_rect_window_resize(dir),
            Message::RectGeometryFetched { pos, size } => self.on_rect_geometry_fetched(pos, size),
            Message::MinimizeStateChecked(is_minimized) => {
                self.on_minimize_state_checked(is_minimized)
            }
            Message::ResetSettings => self.on_reset_settings(),
            Message::Noop => Task::none(),
        }
    }

    fn on_mode_selected(&mut self, mode: Mode) -> Task<Message> {
        self.mode = mode;
        self.refresh_controller_mode();
        self.persist();
        Task::none()
    }

    fn on_monitor_selected(&mut self, i: usize) -> Task<Message> {
        if i < self.monitors.len() {
            self.selected_monitor = i;
            self.refresh_controller_mode();
            self.persist();
        }
        Task::none()
    }

    fn on_window_target_selected(&mut self, choice: WindowChoice) -> Task<Message> {
        self.selected_window_hwnd = Some(choice.hwnd);
        self.refresh_controller_mode();
        Task::none()
    }

    fn on_field_changed(&mut self, field: Field, value: String) -> Task<Message> {
        // Only persist a complete, parseable value so a mid-edit string like "-"
        // or "" never overwrites the saved geometry. Padding may be empty (== 0).
        let valid = match field {
            Field::Padding => value.trim().is_empty() || value.trim().parse::<i32>().is_ok(),
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
        } else {
            self.refresh_controller_mode();
        }
        if valid {
            self.persist();
        }
        Task::none()
    }

    fn on_refresh_monitors(&mut self) -> Task<Message> {
        self.monitors = monitor::enumerate();
        if self.selected_monitor >= self.monitors.len() {
            self.selected_monitor = 0;
        }
        self.set_status(format!("monitors refreshed ({} found)", self.monitors.len()));
        self.refresh_controller_mode();
        Task::none()
    }

    fn on_refresh_windows(&mut self) -> Task<Message> {
        self.external_windows = target::enumerate();
        self.set_status(format!("windows refreshed ({} found)", self.external_windows.len()));
        self.refresh_controller_mode();
        Task::none()
    }

    fn on_tick(&mut self) -> Task<Message> {
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
            self.set_status(format!("auto-released: {reason}"));
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
        Task::none()
    }

    fn on_window_opened(&mut self, id: window::Id) -> Task<Message> {
        self.window_id = Some(id);
        let hwnd_task =
            window::raw_id::<Message>(id).map(|raw| Message::HwndCaptured(Some(raw as isize)));
        let mut tasks = vec![hwnd_task];
        if let Some(icon) = icon::window_icon(IconState::Idle) {
            tasks.push(window::set_icon::<Message>(id, icon));
        }
        // Honor the "start in tray" option only on the initial launch. This event
        // fires once when the window first opens, so the hide never recurs.
        if self.start_in_tray {
            tasks.push(self.hide_to_tray());
        }
        Task::batch(tasks)
    }

    fn on_hwnd_captured(&mut self, hwnd: Option<isize>) -> Task<Message> {
        self.window_hwnd = hwnd;
        if hwnd.is_none() {
            self.set_status("HWND capture failed");
        } else {
            self.refresh_controller_mode();
        }
        Task::none()
    }

    fn on_minimize_on_activate_toggled(&mut self, v: bool) -> Task<Message> {
        self.minimize_on_activate = v;
        self.persist();
        Task::none()
    }

    fn on_launch_on_startup_toggled(&mut self, v: bool) -> Task<Message> {
        match autostart::set_enabled(v) {
            Ok(state) => {
                self.launch_on_startup = state;
                self.set_status(if state {
                    "launch on startup enabled"
                } else {
                    "launch on startup disabled"
                });
            }
            Err(e) => {
                self.set_status(format!("startup setting failed: {e}"));
            }
        }
        Task::none()
    }

    fn on_start_in_tray_toggled(&mut self, v: bool) -> Task<Message> {
        self.start_in_tray = v;
        self.persist();
        Task::none()
    }

    fn on_close_behavior_selected(&mut self, behavior: CloseBehavior) -> Task<Message> {
        self.close_behavior = behavior;
        self.persist();
        Task::none()
    }

    fn on_drag_window(&mut self) -> Task<Message> {
        if let Some(id) = self.window_id {
            return window::drag::<Message>(id);
        }
        Task::none()
    }

    fn on_close_requested(&mut self, id: window::Id) -> Task<Message> {
        if Some(id) == self.rect_window_id {
            self.set_status("draw cancelled");
            return self.exit_draw_mode();
        }
        if Some(id) == self.window_id {
            return self.close_app();
        }
        window::close::<Message>(id)
    }

    fn on_minimize_state_checked(&mut self, is_minimized: Option<bool>) -> Task<Message> {
        self.checking_minimized = false;
        if matches!(is_minimized, Some(true)) {
            return self.hide_to_tray();
        }
        Task::none()
    }

    fn on_reset_settings(&mut self) -> Task<Message> {
        if self.reset_pending {
            self.apply_defaults();
            self.set_status("settings reset to defaults");
            self.persist();
            if self.is_active {
                self.refresh_controller_mode();
            }
        } else {
            self.reset_pending = true;
            self.set_status("click Reset again to confirm");
        }
        Task::none()
    }
}
