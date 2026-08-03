use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use vt100::{MouseProtocolEncoding, MouseProtocolMode};

/// Action handled by mux itself (not forwarded to the child).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    Tab(u8), // 1-based, 1..=9
    SplitVertical,
    SplitHorizontal,
    ClosePane,
    NextPane,
    PaneLeft,
    PaneRight,
    PaneUp,
    PaneDown,
    ToggleSidebar,
    SidebarWider,
    SidebarNarrower,
    /// Paste from CLIPBOARD (Ctrl+Shift+V).
    PasteClipboard,
    /// Literal byte sequence to send to the active PTY.
    Forward,
}

/// Simple, memorable binds — no prefix chord.
///
///   Alt+t       new tab
///   Alt+w       close tab
///   Alt+] / [   next / prev
///   Alt+1..9    jump to tab
///   Alt+v / s   split right / down
///   Alt+x       close active pane
///   Alt+o       focus next pane
///   Alt+h/j/k/l focus pane by direction
///   Alt+b       toggle sidebar
///   Alt+= / +   sidebar wider
///   Alt+- / _   sidebar narrower
///   Alt+q       quit mux
///
/// Everything else is forwarded to the child PTY as a proper terminal sequence,
/// so nested TUI apps (vim, htop, agents, etc.) keep working.
pub fn map_key(key: KeyEvent) -> Action {
    let mods = key.modifiers;
    let alt = mods.contains(KeyModifiers::ALT);
    let ctrl = mods.contains(KeyModifiers::CONTROL);

    // Force-quit even if a child ate something weird. Cyrillic aliases make
    // bindings follow the same physical keys under a Russian keyboard layout.
    if ctrl && alt && matches!(key.code, KeyCode::Char('q' | 'Q' | 'й' | 'Й')) {
        return Action::Quit;
    }

    let shift = mods.contains(KeyModifiers::SHIFT);

    if ctrl && alt {
        match key.code {
            KeyCode::Left => return Action::PaneLeft,
            KeyCode::Right => return Action::PaneRight,
            KeyCode::Up => return Action::PaneUp,
            KeyCode::Down => return Action::PaneDown,
            _ => {}
        }
    }

    // Ctrl+Shift+V — paste clipboard (works even when terminal paste is blocked).
    if ctrl && shift && matches!(key.code, KeyCode::Char('v' | 'V' | 'м' | 'М')) {
        return Action::PasteClipboard;
    }
    // Ctrl+Shift+C is left to the host terminal (Shift bypasses mouse).

    if alt {
        match key.code {
            KeyCode::Char('t' | 'T' | 'е' | 'Е') => return Action::NewTab,
            KeyCode::Char('w' | 'W' | 'ц' | 'Ц') => return Action::CloseTab,
            KeyCode::Char('q' | 'Q' | 'й' | 'Й') => return Action::Quit,
            KeyCode::Char('v' | 'V' | 'м' | 'М') => return Action::SplitVertical,
            KeyCode::Char('s' | 'S' | 'ы' | 'Ы') => return Action::SplitHorizontal,
            KeyCode::Char('x' | 'X' | 'ч' | 'Ч') => return Action::ClosePane,
            KeyCode::Char('o' | 'O' | 'щ' | 'Щ') => return Action::NextPane,
            KeyCode::Char('h' | 'H' | 'р' | 'Р') => return Action::PaneLeft,
            KeyCode::Char('j' | 'J' | 'о' | 'О') => return Action::PaneDown,
            KeyCode::Char('k' | 'K' | 'л' | 'Л') => return Action::PaneUp,
            KeyCode::Char('l' | 'L' | 'д' | 'Д') => return Action::PaneRight,
            KeyCode::Char('b' | 'B' | 'и' | 'И') => return Action::ToggleSidebar,
            KeyCode::Char('=' | '+') => return Action::SidebarWider,
            KeyCode::Char('-' | '_') => return Action::SidebarNarrower,
            KeyCode::Char(']' | 'n' | 'N' | 'ъ' | 'Ъ' | 'т' | 'Т') => return Action::NextTab,
            KeyCode::Char('[' | 'p' | 'P' | 'х' | 'Х' | 'з' | 'З') => return Action::PrevTab,
            KeyCode::Char(c @ '1'..='9') => {
                return Action::Tab(c as u8 - b'0');
            }
            KeyCode::Right => return Action::NextTab,
            KeyCode::Left => return Action::PrevTab,
            _ => {}
        }
    }

    Action::Forward
}

