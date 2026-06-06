//! Draw-rect overlay message handlers (open/move/resize/confirm/cancel).

use super::*;

impl App {
    pub(super) fn on_start_draw_rect(&mut self) -> Task<Message> {
        if self.draw.is_active() {
            return Task::none();
        }
        let settings = window::Settings {
            size: iced::Size::new(800.0, 600.0),
            position: window::Position::Centered,
            decorations: false,
            resizable: true,
            transparent: true,
            level: window::Level::AlwaysOnTop,
            ..Default::default()
        };
        let (rect_id, open_task) = window::open(settings);
        self.draw.begin(rect_id);
        let mut tasks = vec![open_task.map(|_| Message::Noop)];
        if let Some(main_id) = self.window_id {
            tasks.push(window::set_mode::<Message>(main_id, window::Mode::Hidden));
        }
        Task::batch(tasks)
    }

    pub(super) fn on_confirm_rect(&mut self) -> Task<Message> {
        if let Some(rect_id) = self.draw.window_id() {
            return window::position(rect_id).then(move |pos| {
                window::size(rect_id).map(move |size| Message::RectGeometryFetched { pos, size })
            });
        }
        Task::none()
    }

    pub(super) fn on_cancel_rect(&mut self) -> Task<Message> {
        self.set_status("draw cancelled");
        self.exit_draw_mode()
    }

    pub(super) fn on_rect_window_drag(&mut self) -> Task<Message> {
        if let Some(id) = self.draw.window_id() {
            return window::drag::<Message>(id);
        }
        Task::none()
    }

    pub(super) fn on_rect_window_resize(&mut self, dir: window::Direction) -> Task<Message> {
        if let Some(id) = self.draw.window_id() {
            return window::drag_resize::<Message>(id, dir);
        }
        Task::none()
    }

    pub(super) fn on_rect_geometry_fetched(
        &mut self,
        pos: Option<iced::Point>,
        size: iced::Size,
    ) -> Task<Message> {
        if let Some(p) = pos {
            let left = p.x.round() as i32;
            let top = p.y.round() as i32;
            let w = size.width.round().max(1.0) as i32;
            let h = size.height.round().max(1.0) as i32;
            self.custom = CustomRect::new(left, top, w, h);
            self.set_status(format!("rect set: {}×{} at ({},{})", w, h, left, top));
            self.refresh_controller_mode();
            self.persist();
        } else {
            self.set_status("failed to read rect window position");
        }
        self.exit_draw_mode()
    }
}
