//! The four text inputs backing "Custom rect" mode.
//!
//! A TEA sub-model owning the raw input strings. It centralizes parsing them
//! into a `ScreenRect` so the activate path (`build_cage_mode`) and the
//! persistence path no longer each parse the four fields by hand. `padding`
//! is intentionally not here: it applies to every mode, not just Custom.

use super::Field;
use crate::confine::ScreenRect;

pub(super) struct CustomRect {
    left: String,
    top: String,
    width: String,
    height: String,
}

impl CustomRect {
    pub(super) fn new(left: i32, top: i32, width: i32, height: i32) -> Self {
        Self {
            left: left.to_string(),
            top: top.to_string(),
            width: width.to_string(),
            height: height.to_string(),
        }
    }

    /// Current text for a rect field. `Field::Padding` is not part of the rect
    /// and yields an empty string (callers route padding elsewhere).
    pub(super) fn get(&self, field: Field) -> &str {
        match field {
            Field::Left => &self.left,
            Field::Top => &self.top,
            Field::Width => &self.width,
            Field::Height => &self.height,
            Field::Padding => "",
        }
    }

    /// Replace one rect field's text. `Field::Padding` is a no-op.
    pub(super) fn set(&mut self, field: Field, value: String) {
        match field {
            Field::Left => self.left = value,
            Field::Top => self.top = value,
            Field::Width => self.width = value,
            Field::Height => self.height = value,
            Field::Padding => {}
        }
    }

    /// Strict parse used when activating: every field must be a valid integer
    /// and the rect must have positive extent.
    pub(super) fn parse(&self) -> Result<ScreenRect, String> {
        let left: i32 = self.left.trim().parse().map_err(|_| "left not a number")?;
        let top: i32 = self.top.trim().parse().map_err(|_| "top not a number")?;
        let width: i32 = self.width.trim().parse().map_err(|_| "width not a number")?;
        let height: i32 = self
            .height
            .trim()
            .parse()
            .map_err(|_| "height not a number")?;
        if width <= 0 || height <= 0 {
            return Err("width/height must be positive".into());
        }
        Ok(ScreenRect::from_xywh(left, top, width, height))
    }

    /// Lenient per-field value for persistence: falls back to `default` while
    /// the text is mid-edit and not yet a valid integer.
    pub(super) fn value_or(&self, field: Field, default: i32) -> i32 {
        self.get(field).trim().parse().unwrap_or(default)
    }
}