/// Encode a key event the way a real terminal would send it to a PTY.
pub fn encode_key(key: KeyEvent, app_cursor: bool, app_keypad: bool) -> Vec<u8> {
    let mods = key.modifiers;
    let ctrl = mods.contains(KeyModifiers::CONTROL);
    let alt = mods.contains(KeyModifiers::ALT);
    let shift = mods.contains(KeyModifiers::SHIFT);
    let _ = app_keypad; // reserved for numpad SS3 forms

    // Alt + char → ESC then char (standard meta encoding).
    // Note: pure Alt binds are consumed above; this path is for Alt+other
    // that we choose to forward, and for when map_key returns Forward.
    match key.code {
        KeyCode::Char(c) => {
            let mut bytes = Vec::new();
            if alt {
                bytes.push(0x1b);
            }
            if ctrl {
                // Ctrl has defined single-byte forms only for ASCII letters and
                // symbols. Preserve every other character as valid UTF-8 instead
                // of truncating Unicode to u8 (which produced <fffffff> garbage).
                let control = match c {
                    '@' | ' ' => Some(0x00),
                    'a'..='z' => Some((c as u8) - b'a' + 1),
                    'A'..='Z' => Some((c as u8) - b'A' + 1),
                    '[' => Some(0x1b),
                    '\\' => Some(0x1c),
                    ']' => Some(0x1d),
                    '^' => Some(0x1e),
                    '_' => Some(0x1f),
                    '?' => Some(0x7f),
                    _ => None,
                };
                if let Some(control) = control {
                    bytes.push(control);
                } else {
                    push_utf8(&mut bytes, c);
                }
            } else {
                // Shift is already reflected in `c` for letters/symbols.
                push_utf8(&mut bytes, c);
            }
            bytes
        }
        KeyCode::Enter => {
            if alt {
                b"\x1b\r".to_vec()
            } else {
                b"\r".to_vec()
            }
        }
        KeyCode::Tab => {
            if shift {
                encode_backtab(ctrl, alt)
            } else if alt {
                b"\x1b\t".to_vec()
            } else {
                b"\t".to_vec()
            }
        }
        // Crossterm normally reports Shift+Tab as BackTab rather than Tab
        // with a SHIFT modifier. Accept both forms so it is never swallowed.
        KeyCode::BackTab => encode_backtab(ctrl, alt),
        KeyCode::Backspace => {
            // Most terminals send DEL (0x7f); some send BS. DEL is more common for xterm.
            if alt {
                b"\x1b\x7f".to_vec()
            } else if ctrl {
                // Ctrl+Backspace often ^H or ^W depending on term; send ^H.
                vec![0x08]
            } else {
                vec![0x7f]
            }
        }
        KeyCode::Esc => b"\x1b".to_vec(),
        KeyCode::Delete => csi_modded(b"3~", ctrl, alt, shift),
        KeyCode::Insert => csi_modded(b"2~", ctrl, alt, shift),
        KeyCode::Home => {
            if app_cursor && modifier_param(ctrl, alt, shift) == 1 {
                b"\x1bOH".to_vec()
            } else {
                csi_modded_letter(b'H', ctrl, alt, shift)
            }
        }
        KeyCode::End => {
            if app_cursor && modifier_param(ctrl, alt, shift) == 1 {
                b"\x1bOF".to_vec()
            } else {
                csi_modded_letter(b'F', ctrl, alt, shift)
            }
        }
        KeyCode::PageUp => csi_modded(b"5~", ctrl, alt, shift),
        KeyCode::PageDown => csi_modded(b"6~", ctrl, alt, shift),
        KeyCode::Up => csi_arrow(b'A', ctrl, alt, shift, app_cursor),
        KeyCode::Down => csi_arrow(b'B', ctrl, alt, shift, app_cursor),
        KeyCode::Right => csi_arrow(b'C', ctrl, alt, shift, app_cursor),
        KeyCode::Left => csi_arrow(b'D', ctrl, alt, shift, app_cursor),
        KeyCode::F(n) => encode_fn(n, ctrl, alt, shift),
        KeyCode::Null => vec![0],
        // Not forwarded.
        _ => Vec::new(),
    }
}

