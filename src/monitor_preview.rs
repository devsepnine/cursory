use iced::alignment;
use iced::mouse;
use iced::widget::canvas::{Action, Event, Frame, Geometry, Path, Program, Stroke, Text};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

use crate::confine::ScreenRect;
use crate::monitor::MonitorInfo;

pub struct MonitorPreview<'a, Message> {
    pub monitors: &'a [MonitorInfo],
    pub selected: usize,
    pub on_select: Box<dyn Fn(usize) -> Message + 'a>,
}

impl<'a, Message> Program<Message> for MonitorPreview<'a, Message> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        let bg = Path::rectangle(Point::new(0.0, 0.0), Size::new(bounds.width, bounds.height));
        frame.fill(&bg, Color::from_rgba(0.10, 0.12, 0.19, 0.78));

        if let Some(layout) = preview_layout(self.monitors, bounds) {
            for (i, m) in self.monitors.iter().enumerate() {
                let rect = layout.rect_for(m);
                let path = Path::rectangle(Point::new(rect.x, rect.y), Size::new(rect.w, rect.h));
                let is_selected = i == self.selected;
                let fill = if is_selected {
                    Color::from_rgba(0.44, 0.62, 0.98, 0.36)
                } else {
                    Color::from_rgba(0.18, 0.21, 0.32, 0.86)
                };
                let stroke_color = if is_selected {
                    Color::from_rgb(0.55, 0.70, 1.0)
                } else {
                    Color::from_rgb(0.34, 0.39, 0.56)
                };
                let stroke_w: f32 = if is_selected { 2.5 } else { 1.2 };
                frame.fill(&path, fill);
                frame.stroke(
                    &path,
                    Stroke::default()
                        .with_color(stroke_color)
                        .with_width(stroke_w),
                );

                let label = (i + 1).to_string();
                frame.fill_text(Text {
                    content: label,
                    position: Point::new(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0),
                    color: Color::from_rgb(0.90, 0.92, 0.98),
                    size: 18.0.into(),
                    align_x: iced::widget::text::Alignment::Center,
                    align_y: alignment::Vertical::Center,
                    ..Default::default()
                });

                if m.is_primary {
                    frame.fill_text(Text {
                        content: "★".into(),
                        position: Point::new(rect.x + 6.0, rect.y + 4.0),
                        color: Color::from_rgb(0.96, 0.62, 0.20),
                        size: 12.0.into(),
                        ..Default::default()
                    });
                }
            }
        } else {
            frame.fill_text(Text {
                content: "No monitors".into(),
                position: Point::new(bounds.width / 2.0, bounds.height / 2.0),
                color: Color::from_rgb(0.62, 0.66, 0.78),
                size: 13.0.into(),
                align_x: iced::widget::text::Alignment::Center,
                align_y: alignment::Vertical::Center,
                ..Default::default()
            });
        }

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        _state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            if let Some(pos) = cursor.position_in(bounds) {
                if let Some(layout) = preview_layout(self.monitors, bounds) {
                    for (i, m) in self.monitors.iter().enumerate() {
                        let r = layout.rect_for(m);
                        if pos.x >= r.x && pos.x <= r.x + r.w && pos.y >= r.y && pos.y <= r.y + r.h
                        {
                            return Some(Action::publish((self.on_select)(i)).and_capture());
                        }
                    }
                }
            }
        }
        None
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if let Some(pos) = cursor.position_in(bounds) {
            if let Some(layout) = preview_layout(self.monitors, bounds) {
                for m in self.monitors.iter() {
                    let r = layout.rect_for(m);
                    if pos.x >= r.x && pos.x <= r.x + r.w && pos.y >= r.y && pos.y <= r.y + r.h {
                        return mouse::Interaction::Pointer;
                    }
                }
            }
        }
        mouse::Interaction::default()
    }
}

struct Layout {
    bbox_left: i32,
    bbox_top: i32,
    scale: f32,
    offset_x: f32,
    offset_y: f32,
}

struct UiRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Layout {
    fn rect_for(&self, m: &MonitorInfo) -> UiRect {
        let x = (m.bounds.left - self.bbox_left) as f32 * self.scale + self.offset_x;
        let y = (m.bounds.top - self.bbox_top) as f32 * self.scale + self.offset_y;
        let w = m.bounds.width() as f32 * self.scale;
        let h = m.bounds.height() as f32 * self.scale;
        UiRect { x, y, w, h }
    }
}

fn preview_layout(monitors: &[MonitorInfo], bounds: Rectangle) -> Option<Layout> {
    if monitors.is_empty() {
        return None;
    }
    let bbox = bounding_box(monitors);
    let bbox_w = (bbox.right - bbox.left) as f32;
    let bbox_h = (bbox.bottom - bbox.top) as f32;
    if bbox_w <= 0.0 || bbox_h <= 0.0 {
        return None;
    }
    let margin = 14.0;
    let avail_w = (bounds.width - margin * 2.0).max(1.0);
    let avail_h = (bounds.height - margin * 2.0).max(1.0);
    let scale = (avail_w / bbox_w).min(avail_h / bbox_h);
    let total_w = bbox_w * scale;
    let total_h = bbox_h * scale;
    let offset_x = (bounds.width - total_w) / 2.0;
    let offset_y = (bounds.height - total_h) / 2.0;
    Some(Layout {
        bbox_left: bbox.left,
        bbox_top: bbox.top,
        scale,
        offset_x,
        offset_y,
    })
}

fn bounding_box(monitors: &[MonitorInfo]) -> ScreenRect {
    let mut left = i32::MAX;
    let mut top = i32::MAX;
    let mut right = i32::MIN;
    let mut bottom = i32::MIN;
    for m in monitors {
        left = left.min(m.bounds.left);
        top = top.min(m.bounds.top);
        right = right.max(m.bounds.right);
        bottom = bottom.max(m.bounds.bottom);
    }
    ScreenRect::new(left, top, right, bottom)
}
