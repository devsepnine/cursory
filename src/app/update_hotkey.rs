//! Hotkey-recording message handlers (record/preview/confirm/cancel).

use super::*;

impl App {
    /// Clears the in-progress hotkey preview (combo + tracked modifiers).
    fn clear_pending_hotkey(&mut self) {
        self.pending_hotkey = None;
        self.pending_hotkey_mods = iced::keyboard::Modifiers::empty();
    }

    pub(super) fn on_start_hotkey_record(&mut self) -> Task<Message> {
        if self.hotkey.is_some() {
            self.recording_hotkey = true;
            self.clear_pending_hotkey();
            self.status = "press combo, then Confirm (esc to cancel)".into();
        }
        Task::none()
    }

    pub(super) fn on_cancel_hotkey_record(&mut self) -> Task<Message> {
        self.recording_hotkey = false;
        self.clear_pending_hotkey();
        self.status = "hotkey unchanged".into();
        Task::none()
    }

    pub(super) fn on_confirm_hotkey_record(&mut self) -> Task<Message> {
        let Some(captured) = self.pending_hotkey else {
            return Task::none();
        };
        let Some(svc) = self.hotkey.as_mut() else {
            return Task::none();
        };
        match svc.rebind(captured.0, captured.1) {
            Ok(()) => {
                let desc = svc.describe().to_string();
                self.recording_hotkey = false;
                self.clear_pending_hotkey();
                self.status = format!("hotkey set to {desc}");
                self.persist();
            }
            Err(e) => {
                // keep recording mode open so the user can try another combo
                self.clear_pending_hotkey();
                self.status = format!("{e} — try another combo");
            }
        }
        Task::none()
    }

    pub(super) fn on_key_captured(
        &mut self,
        physical: iced::keyboard::key::Physical,
        modifiers: iced::keyboard::Modifiers,
    ) -> Task<Message> {
        if hotkey::is_cancel_key(physical) {
            self.recording_hotkey = false;
            self.clear_pending_hotkey();
            self.status = "hotkey unchanged".into();
            return Task::none();
        }
        self.pending_hotkey_mods = modifiers;
        if let Some(captured) = hotkey::from_iced(physical, modifiers) {
            self.pending_hotkey = Some(captured);
            self.status = format!("preview: {} — click Confirm", hotkey::describe_captured(captured));
        } else {
            let mods_str = hotkey::describe_modifiers(modifiers);
            if mods_str.is_empty() {
                self.status = "press combo, then Confirm".into();
            } else {
                self.status = format!("preview: {mods_str}+_  press a key");
            }
        }
        Task::none()
    }

    pub(super) fn on_hotkey_modifiers_changed(
        &mut self,
        modifiers: iced::keyboard::Modifiers,
    ) -> Task<Message> {
        self.pending_hotkey_mods = modifiers;
        Task::none()
    }
}
