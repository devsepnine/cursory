//! `App` view: the main settings UI (titlebar, panels, action button).

use iced::widget::{Space, button, column, container, mouse_area, radio, row, scrollable, text};
use iced::{Element, Length, Theme};

use super::style::*;
use super::{App, Message, Mode};

impl App {
    pub fn view(&self, window_id: iced::window::Id) -> Element<'_, Message> {
        if Some(window_id) == self.about_window_id {
            return super::about::view(self.about_icon.clone());
        }
        if Some(window_id) == self.draw.window_id() {
            return super::draw::view();
        }
        let titlebar = self.titlebar();
        let content = column![
            section_label("MODE"),
            section_panel(self.mode_picker()),
            Space::new().height(Length::Fixed(4.0)),
            section_label("TARGET"),
            section_panel(self.target_panel()),
            Space::new().height(Length::Fixed(4.0)),
            section_label("SETTINGS"),
            section_panel(self.settings_panel()),
        ]
        .spacing(7)
        .padding([0, 16]);

        let bottom_bar = container(
            column![
                status_description(&self.status),
                Space::new().height(Length::Fixed(8.0)),
                self.action_button(),
            ]
            .spacing(0),
        )
        .padding([0, 16])
        .width(Length::Fill);

        // Only the middle section scrolls; the titlebar and the bottom action
        // bar stay pinned so the ACTIVATE button never gets squished when the
        // content is taller than the fixed window.
        let scroll_area = scrollable(content)
            .height(Length::Fill)
            .width(Length::Fill);

        let stack = column![
            titlebar,
            scroll_area,
            Space::new().height(Length::Fixed(10.0)),
            bottom_bar,
            Space::new().height(Length::Fixed(14.0))
        ];

        container(stack)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(root_style)
            .into()
    }

    fn titlebar(&self) -> Element<'_, Message> {
        let badge: Element<'_, Message> = if self.is_active {
            container(text("ACTIVE").size(10).color(ColorToken::MintText.color()))
                .padding([3, 8])
                .style(active_badge_style)
                .into()
        } else {
            container(text("IDLE").size(10).color(ColorToken::Muted.color()))
                .padding([3, 8])
                .style(idle_badge_style)
                .into()
        };
        let drag_zone = mouse_area(
            container(
                row![
                    text("Cursory").size(16).color(ColorToken::Ink.color()),
                    Space::new().width(Length::Fixed(10.0)),
                    badge,
                    Space::new().width(Length::Fill),
                ]
                .align_y(iced::Alignment::Center),
            )
            .padding([10, 14])
            .width(Length::Fill),
        )
        .on_press(Message::DragWindow);

        let close = button(text("").size(1))
            .width(Length::Fixed(13.0))
            .height(Length::Fixed(13.0))
            .padding(0)
            .on_press(Message::CloseApp)
            .style(|_theme: &Theme, status| traffic_light_style(ColorToken::Coral.color(), status));
        let mini = button(text("").size(1))
            .width(Length::Fixed(13.0))
            .height(Length::Fixed(13.0))
            .padding(0)
            .on_press(Message::MinimizeApp)
            .style(|_theme: &Theme, status| traffic_light_style(ColorToken::Sun.color(), status));

        row![
            Space::new().width(Length::Fixed(14.0)),
            close,
            Space::new().width(Length::Fixed(7.0)),
            mini,
            Space::new().width(Length::Fixed(8.0)),
            drag_zone,
        ]
        .height(Length::Fixed(42.0))
        .align_y(iced::Alignment::Center)
        .into()
    }

    fn mode_picker(&self) -> Element<'_, Message> {
        column![
            radio(
                "App window",
                Mode::Window,
                Some(self.mode),
                Message::ModeSelected
            )
            .size(14),
            radio(
                "Monitor",
                Mode::Monitor,
                Some(self.mode),
                Message::ModeSelected
            )
            .size(14),
            radio(
                "Custom rect",
                Mode::Custom,
                Some(self.mode),
                Message::ModeSelected
            )
            .size(14),
        ]
        .spacing(4)
        .into()
    }

    fn target_panel(&self) -> Element<'_, Message> {
        match self.mode {
            Mode::Window => self.window_panel(),
            Mode::Monitor => self.monitor_panel(),
            Mode::Custom => self.custom_panel(),
        }
    }


    fn action_button(&self) -> Element<'_, Message> {
        let is_active = self.is_active;
        let label = if is_active { "RELEASE" } else { "ACTIVATE" };
        button(
            text(label)
                .size(15)
                .color(iced::Color::WHITE)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Center),
        )
        .padding([11, 0])
        .width(Length::Fill)
        .on_press(Message::ToggleActive)
        .style(move |_theme: &Theme, status: button::Status| {
            let base = if is_active {
                ColorToken::Coral.color()
            } else {
                ColorToken::Blue.color()
            };
            let bg_color = match status {
                button::Status::Hovered => lighten(base, 0.08),
                button::Status::Pressed => darken(base, 0.07),
                _ => base,
            };
            button::Style {
                background: Some(bg_color.into()),
                text_color: iced::Color::WHITE,
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
    }

}
