//! Tracks the main window's tray-hide lifecycle: whether it is currently hidden
//! in the tray, and whether an async "is the window minimized?" probe is in
//! flight. A TEA sub-model — `App` issues the window Tasks and reads these flags
//! to decide when to auto-hide a window the user manually minimized.

#[derive(Default)]
pub(super) struct TrayState {
    hidden: bool,
    checking_minimized: bool,
}

impl TrayState {
    /// Whether to start an async minimized-state probe now: only when the window
    /// is neither already hidden nor a probe already pending.
    pub(super) fn should_probe_minimized(&self) -> bool {
        !self.hidden && !self.checking_minimized
    }

    pub(super) fn begin_minimize_probe(&mut self) {
        self.checking_minimized = true;
    }

    pub(super) fn end_minimize_probe(&mut self) {
        self.checking_minimized = false;
    }

    pub(super) fn mark_hidden(&mut self) {
        self.hidden = true;
    }

    pub(super) fn mark_restored(&mut self) {
        self.hidden = false;
    }
}