/// Encode a host mouse event for the child TUI, translating coordinates to
/// the child's own zero-based pane grid before this function is called.
pub fn encode_mouse(
    event: MouseEvent,
    col: u16,
    row: u16,
    mode: MouseProtocolMode,
    encoding: MouseProtocolEncoding,
) -> Vec<u8> {
    let Some((mut code, release)) = mouse_code(event.kind, mode) else {
        return Vec::new();
    };

    if event.modifiers.contains(KeyModifiers::SHIFT) {
        code += 4;
    }
    if event.modifiers.contains(KeyModifiers::ALT) {
        code += 8;
    }
    if event.modifiers.contains(KeyModifiers::CONTROL) {
        code += 16;
    }

    let x = u32::from(col) + 1;
    let y = u32::from(row) + 1;
    match encoding {
        MouseProtocolEncoding::Sgr => {
            let suffix = if release { 'm' } else { 'M' };
            format!("\x1b[<{code};{x};{y}{suffix}").into_bytes()
        }
        MouseProtocolEncoding::Default | MouseProtocolEncoding::Utf8 => {
            // Legacy X10 reports every release as button 3. SGR is the only
            // encoding that carries the released button in an `m` record.
            if release {
                code = 3 + (code & !0b11);
            }
            encode_legacy_mouse(code, x, y, encoding)
        }
    }
}

fn mouse_code(kind: MouseEventKind, mode: MouseProtocolMode) -> Option<(u32, bool)> {
    if mode == MouseProtocolMode::None {
        return None;
    }

    match kind {
        MouseEventKind::Down(button) => Some((mouse_button_code(button), false)),
        MouseEventKind::Up(button) if mode != MouseProtocolMode::Press => {
            Some((mouse_button_code(button), true))
        }
        MouseEventKind::Drag(button)
            if matches!(
                mode,
                MouseProtocolMode::ButtonMotion | MouseProtocolMode::AnyMotion
            ) =>
        {
            Some((32 + mouse_button_code(button), false))
        }
        MouseEventKind::Moved if mode == MouseProtocolMode::AnyMotion => Some((35, false)),
        MouseEventKind::ScrollUp => Some((64, false)),
        MouseEventKind::ScrollDown => Some((65, false)),
        MouseEventKind::ScrollLeft => Some((66, false)),
        MouseEventKind::ScrollRight => Some((67, false)),
        _ => None,
    }
}

fn mouse_button_code(button: MouseButton) -> u32 {
    match button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
    }
}

fn encode_legacy_mouse(code: u32, x: u32, y: u32, encoding: MouseProtocolEncoding) -> Vec<u8> {
    let values = [code + 32, x + 32, y + 32];
    let mut bytes = b"\x1b[M".to_vec();
    match encoding {
        MouseProtocolEncoding::Default => {
            for value in values {
                let Ok(byte) = u8::try_from(value) else {
                    return Vec::new();
                };
                bytes.push(byte);
            }
        }
        MouseProtocolEncoding::Utf8 => {
            for value in values {
                let Some(c) = char::from_u32(value) else {
                    return Vec::new();
                };
                push_utf8(&mut bytes, c);
            }
        }
        MouseProtocolEncoding::Sgr => unreachable!("SGR handled before legacy encoder"),
    }
    bytes
}

fn push_utf8(bytes: &mut Vec<u8>, c: char) {
    let mut buf = [0u8; 4];
    bytes.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
}

fn modifier_param(ctrl: bool, alt: bool, shift: bool) -> u8 {
    // xterm modifier encoding: 1 + (shift?1:0) + (alt?2:0) + (ctrl?4:0)
    let mut m = 1u8;
    if shift {
        m += 1;
    }
    if alt {
        m += 2;
    }
    if ctrl {
        m += 4;
    }
    m
}

