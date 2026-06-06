//! Core domain vocabulary shared between the application layer and persistence.
//!
//! `Mode` and `CloseBehavior` are configuration concepts, not UI widgets, so
//! they live here rather than in `app`. Keeping them in a leaf module lets both
//! `app` and `settings` depend on them in one direction, breaking the former
//! `app` <-> `settings` import cycle.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Window,
    Monitor,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseBehavior {
    ToTray,
    Exit,
}
