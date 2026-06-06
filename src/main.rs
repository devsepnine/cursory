#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod autostart;
mod confine;
mod domain;
mod hotkey;
mod icon;
mod monitor;
mod monitor_preview;
mod settings;
mod single_instance;
mod target;
mod tray;

use crate::app::App;

fn main() -> iced::Result {
    // Enforce a single running instance. A second launch wakes the existing one
    // (so it pops out of the tray) and exits. The guard holds the lock for the
    // whole process lifetime; run() blocks until the app closes.
    let _instance = match single_instance::SingleInstance::acquire() {
        Some(guard) => guard,
        None => {
            single_instance::signal_existing();
            return Ok(());
        }
    };

    iced::application(App::default, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        .style(|_state, theme: &iced::Theme| iced::theme::Style {
            background_color: iced::Color::TRANSPARENT,
            text_color: theme.extended_palette().background.base.text,
        })
        .window_size((460.0, 880.0))
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .exit_on_close_request(false)
        .default_font(iced::Font::with_name("Malgun Gothic"))
        .run()
}
