//! The "About" window view: app name + version and clickable author links,
//! styled to match the main window's TokyoNight palette. Stateless — every
//! widget is built fresh, so these are `'static` elements.

use iced::widget::{Space, button, column, container, row, text};
use iced::{Element, Length};

use super::Message;
use super::style::*;

const GITHUB_URL: &str = "https://github.com/devsepnine";
const X_URL: &str = "https://x.com/devsepnine";

pub(super) fn view() -> Element<'static, Message> {
    let links = row![
        link_button("GitHub", Message::OpenUrl(GITHUB_URL)),
        link_button("X", Message::OpenUrl(X_URL)),
    ]
    .spacing(10);

    let body = column![
        text("Cursory").size(26).color(ColorToken::Ink.color()),
        text(concat!("v", env!("CARGO_PKG_VERSION")))
            .size(12)
            .color(ColorToken::Muted.color()),
        Space::new().height(Length::Fixed(16.0)),
        links,
        Space::new().height(Length::Fixed(18.0)),
        button(text("Close").size(12).color(ColorToken::Ink.color()))
            .padding([6, 22])
            .on_press(Message::CloseAbout)
            .style(secondary_button_style),
    ]
    .spacing(6)
    .align_x(iced::Alignment::Center);

    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(18)
        .style(root_style)
        .into()
}

fn link_button(label: &'static str, message: Message) -> Element<'static, Message> {
    button(text(label).size(13).color(iced::Color::WHITE))
        .padding([7, 20])
        .on_press(message)
        .style(confirm_button_style)
        .into()
}