fn csi_arrow(letter: u8, ctrl: bool, alt: bool, shift: bool, app_cursor: bool) -> Vec<u8> {
    let m = modifier_param(ctrl, alt, shift);
    if m == 1 {
        if app_cursor {
            // DECCKM application cursor: SS3 <letter>  (ESC O A)
            vec![0x1b, b'O', letter]
        } else {
            vec![0x1b, b'[', letter]
        }
    } else {
        // CSI 1 ; <mod> <letter>
        format!("\x1b[1;{}{}", m, letter as char).into_bytes()
    }
}

fn csi_modded_letter(letter: u8, ctrl: bool, alt: bool, shift: bool) -> Vec<u8> {
    let m = modifier_param(ctrl, alt, shift);
    if m == 1 {
        vec![0x1b, b'[', letter]
    } else {
        format!("\x1b[1;{}{}", m, letter as char).into_bytes()
    }
}

fn csi_modded(suffix: &[u8], ctrl: bool, alt: bool, shift: bool) -> Vec<u8> {
    let m = modifier_param(ctrl, alt, shift);
    let mut out = vec![0x1b, b'['];
    if m == 1 {
        out.extend_from_slice(suffix);
    } else {
        // e.g. CSI 3 ; 5 ~  for Ctrl+Delete
        // suffix is like b"3~" — insert ;mod before final char
        if let Some((last, head)) = suffix.split_last() {
            out.extend_from_slice(head);
            out.push(b';');
            out.extend_from_slice(m.to_string().as_bytes());
            out.push(*last);
        }
    }
    out
}

fn encode_backtab(ctrl: bool, alt: bool) -> Vec<u8> {
    if !ctrl && !alt {
        return b"\x1b[Z".to_vec();
    }

    // BackTab already implies Shift even when the host event does not retain
    // it in the modifier bitset.
    let m = modifier_param(ctrl, alt, true);
    format!("\x1b[1;{m}Z").into_bytes()
}

