//! Self-contained state for an in-progress hotkey-recording session.
//!
//! A TEA sub-model: it owns the recording flag and the previewed combo and
//! exposes the transitions over them. Side effects — registering the combo with
//! the OS, persisting, and status text — stay in `App`, which drives this
//! sub-model. Keeping the three recording fields together (rather than loose on
//! `App`) means only this module and the hotkey handlers touch them.

use crate::hotkey::Captured;

#[derive(Default)]
pub(super) struct HotkeyRecorder {
    recording: bool,
    pending: Option<Captured>,
    pending_mods: iced::keyboard::Modifiers,
}

impl HotkeyRecorder {
    pub(super) fn is_recording(&self) -> bool {
        self.recording
    }

    pub(super) fn pending(&self) -> Option<Captured> {
        self.pending
    }

    pub(super) fn pending_mods(&self) -> iced::keyboard::Modifiers {
        self.pending_mods
    }

    /// Begin a recording session, discarding any prior preview.
    pub(super) fn start(&mut self) {
        self.recording = true;
        self.clear();
    }

    /// End the session (confirmed or cancelled) and drop the preview.
    pub(super) fn stop(&mut self) {
        self.recording = false;
        self.clear();
    }

    /// Drop the previewed combo but stay in recording mode (e.g. after a failed
    /// rebind, so the user can try another combo).
    pub(super) fn clear(&mut self) {
        self.pending = None;
        self.pending_mods = iced::keyboard::Modifiers::empty();
    }

    /// Track a modifiers change. Called on every key/modifier event so the
    /// preview reflects currently-held modifiers.
    pub(super) fn set_modifiers(&mut self, modifiers: iced::keyboard::Modifiers) {
        self.pending_mods = modifiers;
    }

    /// Record a fully-resolved combo as the pending preview. A modifier-only or
    /// unmappable press leaves any prior pending combo intact.
    pub(super) fn set_combo(&mut self, captured: Captured) {
        self.pending = Some(captured);
    }
}
