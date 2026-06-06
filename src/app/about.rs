//! The "About" window view: the app icon, name + version, GitHub/X icon links,
//! and a full-width Close button — styled to match the main window's TokyoNight
//! palette. Stateless; the only borrowed input is the prebuilt app-icon handle.

use iced::widget::{Space, button, column, container, image, row, svg, text};
use iced::{Element, Length};

use super::Message;
use super::style::*;

const GITHUB_URL: &str = "https://github.com/devsepnine";
const X_URL: &str = "https://x.com/devsepnine";

// Official brand marks (single-path SVGs); tinted to the Ink token at render.
const GITHUB_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82c.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8z"/></svg>"##;
const X_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24h-6.66l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z"/></svg>"##;

pub(super) fn view(icon: image::Handle) -> Element<'static, Message> {
    let logo = image(icon)
        .width(Length::Fixed(60.0))
        .height(Length::Fixed(60.0));

    let links = row![
        icon_link(GITHUB_SVG, Message::OpenUrl(GITHUB_URL)),
        icon_link(X_SVG, Message::OpenUrl(X_URL)),
    ]
    .spacing(12);

    let close = button(
        text("Close")
            .size(13)
            .color(ColorToken::Ink.color())
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .padding([9, 0])
    .width(Length::Fill)
    .on_press(Message::CloseAbout)
    .style(secondary_button_style);

    let body = column![
        logo,
        text("Cursory").size(22).color(ColorToken::Ink.color()),
        text(concat!("v", env!("CARGO_PKG_VERSION")))
            .size(11)
            .color(ColorToken::Muted.color()),
        Space::new().height(Length::Fixed(10.0)),
        links,
        Space::new().height(Length::Fixed(16.0)),
        close,
    ]
    .spacing(5)
    .width(Length::Fill)
    .align_x(iced::Alignment::Center);

    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_y(Length::Fill)
        .padding(22)
        .style(root_style)
        .into()
}

fn icon_link(bytes: &'static [u8], message: Message) -> Element<'static, Message> {
    let logo = svg(svg::Handle::from_memory(bytes))
        .width(Length::Fixed(20.0))
        .height(Length::Fixed(20.0))
        .style(|_theme, _status| svg::Style {
            color: Some(ColorToken::Ink.color()),
        });
    button(logo)
        .padding(11)
        .on_press(message)
        .style(secondary_button_style)
        .into()
}
