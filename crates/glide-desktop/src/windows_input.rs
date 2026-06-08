use anyhow::{anyhow, Result};
use winapi::shared::minwindef::{DWORD, WORD};
use winapi::shared::windef::POINT;
use winapi::um::winuser::{
    GetCursorPos, GetSystemMetrics, SendInput, SetCursorPos, INPUT, INPUT_KEYBOARD, INPUT_MOUSE,
    KEYBDINPUT, KEYEVENTF_KEYUP, MOUSEEVENTF_HWHEEL, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEEVENTF_WHEEL, MOUSEINPUT, SM_CXSCREEN, SM_CYSCREEN, VK_BACK, VK_CONTROL, VK_DELETE,
    VK_DOWN, VK_ESCAPE, VK_F1, VK_F10, VK_F11, VK_F12, VK_F2, VK_F3, VK_F4, VK_F5, VK_F6, VK_F7,
    VK_F8, VK_F9, VK_LEFT, VK_LWIN, VK_MENU, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_SPACE, VK_TAB,
    VK_UP,
};

use crate::input_adapter::InputBackend;

const WHEEL_DELTA: i32 = 120;

/// Windows input backend using SendInput and cursor APIs.
pub struct WindowsInputBackend;

impl WindowsInputBackend {
    pub fn new() -> Self {
        Self
    }

    fn send_key(&self, vk: WORD, key_up: bool) -> Result<()> {
        let flags = if key_up { KEYEVENTF_KEYUP } else { 0 };
        let mut input = unsafe { std::mem::zeroed::<INPUT>() };
        input.type_ = INPUT_KEYBOARD;
        unsafe {
            *input.u.ki_mut() = KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            };
        }
        send_input(&mut input)
    }

    fn send_mouse(&self, flags: DWORD, data: DWORD) -> Result<()> {
        let mut input = unsafe { std::mem::zeroed::<INPUT>() };
        input.type_ = INPUT_MOUSE;
        unsafe {
            *input.u.mi_mut() = MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            };
        }
        send_input(&mut input)
    }

    fn set_cursor(&self, x: i32, y: i32) -> Result<()> {
        let ok = unsafe { SetCursorPos(x, y) };
        if ok == 0 {
            return Err(anyhow!("failed to set Windows cursor position"));
        }
        Ok(())
    }
}

impl Default for WindowsInputBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl InputBackend for WindowsInputBackend {
    async fn inject_key(&self, key_code: &str, pressed: bool, modifiers: &[String]) -> Result<()> {
        let vk = virtual_key(key_code)
            .ok_or_else(|| anyhow!("unsupported Windows key code: {}", key_code))?;
        let modifier_keys = modifier_virtual_keys(modifiers);

        if pressed {
            for modifier in &modifier_keys {
                self.send_key(*modifier, false)?;
            }
            self.send_key(vk, false)?;
            if !modifier_keys.is_empty() {
                self.send_key(vk, true)?;
                for modifier in modifier_keys.iter().rev() {
                    self.send_key(*modifier, true)?;
                }
            }
        } else {
            self.send_key(vk, true)?;
            for modifier in modifier_keys.iter().rev() {
                self.send_key(*modifier, true)?;
            }
        }

        Ok(())
    }

    async fn inject_mouse_button(&self, button: &str, pressed: bool, x: i32, y: i32) -> Result<()> {
        self.set_cursor(x, y)?;
        let flags = match (button, pressed) {
            ("left", true) => MOUSEEVENTF_LEFTDOWN,
            ("left", false) => MOUSEEVENTF_LEFTUP,
            ("right", true) => MOUSEEVENTF_RIGHTDOWN,
            ("right", false) => MOUSEEVENTF_RIGHTUP,
            ("middle", true) => MOUSEEVENTF_MIDDLEDOWN,
            ("middle", false) => MOUSEEVENTF_MIDDLEUP,
            _ => return Err(anyhow!("unsupported Windows mouse button: {}", button)),
        };
        self.send_mouse(flags, 0)
    }

    async fn inject_mouse_move(
        &self,
        x: i32,
        y: i32,
        _dx: Option<i32>,
        _dy: Option<i32>,
    ) -> Result<()> {
        self.set_cursor(x, y)
    }

    async fn inject_mouse_scroll(&self, dx: i32, dy: i32) -> Result<()> {
        if dy != 0 {
            let delta = signed_wheel_delta(dy);
            self.send_mouse(MOUSEEVENTF_WHEEL, delta)?;
        }
        if dx != 0 {
            let delta = signed_wheel_delta(dx);
            self.send_mouse(MOUSEEVENTF_HWHEEL, delta)?;
        }
        Ok(())
    }

