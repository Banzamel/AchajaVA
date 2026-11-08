//! Key names the listener and the Terminal tab send to the Claude console: space, enter, esc,
//! backspace, ctrl+u, up, down, tab, or "text:<any text>" typed character by character.

pub const LEFT_CTRL_PRESSED: u32 = 0x0008;
pub const ENHANCED_KEY: u32 = 0x0100;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Key {
    pub vk: u16,
    pub scan: u16,
    pub ch: u16,
    pub ctrl: u32,
}

fn named(name: &str) -> Option<Key> {
    let k = |vk, scan, ch: char, ctrl| Some(Key { vk, scan, ch: ch as u16, ctrl });
    match name {
        "space" => k(0x20, 0x39, ' ', 0),
        "enter" => k(0x0D, 0x1C, '\r', 0),
        "esc" => k(0x1B, 0x01, '\u{1b}', 0),
        "backspace" => k(0x08, 0x0E, '\u{8}', 0),
        "ctrl+u" => k(0x55, 0x16, '\u{15}', LEFT_CTRL_PRESSED),
        "up" => k(0x26, 0x48, '\0', ENHANCED_KEY),
        "down" => k(0x28, 0x50, '\0', ENHANCED_KEY),
        "tab" => k(0x09, 0x0F, '\t', 0),
        _ => None,
    }
}

/// The key presses (each one down and up) for a list of key names; unknown names are skipped.
pub fn expand(keys: &[String]) -> Vec<Key> {
    let mut out = Vec::new();
    for key in keys {
        if let Some(text) = key.strip_prefix("text:") {
            for unit in text.encode_utf16() {
                out.push(Key { vk: 0, scan: 0, ch: unit, ctrl: 0 });
            }
        } else if let Some(k) = named(key) {
            out.push(k);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_names_and_text() {
        let keys = expand(&["ctrl+u".into(), "text:ok".into(), "nope".into()]);
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].ctrl, LEFT_CTRL_PRESSED);
        assert_eq!(keys[1].ch, 'o' as u16);
    }
}
