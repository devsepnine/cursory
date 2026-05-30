//! Color tokens, container/button styles, and small layout helpers shared by
//! the view and draw modules.

use iced::widget::{button, column, container, text};
use iced::{Element, Length, Theme};

use super::Message;

#[derive(Debug, Clone, Copy)]
pub(super) enum ColorToken {
    Ink,
    Muted,
    Border,
    Panel,
    Root,
    Blue,
    Coral,
    Sun,
    Mint,
    MintText,
}

impl ColorToken {
    pub(super) fn color(self) -> iced::Color {
        match self {
            Self::Ink => iced::Color::from_rgb(0.90, 0.92, 0.98),
            Self::Muted => iced::Color::from_rgb(0.62, 0.66, 0.78),
            Self::Border => iced::Color::from_rgba(0.56, 0.60, 0.78, 0.38),
            Self::Panel => iced::Color::from_rgba(0.13, 0.15, 0.23, 0.82),
            Self::Root => iced::Color::from_rgba(0.08, 0.09, 0.14, 0.97),
            Self::Blue => iced::Color::from_rgb(0.43, 0.59, 0.96),
            Self::Coral => iced::Color::from_rgb(0.96, 0.36, 0.36),
            Self::Sun => iced::Color::from_rgb(0.96, 0.70, 0.25),
            Self::Mint => iced::Color::from_rgba(0.24, 0.58, 0.39, 0.24),
            Self::MintText => iced::Color::from_rgb(0.53, 0.92, 0.68),
        }
    }
}

pub(super) fn section_label(label: &'static str) -> Element<'static, Message> {
    text(label).size(10).color(ColorToken::Muted.color()).into()
}

pub(super) fn section_panel<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content)
        .padding(11)
        .width(Length::Fill)
        .style(panel_style)
        .into()
}

pub(super) fn status_description(status: &str) -> Element<'_, Message> {
    container(
        column![
            text("STATUS").size(10).color(ColorToken::Muted.color()),
            text(status).size(12).color(ColorToken::Ink.color()),
        ]
        .spacing(3),
    )
    .padding([8, 10])
    .width(Length::Fill)
    .style(status_style)
    .into()
}

pub(super) fn root_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(ColorToken::Root.color().into()),
        border: iced::Border {
            radius: 14.0.into(),
            color: ColorToken::Border.color(),
            width: 1.0,
        },
        ..Default::default()
    }
}

pub(super) fn panel_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(ColorToken::Panel.color().into()),
        border: iced::Border {
            radius: 8.0.into(),
            color: ColorToken::Border.color(),
            width: 1.0,
        },
        ..Default::default()
    }
}

pub(super) fn status_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Color::from_rgba(0.10, 0.12, 0.19, 0.86).into()),
        border: iced::Border {
            radius: 8.0.into(),
            color: ColorToken::Border.color(),
            width: 1.0,
        },
        ..Default::default()
    }
}

pub(super) fn active_badge_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(ColorToken::Mint.color().into()),
        border: iced::Border {
            radius: 8.0.into(),
            color: iced::Color::from_rgba(0.48, 0.88, 0.62, 0.50),
            width: 1.0,
        },
        ..Default::default()
    }
}

pub(super) fn idle_badge_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Color::from_rgba(0.18, 0.20, 0.29, 0.78).into()),
        border: iced::Border {
            radius: 8.0.into(),
            color: ColorToken::Border.color(),
            width: 1.0,
        },
        ..Default::default()
    }
}

pub(super) fn traffic_light_style(color: iced::Color, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => lighten(color, 0.10),
        button::Status::Pressed => darken(color, 0.08),
        _ => color,
    };
    button::Style {
        background: Some(background.into()),
        text_color: iced::Color::TRANSPARENT,
        border: iced::Border {
            radius: 8.0.into(),
            color: darken(color, 0.12),
            width: 1.0,
        },
        ..Default::default()
    }
}

pub(super) fn danger_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let base = ColorToken::Coral.color();
    let background = match status {
        button::Status::Hovered => lighten(base, 0.08),
        button::Status::Pressed => darken(base, 0.07),
        _ => base,
    };
    button::Style {
        background: Some(background.into()),
        text_color: iced::Color::WHITE,
        border: iced::Border {
            radius: 8.0.into(),
            color: darken(ColorToken::Coral.color(), 0.12),
            width: 1.0,
        },
        ..Default::default()
    }
}

pub(super) fn confirm_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let base = ColorToken::Blue.color();
    let background = match status {
        button::Status::Hovered => lighten(base, 0.08),
        button::Status::Pressed => darken(base, 0.07),
        button::Status::Disabled => iced::Color::from_rgba(0.30, 0.34, 0.46, 0.55),
        _ => base,
    };
    button::Style {
        background: Some(background.into()),
        text_color: iced::Color::WHITE,
        border: iced::Border {
            radius: 8.0.into(),
            color: ColorToken::Border.color(),
            width: 1.0,
        },
        ..Default::default()
    }
}

pub(super) fn secondary_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let base = iced::Color::from_rgba(0.18, 0.20, 0.30, 0.82);
    let background = match status {
        button::Status::Hovered => iced::Color::from_rgba(0.24, 0.28, 0.40, 0.92),
        button::Status::Pressed => iced::Color::from_rgba(0.15, 0.17, 0.25, 0.96),
        _ => base,
    };
    button::Style {
        background: Some(background.into()),
        text_color: ColorToken::Ink.color(),
        border: iced::Border {
            radius: 8.0.into(),
            color: ColorToken::Border.color(),
            width: 1.0,
        },
        ..Default::default()
    }
}

pub(super) fn lighten(color: iced::Color, amount: f32) -> iced::Color {
    iced::Color {
        r: (color.r + amount).min(1.0),
        g: (color.g + amount).min(1.0),
        b: (color.b + amount).min(1.0),
        a: color.a,
    }
}

pub(super) fn darken(color: iced::Color, amount: f32) -> iced::Color {
    iced::Color {
        r: (color.r - amount).max(0.0),
        g: (color.g - amount).max(0.0),
        b: (color.b - amount).max(0.0),
        a: color.a,
    }
}
