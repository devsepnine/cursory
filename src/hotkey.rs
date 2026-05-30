use std::str::FromStr;

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

pub type Captured = (Modifiers, Code);

pub struct HotkeyService {
    manager: GlobalHotKeyManager,
    current: Option<HotKey>,
    description: String,
}

impl HotkeyService {
    pub fn try_new(initial: Option<&str>) -> Option<Self> {
        let manager = GlobalHotKeyManager::new().ok()?;
        let mut svc = Self {
            manager,
            current: None,
            description: String::new(),
        };
        let (mods, code) = initial
            .and_then(|s| HotKey::from_str(s).ok())
            .map(|h| (h.mods, h.key))
            .unwrap_or((Modifiers::CONTROL | Modifiers::ALT, Code::KeyL));
        let _ = svc.rebind(mods, code);
        Some(svc)
    }

    pub fn serialize(&self) -> Option<String> {
        self.current.map(|h| h.into_string())
    }

    pub fn reset_default(&mut self) -> Result<(), String> {
        self.rebind(Modifiers::CONTROL | Modifiers::ALT, Code::KeyL)
    }

    pub fn rebind(&mut self, mods: Modifiers, code: Code) -> Result<(), String> {
        let new_hotkey = HotKey::new(Some(mods), code);
        if Some(new_hotkey) == self.current {
            return Ok(());
        }
        self.manager
            .register(new_hotkey)
            .map_err(|e| format_register_error(&e, mods, code))?;
        if let Some(old) = self.current.take() {
            let _ = self.manager.unregister(old);
        }
        self.description = describe(mods, code);
        self.current = Some(new_hotkey);
        Ok(())
    }

    pub fn describe(&self) -> &str {
        &self.description
    }

    pub fn poll_toggle(&self) -> bool {
        let target = self.current.as_ref().map(|h| h.id());
        let mut fired = false;
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if Some(event.id) == target && event.state == HotKeyState::Pressed {
                fired = true;
            }
        }
        fired
    }

    pub fn drain(&self) {
        while GlobalHotKeyEvent::receiver().try_recv().is_ok() {}
    }
}

pub fn is_cancel_key(physical: iced::keyboard::key::Physical) -> bool {
    matches!(
        physical,
        iced::keyboard::key::Physical::Code(iced::keyboard::key::Code::Escape)
    )
}

pub fn from_iced(
    physical: iced::keyboard::key::Physical,
    modifiers: iced::keyboard::Modifiers,
) -> Option<Captured> {
    let iced_code = match physical {
        iced::keyboard::key::Physical::Code(c) => c,
        iced::keyboard::key::Physical::Unidentified(_) => return None,
    };
    if is_reserved_code(iced_code) {
        return None;
    }
    // The iced `Code` Debug name (e.g. "KeyA", "Numpad0") is fed to
    // global-hotkey's parser. This is an implicit cross-crate contract; if a
    // future version of either crate diverges, parsing fails silently. The
    // test module below pins representative mappings so drift is caught at
    // build time, and we log here so a runtime miss is at least diagnosable.
    let name = format!("{iced_code:?}");
    let parsed = match HotKey::from_str(&name) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("hotkey: unsupported key code {name:?} ({e})");
            return None;
        }
    };
    Some((iced_modifiers(modifiers), parsed.key))
}

fn iced_modifiers(modifiers: iced::keyboard::Modifiers) -> Modifiers {
    let mut mods = Modifiers::empty();
    if modifiers.control() {
        mods |= Modifiers::CONTROL;
    }
    if modifiers.alt() {
        mods |= Modifiers::ALT;
    }
    if modifiers.shift() {
        mods |= Modifiers::SHIFT;
    }
    if modifiers.logo() {
        mods |= Modifiers::META;
    }
    mods
}

