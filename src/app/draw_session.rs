//! State for the full-screen "draw a rectangle" overlay session.
//!
//! A TEA sub-model holding whether the overlay is active and the id of its
//! window. `App` orchestrates the actual window open/close Tasks; this only
//! tracks the session so handlers, the view, and the subscription can query it.

use iced::window;

#[derive(Default)]
pub(super) struct DrawSession {
    active: bool,
    window_id: Option<window::Id>,
}

impl DrawSession {
    pub(super) fn is_active(&self) -> bool {
        self.active
    }

    pub(super) fn window_id(&self) -> Option<window::Id> {
        self.window_id
    }

    /// Begin a session bound to the freshly-opened overlay window.
    pub(super) fn begin(&mut self, window_id: window::Id) {
        self.active = true;
        self.window_id = Some(window_id);
    }

    /// End the session, returning the overlay window id (if any) so the caller
    /// can close it.
    pub(super) fn end(&mut self) -> Option<window::Id> {
        self.active = false;
        self.window_id.take()
    }
}
