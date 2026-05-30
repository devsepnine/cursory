//! The full-screen overlay shown while the user drags/resizes a rectangle in
//! "Custom rect" draw mode. Stateless: every widget is built fresh, so these
//! are free functions returning `'static` elements.

use iced::mouse;
use iced::widget::{Space, button, column, container, mouse_area, row, stack, text};
use iced::{Element, Length, Theme, window};

use super::Message;

const EDGE: f32 = 8.0;

pub(super) fn view() -> Element<'static, Message> {
    let middle = row![edge_v(window::Direction::West), center_zone(), edge_v(window::Direction::East)];
    let edges = column![edge_h(window::Direction::North), middle, edge_h(window::Direction::South)];
    let stacked = stack![edges, corners_overlay()];
    container(stacked)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(overlay_style)
        .into()
}

fn confirm_button() -> Element<'static, Message> {
    button(text("Confirm").size(14))
        .padding([8, 22])
        .on_press(Message::ConfirmRect)
        .style(|theme: &Theme, status: button::Status| {
            let palette = theme.extended_palette();
            let (bg, tc) = match status {
                button::Status::Hovered => {
                    (palette.primary.strong.color, palette.primary.strong.text)
                }
                button::Status::Pressed => (palette.primary.weak.color, palette.primary.weak.text),
                _ => (palette.primary.base.color, palette.primary.base.text),
            };
            button::Style {
                background: Some(bg.into()),
                text_color: tc,
                border: iced::Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn cancel_button() -> Element<'static, Message> {
    button(text("×").size(16))
        .padding([2, 8])
        .on_press(Message::CancelRect)
        .style(|theme: &Theme, status: button::Status| {
            let palette = theme.extended_palette();
            let (bg, tc) = match status {
                button::Status::Hovered => (
                    iced::Background::Color(iced::Color::from_rgb(0.85, 0.25, 0.30)),
                    iced::Color::WHITE,
                ),
                _ => (
                    iced::Background::Color(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.45)),
                    palette.background.base.text,
                ),
            };
            button::Style {
                background: Some(bg),
                text_color: tc,
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn center_zone() -> Element<'static, Message> {
    mouse_area(
        container(column![
            row![Space::new().width(Length::Fill), cancel_button()].padding([4, 4]),
            Space::new().height(Length::Fill),
            container(confirm_button()).center_x(Length::Fill),
            Space::new().height(Length::Fill),
        ])
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .interaction(mouse::Interaction::Grab)
    .on_press(Message::RectWindowDrag)
    .into()
}

/// A horizontal resize bar (top/bottom edge): full width, `EDGE` tall.
fn edge_h(dir: window::Direction) -> Element<'static, Message> {
    container(resize_zone(dir, mouse::Interaction::ResizingVertically))
        .width(Length::Fill)
        .height(Length::Fixed(EDGE))
        .into()
}

/// A vertical resize bar (left/right edge): `EDGE` wide, full height.
fn edge_v(dir: window::Direction) -> Element<'static, Message> {
    container(resize_zone(dir, mouse::Interaction::ResizingHorizontally))
        .width(Length::Fixed(EDGE))
        .height(Length::Fill)
        .into()
}

fn resize_zone(dir: window::Direction, cursor: mouse::Interaction) -> Element<'static, Message> {
    mouse_area(Space::new().width(Length::Fill).height(Length::Fill))
        .interaction(cursor)
        .on_press(Message::RectWindowResize(dir))
        .into()
}

fn corner(dir: window::Direction, cursor: mouse::Interaction) -> Element<'static, Message> {
    mouse_area(
        Space::new()
            .width(Length::Fixed(EDGE * 2.0))
            .height(Length::Fixed(EDGE * 2.0)),
    )
    .interaction(cursor)
    .on_press(Message::RectWindowResize(dir))
    .into()
}

fn corners_overlay() -> Element<'static, Message> {
    column![
        row![
            container(corner(
                window::Direction::NorthWest,
                mouse::Interaction::ResizingDiagonallyDown
            ))
            .align_left(Length::Shrink),
            Space::new().width(Length::Fill),
            container(corner(
                window::Direction::NorthEast,
                mouse::Interaction::ResizingDiagonallyUp
            ))
            .align_right(Length::Shrink),
        ],
        Space::new().height(Length::Fill),
        row![
            container(corner(
                window::Direction::SouthWest,
                mouse::Interaction::ResizingDiagonallyUp
            ))
            .align_left(Length::Shrink),
            Space::new().width(Length::Fill),
            container(corner(
                window::Direction::SouthEast,
                mouse::Interaction::ResizingDiagonallyDown
            ))
            .align_right(Length::Shrink),
        ],
    ]
    .into()
}

fn overlay_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.25).into()),
        border: iced::Border {
            color: iced::Color::from_rgb(0.95, 0.25, 0.30),
            width: 2.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}
