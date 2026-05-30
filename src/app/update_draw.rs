//! Draw-rect overlay message handlers (open/move/resize/confirm/cancel).

use super::*;

impl App {
    pub(super) fn on_start_draw_rect(&mut self) -> Task<Message> {
        if self.drawing_rect {
            return Task::none();
        }
        self.drawing_rect = true;
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
        self.rect_window_id = Some(rect_id);
        let mut tasks = vec![open_task.map(|_| Message::Noop)];
        if let Some(main_id) = self.window_id {
            tasks.push(window::set_mode::<Message>(main_id, window::Mode::Hidden));
        }
        Task::batch(tasks)
    }

    pub(super) fn on_confirm_rect(&mut self) -> Task<Message> {
        if let Some(rect_id) = self.rect_window_id {
            return window::position(rect_id).then(move |pos| {
                window::size(rect_id).map(move |size| Message::RectGeometryFetched { pos, size })
            });
        }
        Task::none()
    }

    pub(super) fn on_cancel_rect(&mut self) -> Task<Message> {
        self.status = "draw cancelled".into();
        self.exit_draw_mode()
    }

    pub(super) fn on_rect_window_drag(&mut self) -> Task<Message> {
        if let Some(id) = self.rect_window_id {
            return window::drag::<Message>(id);
        }
        Task::none()
    }

    pub(super) fn on_rect_window_resize(&mut self, dir: window::Direction) -> Task<Message> {
        if let Some(id) = self.rect_window_id {
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
            self.custom_left = left.to_string();
            self.custom_top = top.to_string();
            self.custom_width = w.to_string();
            self.custom_height = h.to_string();
            self.status = format!("rect set: {}×{} at ({},{})", w, h, left, top);
            if matches!(self.mode, Mode::Custom) {
                self.refresh_controller_mode();
            }
            self.persist();
        } else {
            self.status = "failed to read rect window position".into();
        }
        self.exit_draw_mode()
    }
}
