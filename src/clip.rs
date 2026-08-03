//! Clipboard helpers — paste into the PTY without extra deps.

use std::process::Command;

/// Clipboard (Ctrl+C / wl-copy / xclip CLIPBOARD).
pub fn read_clipboard() -> Option<String> {
    read_cmd(&[
        (&["wl-paste", "--no-newline"], &[]),
        (&["xclip", "-selection", "clipboard", "-o"], &[]),
        (&["xsel", "--clipboard", "--output"], &[]),
    ])
}

/// Primary selection (middle-click paste on X11/Wayland).
pub fn read_primary() -> Option<String> {
    read_cmd(&[
        (&["wl-paste", "--primary", "--no-newline"], &[]),
        (&["xclip", "-selection", "primary", "-o"], &[]),
        (&["xsel", "--primary", "--output"], &[]),
    ])
    .or_else(read_clipboard)
}

fn read_cmd(cmds: &[(&[&str], &[&str])]) -> Option<String> {
    for (argv, _) in cmds {
        if argv.is_empty() {
            continue;
        }
        let mut c = Command::new(argv[0]);
        if argv.len() > 1 {
            c.args(&argv[1..]);
        }
        if let Ok(out) = c.output() {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).to_string();
                // Keep exact clipboard bytes as UTF-8 text; don't trim —
                // trailing newline is often intentional.
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }
    None
}