fn format_register_error(err: &global_hotkey::Error, mods: Modifiers, code: Code) -> String {
    use global_hotkey::Error as E;
    let combo = describe(mods, code);
    match err {
        E::AlreadyRegistered(_) => format!("{combo} is already bound to this app"),
        E::FailedToRegister(_) | E::OsError(_) => {
            format!("{combo} is unavailable — another app or the OS may be using it")
        }
        E::FailedToUnRegister(_) => format!("failed to release previous hotkey: {err}"),
        _ => format!("failed to bind {combo}: {err}"),
    }
}

fn is_reserved_code(code: iced::keyboard::key::Code) -> bool {
    use iced::keyboard::key::Code as C;
    matches!(
        code,
        C::Escape
            | C::ShiftLeft
            | C::ShiftRight
            | C::ControlLeft
            | C::ControlRight
            | C::AltLeft
            | C::AltRight
            | C::SuperLeft
            | C::SuperRight
            | C::Fn
            | C::FnLock
            | C::Hyper
            | C::Meta
    )
}

pub fn describe_captured((mods, code): Captured) -> String {
    describe(mods, code)
}

pub fn describe_modifiers(modifiers: iced::keyboard::Modifiers) -> String {
    modifier_labels(iced_modifiers(modifiers)).join("+")
}

fn modifier_labels(mods: Modifiers) -> Vec<&'static str> {
    let mut parts = Vec::new();
    if mods.contains(Modifiers::CONTROL) {
        parts.push("Ctrl");
    }
    if mods.contains(Modifiers::ALT) {
        parts.push("Alt");
    }
    if mods.contains(Modifiers::SHIFT) {
        parts.push("Shift");
    }
    if mods.contains(Modifiers::META) {
        parts.push("Win");
    }
    parts
}

fn describe(mods: Modifiers, code: Code) -> String {
    let mods_str = modifier_labels(mods).join("+");
    let key = code_label(code);
    if mods_str.is_empty() {
        key
    } else {
        format!("{mods_str}+{key}")
    }
}

fn code_label(code: Code) -> String {
    let raw = format!("{code:?}");
    if let Some(rest) = raw.strip_prefix("Key") {
        return rest.to_string();
    }
    if let Some(rest) = raw.strip_prefix("Digit") {
        return rest.to_string();
    }
    if let Some(rest) = raw.strip_prefix("Numpad") {
        return format!("Num{rest}");
    }
    raw
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::keyboard::key::{Code as IcedCode, Physical};

    fn captured(code: IcedCode) -> Option<Captured> {
        from_iced(Physical::Code(code), iced::keyboard::Modifiers::empty())
    }

    /// Pins the implicit contract between iced's `Code` Debug names and
    /// global-hotkey's `HotKey::from_str` parser. If either crate renames a
    /// variant, this fails at build time instead of silently dropping the bind.
    #[test]
    fn representative_keys_map_through_debug_names() {
        let cases = [
            (IcedCode::KeyA, Code::KeyA),
            (IcedCode::KeyL, Code::KeyL),
            (IcedCode::Digit0, Code::Digit0),
            (IcedCode::F1, Code::F1),
            (IcedCode::F12, Code::F12),
            (IcedCode::Space, Code::Space),
            (IcedCode::Enter, Code::Enter),
            (IcedCode::ArrowUp, Code::ArrowUp),
            (IcedCode::Numpad0, Code::Numpad0),
            (IcedCode::NumpadAdd, Code::NumpadAdd),
            (IcedCode::Minus, Code::Minus),
            (IcedCode::BracketLeft, Code::BracketLeft),
        ];
        for (iced_code, expected) in cases {
            let got = captured(iced_code).map(|(_, c)| c);
            assert_eq!(got, Some(expected), "mapping failed for {iced_code:?}");
        }
    }

    #[test]
    fn reserved_keys_are_rejected() {
        assert!(captured(IcedCode::Escape).is_none());
        assert!(captured(IcedCode::ControlLeft).is_none());
        assert!(captured(IcedCode::AltRight).is_none());
    }

    #[test]
    fn unidentified_physical_is_rejected() {
        let p = Physical::Unidentified(iced::keyboard::key::NativeCode::Unidentified);
        assert!(from_iced(p, iced::keyboard::Modifiers::empty()).is_none());
    }
}
