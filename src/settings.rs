use std::fs;
use std::path::PathBuf;

use crate::app::{CloseBehavior, Mode};

#[derive(Debug, Clone)]
pub struct Settings {
    pub mode: Mode,
    pub selected_monitor: usize,
    pub custom_left: i32,
    pub custom_top: i32,
    pub custom_width: i32,
    pub custom_height: i32,
    pub padding: i32,
    pub minimize_on_activate: bool,
    pub close_behavior: CloseBehavior,
    pub hotkey: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::Monitor,
            selected_monitor: 0,
            custom_left: 100,
            custom_top: 100,
            custom_width: 800,
            custom_height: 600,
            padding: 4,
            minimize_on_activate: true,
            close_behavior: CloseBehavior::ToTray,
            hotkey: None,
        }
    }
}

pub fn load() -> Settings {
    let Some(path) = config_path() else {
        return Settings::default();
    };
    let Ok(text) = fs::read_to_string(&path) else {
        return Settings::default();
    };
    let mut s = Settings::default();
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let key = k.trim();
        let value = v.trim();
        match key {
            "mode" => match value {
                "Window" => s.mode = Mode::Window,
                "Monitor" => s.mode = Mode::Monitor,
                "Custom" => s.mode = Mode::Custom,
                _ => {}
            },
            "selected_monitor" => {
                if let Ok(n) = value.parse() {
                    s.selected_monitor = n;
                }
            }
            "custom_left" => {
                if let Ok(n) = value.parse() {
                    s.custom_left = n;
                }
            }
            "custom_top" => {
                if let Ok(n) = value.parse() {
                    s.custom_top = n;
                }
            }
            "custom_width" => {
                if let Ok(n) = value.parse() {
                    s.custom_width = n;
                }
            }
            "custom_height" => {
                if let Ok(n) = value.parse() {
                    s.custom_height = n;
                }
            }
            "padding" => {
                if let Ok(n) = value.parse() {
                    s.padding = n;
                }
            }
            "minimize_on_activate" => {
                s.minimize_on_activate = value.eq_ignore_ascii_case("true");
            }
            "close_behavior" => match value {
                "ToTray" => s.close_behavior = CloseBehavior::ToTray,
                "Exit" => s.close_behavior = CloseBehavior::Exit,
                _ => {}
            },
            "hotkey" => {
                if !value.is_empty() {
                    s.hotkey = Some(value.to_string());
                }
            }
            _ => {}
        }
    }
    s
}

pub fn save(s: &Settings) -> Result<(), String> {
    let path = config_path().ok_or("cannot resolve %APPDATA%\\cursory path")?;
    let mode = match s.mode {
        Mode::Window => "Window",
        Mode::Monitor => "Monitor",
        Mode::Custom => "Custom",
    };
    let close = match s.close_behavior {
        CloseBehavior::ToTray => "ToTray",
        CloseBehavior::Exit => "Exit",
    };
    let hotkey = s.hotkey.as_deref().unwrap_or("");
    let body = format!(
        "mode={mode}\n\
         selected_monitor={}\n\
         custom_left={}\n\
         custom_top={}\n\
         custom_width={}\n\
         custom_height={}\n\
         padding={}\n\
         minimize_on_activate={}\n\
         close_behavior={close}\n\
         hotkey={hotkey}\n",
        s.selected_monitor,
        s.custom_left,
        s.custom_top,
        s.custom_width,
        s.custom_height,
        s.padding,
        s.minimize_on_activate,
    );
    fs::write(&path, body).map_err(|e| format!("cannot write settings: {e}"))
}

fn config_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA")?;
    let mut p = PathBuf::from(base);
    p.push("cursory");
    // Fail closed: if the directory cannot be created, a later write would fail
    // anyway, so report the path as unavailable instead of returning a dead path.
    fs::create_dir_all(&p).ok()?;
    p.push("settings.conf");
    Some(p)
}
