use iced::alignment;
use iced::mouse;
use iced::widget::canvas::event::{self as canvas_event, Event};
use iced::widget::canvas::{Frame, Geometry, Path, Program, Stroke, Text};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    Move,
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
}

#[derive(Debug, Clone, Copy)]
pub struct RectF {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl RectF {
    pub fn width(&self) -> f32 {
        self.right - self.left
    }
    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }
    pub fn normalize(self) -> Self {
        let mut r = self;
        if r.right < r.left {
            std::mem::swap(&mut r.left, &mut r.right);
        }
        if r.bottom < r.top {
            std::mem::swap(&mut r.top, &mut r.bottom);
        }
        r
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DrawEvent {
    Press(Point, Handle),
    Move(Point),
    Release,
}

pub struct DrawRectProgram<'a, Message> {
    pub rect: RectF,
    pub on_event: Box<dyn Fn(DrawEvent) -> Message + 'a>,
}

const HANDLE_HIT: f32 = 12.0;
const HANDLE_DRAW: f32 = 8.0;

impl<'a, Message> Program<Message> for DrawRectProgram<'a, Message> {
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

        let dim = Path::rectangle(Point::ORIGIN, bounds.size());
        frame.fill(&dim, Color::from_rgba(0.0, 0.0, 0.0, 0.45));

        let r = self.rect.normalize();
        let path = Path::rectangle(
            Point::new(r.left, r.top),
            Size::new(r.width(), r.height()),
        );
        frame.fill(&path, Color::from_rgba(0.55, 0.85, 1.0, 0.12));
        frame.stroke(
            &path,
            Stroke::default()
                .with_color(Color::from_rgb(0.6, 0.9, 1.0))
                .with_width(2.0),
        );

        for (cx, cy) in handle_centers(&r) {
            let half = HANDLE_DRAW / 2.0;
            let h = Path::rectangle(
                Point::new(cx - half, cy - half),
                Size::new(HANDLE_DRAW, HANDLE_DRAW),
            );
            frame.fill(&h, Color::from_rgb(0.95, 0.95, 1.0));
            frame.stroke(
                &h,
                Stroke::default()
                    .with_color(Color::from_rgb(0.3, 0.4, 0.55))
                    .with_width(1.0),
            );
        }

        let info = format!(
            "{:.0} × {:.0}  ({:.0}, {:.0})",
            r.width(),
            r.height(),
            r.left,
            r.top
        );
        frame.fill_text(Text {
            content: info,
            position: Point::new(r.left + 8.0, (r.top - 22.0).max(8.0)),
            color: Color::WHITE,
            size: 14.0.into(),
            ..Default::default()
        });

        frame.fill_text(Text {
            content: "Drag edges/corners to resize · Drag inside to move · Enter to confirm · ESC to cancel".into(),
            position: Point::new(bounds.width / 2.0, 28.0),
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.85),
            size: 15.0.into(),
            horizontal_alignment: alignment::Horizontal::Center,
            ..Default::default()
        });

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        _state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas_event::Status, Option<Message>) {
        let pos = match cursor.position_in(bounds) {
            Some(p) => p,
            None => return (canvas_event::Status::Ignored, None),
        };
        let r = self.rect.normalize();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(handle) = hit_test(&r, pos) {
                    return (
                        canvas_event::Status::Captured,
                        Some((self.on_event)(DrawEvent::Press(pos, handle))),
                    );
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                return (
                    canvas_event::Status::Captured,
                    Some((self.on_event)(DrawEvent::Move(pos))),
                );
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                return (
                    canvas_event::Status::Captured,
                    Some((self.on_event)(DrawEvent::Release)),
                );
            }
            _ => {}
        }
        (canvas_event::Status::Ignored, None)
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if let Some(pos) = cursor.position_in(bounds) {
            let r = self.rect.normalize();
            if let Some(h) = hit_test(&r, pos) {
                return cursor_for(h);
            }
        }
        mouse::Interaction::default()
    }
}

fn handle_centers(r: &RectF) -> [(f32, f32); 8] {
    let mx = (r.left + r.right) / 2.0;
    let my = (r.top + r.bottom) / 2.0;
    [
        (r.left, r.top),
        (mx, r.top),
        (r.right, r.top),
        (r.right, my),
        (r.right, r.bottom),
        (mx, r.bottom),
        (r.left, r.bottom),
        (r.left, my),
    ]
}

pub fn hit_test(r: &RectF, p: Point) -> Option<Handle> {
    let near = |a: f32, b: f32| (a - b).abs() < HANDLE_HIT;
    let inside_x = p.x > r.left + HANDLE_HIT && p.x < r.right - HANDLE_HIT;
    let inside_y = p.y > r.top + HANDLE_HIT && p.y < r.bottom - HANDLE_HIT;
    if near(p.x, r.left) && near(p.y, r.top) {
        return Some(Handle::NW);
    }
    if near(p.x, r.right) && near(p.y, r.top) {
        return Some(Handle::NE);
    }
    if near(p.x, r.left) && near(p.y, r.bottom) {
        return Some(Handle::SW);
    }
    if near(p.x, r.right) && near(p.y, r.bottom) {
        return Some(Handle::SE);
    }
    if near(p.y, r.top) && inside_x {
        return Some(Handle::N);
    }
    if near(p.y, r.bottom) && inside_x {
        return Some(Handle::S);
    }
    if near(p.x, r.left) && inside_y {
        return Some(Handle::W);
    }
    if near(p.x, r.right) && inside_y {
        return Some(Handle::E);
    }
    if p.x > r.left && p.x < r.right && p.y > r.top && p.y < r.bottom {
        return Some(Handle::Move);
    }
    None
}

pub fn apply_drag(handle: Handle, origin: Point, initial: RectF, current: Point) -> RectF {
    let dx = current.x - origin.x;
    let dy = current.y - origin.y;
    let mut r = initial;
    match handle {
        Handle::Move => {
            r.left += dx;
            r.right += dx;
            r.top += dy;
            r.bottom += dy;
        }
        Handle::N => r.top = initial.top + dy,
        Handle::S => r.bottom = initial.bottom + dy,
        Handle::W => r.left = initial.left + dx,
        Handle::E => r.right = initial.right + dx,
        Handle::NW => {
            r.left = initial.left + dx;
            r.top = initial.top + dy;
        }
        Handle::NE => {
            r.right = initial.right + dx;
            r.top = initial.top + dy;
        }
        Handle::SW => {
            r.left = initial.left + dx;
            r.bottom = initial.bottom + dy;
        }
        Handle::SE => {
            r.right = initial.right + dx;
            r.bottom = initial.bottom + dy;
        }
    }
    r
}

fn cursor_for(h: Handle) -> mouse::Interaction {
    match h {
        Handle::Move => mouse::Interaction::Grab,
        Handle::N | Handle::S => mouse::Interaction::ResizingVertically,
        Handle::E | Handle::W => mouse::Interaction::ResizingHorizontally,
        Handle::NE | Handle::SW => mouse::Interaction::ResizingDiagonallyUp,
        Handle::NW | Handle::SE => mouse::Interaction::ResizingDiagonallyDown,
    }
}
