//! Target-selection panels for each mode (App window / Monitor / Custom rect).

use iced::widget::{Space, button, canvas, column, pick_list, row, text, text_input};
use iced::{Element, Length};

use super::style::*;
use super::{App, Field, Message, WindowChoice};
use crate::monitor_preview::MonitorPreview;

impl App {
    pub(super) fn window_panel(&self) -> Element<'_, Message> {
        let self_hwnd = self.window_hwnd.unwrap_or(0);
        let mut options: Vec<WindowChoice> = Vec::new();
        if self_hwnd != 0 {
            options.push(WindowChoice {
                hwnd: self_hwnd,
                label: "(this app) Cursory".into(),
            });
        }
        for w in &self.external_windows {
            if w.hwnd == self_hwnd {
                continue;
            }
            options.push(WindowChoice {
                hwnd: w.hwnd,
                label: w.title.clone(),
            });
        }
        let selected_hwnd = self.selected_window_hwnd.unwrap_or(self_hwnd);
        let current = options.iter().find(|c| c.hwnd == selected_hwnd).cloned();
        column![
            pick_list(options, current, Message::WindowTargetSelected).width(Length::Fill),
            wide_secondary_button("Refresh", Message::RefreshWindows),
        ]
        .spacing(8)
        .into()
    }

    pub(super) fn monitor_panel(&self) -> Element<'_, Message> {
        if self.monitors.is_empty() {
            return column![
                text("No monitors. Try Refresh.").size(12),
                button(text("Refresh").size(11))
                    .padding([4, 12])
                    .on_press(Message::RefreshMonitors)
                    .style(secondary_button_style),
            ]
            .spacing(6)
            .into();
        }
        let preview = canvas(MonitorPreview {
            monitors: &self.monitors,
            selected: self.selected_monitor,
            on_select: Box::new(Message::MonitorSelectedByIndex),
        })
        .width(Length::Fill)
        .height(Length::Fixed(170.0));

        let idx = self.selected_monitor.min(self.monitors.len() - 1);
        let m = &self.monitors[idx];
        let primary = if m.is_primary { " · primary" } else { "" };
        let info = format!(
            "#{}  {}×{} at ({},{}){}",
            idx + 1,
            m.bounds.width(),
            m.bounds.height(),
            m.bounds.left,
            m.bounds.top,
            primary
        );

        column![
            preview,
            text(info).size(12),
            wide_secondary_button("Refresh", Message::RefreshMonitors),
        ]
        .spacing(8)
        .into()
    }

    pub(super) fn custom_panel(&self) -> Element<'_, Message> {
        let field = |label: &'static str, value: &str, f: Field| {
            row![
                text(label).size(12),
                Space::new().width(Length::Fill),
                text_input("0", value)
                    .on_input(move |s| Message::FieldChanged(f, s))
                    .width(Length::Fixed(140.0))
                    .size(13),
                text("px").size(12),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center)
        };
        column![
            field("Left", self.custom.get(Field::Left), Field::Left),
            field("Top", self.custom.get(Field::Top), Field::Top),
            field("Width", self.custom.get(Field::Width), Field::Width),
            field("Height", self.custom.get(Field::Height), Field::Height),
            wide_secondary_button("Draw on screen", Message::StartDrawRect),
        ]
        .spacing(6)
        .into()
    }
}

/// A full-width secondary button used as the action row of each target panel.
fn wide_secondary_button(label: &'static str, on_press: Message) -> Element<'static, Message> {
    button(
        text(label)
            .size(13)
            .color(ColorToken::Ink.color())
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .padding([8, 16])
    .width(Length::Fill)
    .on_press(on_press)
    .style(secondary_button_style)
    .into()
}
