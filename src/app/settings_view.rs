//! The SETTINGS panel: padding, toggles, close behavior, hotkey, reset.

use iced::widget::{Space, button, checkbox, column, radio, row, text, text_input};
use iced::{Element, Length};

use super::style::*;
use super::{App, CloseBehavior, Field, Message};
use crate::hotkey;

impl App {
    pub(super) fn settings_panel(&self) -> Element<'_, Message> {
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

        let start_in_tray = checkbox(self.start_in_tray)
            .label("Start hidden in tray")
            .on_toggle(Message::StartInTrayToggled)
            .size(15)
            .text_size(13);

        column![
            self.padding_row(),
            mini,
            startup,
            start_in_tray,
            self.close_behavior_section(),
            self.hotkey_row(),
            self.reset_row(),
        ]
        .spacing(8)
        .into()
    }

    fn padding_row(&self) -> Element<'_, Message> {
        row![
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
        .align_y(iced::Alignment::Center)
        .into()
    }

    fn hotkey_row(&self) -> Element<'_, Message> {
        let label_text = self.hotkey_label();
        let action: Element<'_, Message> = if self.recorder.is_recording() {
            let mut confirm_btn = button(text("Confirm").size(11))
                .padding([4, 10])
                .style(confirm_button_style);
            if self.recorder.pending().is_some() {
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

        row![text(label_text).width(Length::Fill).size(12), action]
            .spacing(8)
            .align_y(iced::Alignment::Center)
            .into()
    }

    fn hotkey_label(&self) -> String {
        if self.recorder.is_recording() {
            if let Some(captured) = self.recorder.pending() {
                return format!("Preview  {}", hotkey::describe_captured(captured));
            }
            let mods_str = hotkey::describe_modifiers(self.recorder.pending_mods());
            return if mods_str.is_empty() {
                "Recording  press a combo...".to_string()
            } else {
                format!("Recording  {mods_str}+_")
            };
        }
        match self.hotkey.as_ref() {
            Some(svc) => format!("Hotkey  {}", svc.describe()),
            None => "Hotkey unavailable".into(),
        }
    }

    fn close_behavior_section(&self) -> Element<'_, Message> {
        column![
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
        .spacing(4)
        .into()
    }

    fn reset_row(&self) -> Element<'_, Message> {
        let (label, style): (&str, fn(&iced::Theme, button::Status) -> button::Style) =
            if self.reset_pending {
                ("Click again to confirm reset", danger_button_style)
            } else {
                ("Reset to defaults", secondary_button_style)
            };
        let reset_btn = button(text(label).size(11))
            .padding([4, 10])
            .on_press(Message::ResetSettings)
            .style(style);
        row![Space::new().width(Length::Fill), reset_btn]
            .align_y(iced::Alignment::Center)
            .into()
    }
}