    async fn cursor_position(&self) -> Result<(i32, i32)> {
        let mut point = POINT { x: 0, y: 0 };
        let ok = unsafe { GetCursorPos(&mut point) };
        if ok == 0 {
            return Err(anyhow!("failed to read Windows cursor position"));
        }
        Ok((point.x, point.y))
    }

    async fn screen_size(&self) -> Result<(i32, i32)> {
        let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        if width <= 0 || height <= 0 {
            return Err(anyhow!("failed to read Windows screen size"));
        }
        Ok((width, height))
    }
}

fn send_input(input: &mut INPUT) -> Result<()> {
    let sent = unsafe { SendInput(1, input, std::mem::size_of::<INPUT>() as i32) };
    if sent == 0 {
        return Err(anyhow!("Windows SendInput failed"));
    }
    Ok(())
}

fn signed_wheel_delta(amount: i32) -> DWORD {
    let clamped = amount.clamp(-10, 10);
    (clamped * WHEEL_DELTA) as DWORD
}

fn modifier_virtual_keys(modifiers: &[String]) -> Vec<WORD> {
    modifiers
        .iter()
        .filter_map(|modifier| modifier_virtual_key(modifier))
        .collect()
}

fn modifier_virtual_key(key: &str) -> Option<WORD> {
    match key {
        "Ctrl" | "Control" | "Ctrl_L" | "Ctrl_R" => Some(VK_CONTROL as WORD),
        "Alt" | "Alt_L" | "Alt_R" => Some(VK_MENU as WORD),
        "Shift" | "Shift_L" | "Shift_R" => Some(VK_SHIFT as WORD),
        "Super" | "Win" | "Meta" | "Super_L" | "Super_R" => Some(VK_LWIN as WORD),
        _ => None,
    }
}

fn virtual_key(key_code: &str) -> Option<WORD> {
    if let Some(modifier) = modifier_virtual_key(key_code) {
        return Some(modifier);
    }
    let upper = key_code.to_ascii_uppercase();
    if upper.len() == 1 {
        let byte = upper.as_bytes()[0];
        if byte.is_ascii_alphanumeric() {
            return Some(byte as WORD);
        }
    }
    match upper.as_str() {
        "ENTER" | "RETURN" => Some(VK_RETURN as WORD),
        "ESC" | "ESCAPE" => Some(VK_ESCAPE as WORD),
        "BACKSPACE" | "BACK_SPACE" => Some(VK_BACK as WORD),
        "DELETE" | "DEL" => Some(VK_DELETE as WORD),
        "TAB" => Some(VK_TAB as WORD),
        "SPACE" => Some(VK_SPACE as WORD),
        "UP" => Some(VK_UP as WORD),
        "DOWN" => Some(VK_DOWN as WORD),
        "LEFT" => Some(VK_LEFT as WORD),
        "RIGHT" => Some(VK_RIGHT as WORD),
        "F1" => Some(VK_F1 as WORD),
        "F2" => Some(VK_F2 as WORD),
        "F3" => Some(VK_F3 as WORD),
        "F4" => Some(VK_F4 as WORD),
        "F5" => Some(VK_F5 as WORD),
        "F6" => Some(VK_F6 as WORD),
        "F7" => Some(VK_F7 as WORD),
        "F8" => Some(VK_F8 as WORD),
        "F9" => Some(VK_F9 as WORD),
        "F10" => Some(VK_F10 as WORD),
        "F11" => Some(VK_F11 as WORD),
        "F12" => Some(VK_F12 as WORD),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_common_key_codes() {
        assert_eq!(virtual_key("A"), Some(0x41));
        assert_eq!(virtual_key("z"), Some(0x5A));
        assert_eq!(virtual_key("5"), Some(0x35));
        assert_eq!(virtual_key("Return"), Some(VK_RETURN as WORD));
        assert_eq!(virtual_key("Ctrl_L"), Some(VK_CONTROL as WORD));
        assert_eq!(virtual_key("unknown"), None);
    }

    #[test]
    fn maps_modifiers() {
        let modifiers = vec!["Ctrl".to_string(), "Shift".to_string(), "noop".to_string()];
        assert_eq!(
            modifier_virtual_keys(&modifiers),
            vec![VK_CONTROL as WORD, VK_SHIFT as WORD]
        );
    }

    #[test]
    fn wheel_delta_is_clamped_and_signed() {
        assert_eq!(signed_wheel_delta(1), WHEEL_DELTA as DWORD);
        assert_eq!(signed_wheel_delta(20), (10 * WHEEL_DELTA) as DWORD);
        assert_eq!(signed_wheel_delta(-1), (-WHEEL_DELTA) as DWORD);
    }
}
