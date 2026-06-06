//! Hotkey-recording message handlers (record/preview/confirm/cancel).

use super::*;

impl App {
    pub(super) fn on_start_hotkey_record(&mut self) -> Task<Message> {
        if self.hotkey.is_some() {
            self.recorder.start();
            self.set_status("press combo, then Confirm (esc to cancel)");
        }
        Task::none()
    }

    pub(super) fn on_cancel_hotkey_record(&mut self) -> Task<Message> {
        self.recorder.stop();
        self.set_status("hotkey unchanged");
        Task::none()
    }

    pub(super) fn on_confirm_hotkey_record(&mut self) -> Task<Message> {
        let Some(captured) = self.recorder.pending() else {
            return Task::none();
        };
        let Some(svc) = self.hotkey.as_mut() else {
            return Task::none();
        };
        match svc.rebind(captured.0, captured.1) {
            Ok(()) => {
                let desc = svc.describe().to_string();
                self.recorder.stop();
                self.set_status(format!("hotkey set to {desc}"));
                self.persist();
            }
            Err(e) => {
                // keep recording mode open so the user can try another combo
                self.recorder.clear();
                self.set_status(format!("{e} — try another combo"));
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
            self.recorder.stop();
            self.set_status("hotkey unchanged");
            return Task::none();
        }
        self.recorder.set_modifiers(modifiers);
        if let Some(captured) = hotkey::from_iced(physical, modifiers) {
            self.recorder.set_combo(captured);
            self.set_status(format!("preview: {} — click Confirm", hotkey::describe_captured(captured)));
        } else {
            let mods_str = hotkey::describe_modifiers(modifiers);
            if mods_str.is_empty() {
                self.set_status("press combo, then Confirm");
            } else {
                self.set_status(format!("preview: {mods_str}+_  press a key"));
            }
        }
        Task::none()
    }

    pub(super) fn on_hotkey_modifiers_changed(
        &mut self,
        modifiers: iced::keyboard::Modifiers,
    ) -> Task<Message> {
        self.recorder.set_modifiers(modifiers);
        Task::none()
    }
}