fn encode_fn(n: u8, ctrl: bool, alt: bool, shift: bool) -> Vec<u8> {
    let m = modifier_param(ctrl, alt, shift);

    // Xterm sends F1-F4 as SS3 P/Q/R/S without modifiers, and as
    // CSI 1;<modifier>P/Q/R/S when modified. CSI 1~ is Home, not F1.
    if let 1..=4 = n {
        let final_byte = b'P' + (n - 1);
        return if m == 1 {
            vec![0x1b, b'O', final_byte]
        } else {
            format!("\x1b[1;{}{}", m, final_byte as char).into_bytes()
        };
    }

    // Xterm's tilde-numbered forms for F5-F12.
    let base: &[u8] = match n {
        5 => b"15",
        6 => b"17",
        7 => b"18",
        8 => b"19",
        9 => b"20",
        10 => b"21",
        11 => b"23",
        12 => b"24",
        _ => return Vec::new(),
    };
    if m == 1 {
        let mut v = vec![0x1b, b'['];
        v.extend_from_slice(base);
        v.push(b'~');
        v
    } else {
        format!("\x1b[{};{}~", std::str::from_utf8(base).unwrap_or("1"), m).into_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn alt_t_is_new_tab() {
        let k = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::ALT);
        assert_eq!(map_key(k), Action::NewTab);
    }

    #[test]
    fn split_and_pane_bindings() {
        let alt = KeyModifiers::ALT;
        assert_eq!(
            map_key(KeyEvent::new(KeyCode::Char('v'), alt)),
            Action::SplitVertical
        );
        assert_eq!(
            map_key(KeyEvent::new(KeyCode::Char('s'), alt)),
            Action::SplitHorizontal
        );
        assert_eq!(
            map_key(KeyEvent::new(KeyCode::Char('x'), alt)),
            Action::ClosePane
        );
        assert_eq!(
            map_key(KeyEvent::new(KeyCode::Char('h'), alt)),
            Action::PaneLeft
        );
    }

    #[test]
    fn russian_layout_uses_the_same_physical_bindings() {
        let quit = KeyEvent::new(KeyCode::Char('й'), KeyModifiers::ALT);
        let new_tab = KeyEvent::new(KeyCode::Char('е'), KeyModifiers::ALT);
        let close = KeyEvent::new(KeyCode::Char('ц'), KeyModifiers::ALT);
        let split = KeyEvent::new(KeyCode::Char('м'), KeyModifiers::ALT);

        assert_eq!(map_key(quit), Action::Quit);
        assert_eq!(map_key(new_tab), Action::NewTab);
        assert_eq!(map_key(close), Action::CloseTab);
        assert_eq!(map_key(split), Action::SplitVertical);
    }

    #[test]
    fn modified_unicode_is_never_truncated() {
        let key = KeyEvent::new(
            KeyCode::Char('ж'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        );
        let mut expected = vec![0x1b];
        expected.extend_from_slice("ж".as_bytes());

        assert_eq!(encode_key(key, false, false), expected);
    }

    #[test]
    fn plain_char_forwards() {
        let k = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        assert_eq!(map_key(k), Action::Forward);
        assert_eq!(encode_key(k, false, false), b"a");
    }

    #[test]
    fn ctrl_c() {
        let k = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(encode_key(k, false, false), vec![0x03]);
    }

    #[test]
    fn app_cursor_up() {
        let k = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(encode_key(k, true, false), b"\x1bOA");
        assert_eq!(encode_key(k, false, false), b"\x1b[A");
    }

    #[test]
    fn backtab_is_forwarded_in_both_crossterm_forms() {
        let backtab = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
        let shifted_tab = KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT);
        assert_eq!(encode_key(backtab, false, false), b"\x1b[Z");
        assert_eq!(encode_key(shifted_tab, false, false), b"\x1b[Z");

        let ctrl_backtab = KeyEvent::new(
            KeyCode::BackTab,
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        assert_eq!(encode_key(ctrl_backtab, false, false), b"\x1b[1;6Z");
    }

    #[test]
    fn function_keys_use_xterm_sequences() {
        assert_eq!(
            encode_key(
                KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE),
                false,
                false
            ),
            b"\x1bOP"
        );
        assert_eq!(
            encode_key(
                KeyEvent::new(KeyCode::F(4), KeyModifiers::NONE),
                false,
                false
            ),
            b"\x1bOS"
        );
        assert_eq!(
            encode_key(
                KeyEvent::new(KeyCode::F(1), KeyModifiers::CONTROL),
                false,
                false
            ),
            b"\x1b[1;5P"
        );
        assert_eq!(
            encode_key(
                KeyEvent::new(
                    KeyCode::F(12),
                    KeyModifiers::SHIFT | KeyModifiers::ALT | KeyModifiers::CONTROL,
                ),
                false,
                false,
            ),
            b"\x1b[24;8~"
        );
    }

    #[test]
    fn sgr_mouse_uses_pane_local_coordinates() {
        let down = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 99,
            row: 99,
            modifiers: KeyModifiers::NONE,
        };
        let up = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Right),
            modifiers: KeyModifiers::CONTROL,
            ..down
        };

        assert_eq!(
            encode_mouse(
                down,
                4,
                2,
                MouseProtocolMode::PressRelease,
                MouseProtocolEncoding::Sgr,
            ),
            b"\x1b[<0;5;3M"
        );
        assert_eq!(
            encode_mouse(
                up,
                4,
                2,
                MouseProtocolMode::PressRelease,
                MouseProtocolEncoding::Sgr,
            ),
            b"\x1b[<18;5;3m"
        );
    }

    #[test]
    fn mouse_mode_filters_release_and_motion() {
        let release = MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };
        let moved = MouseEvent {
            kind: MouseEventKind::Moved,
            ..release
        };

        assert!(encode_mouse(
            release,
            0,
            0,
            MouseProtocolMode::Press,
            MouseProtocolEncoding::Sgr,
        )
        .is_empty());
        assert!(encode_mouse(
            moved,
            0,
            0,
            MouseProtocolMode::ButtonMotion,
            MouseProtocolEncoding::Sgr,
        )
        .is_empty());
        assert_eq!(
            encode_mouse(
                moved,
                0,
                0,
                MouseProtocolMode::AnyMotion,
                MouseProtocolEncoding::Sgr,
            ),
            b"\x1b[<35;1;1M"
        );
    }

    #[test]
    fn legacy_mouse_encoding_is_well_formed() {
        let down = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(
            encode_mouse(
                down,
                0,
                0,
                MouseProtocolMode::Press,
                MouseProtocolEncoding::Default,
            ),
            b"\x1b[M !!"
        );
    }
}
