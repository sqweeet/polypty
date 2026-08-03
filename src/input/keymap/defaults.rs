use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::input::Action;

/// Preserve the built-in map when no user override exists.
pub(super) fn default_map_key(key: KeyEvent) -> Action {
    let modifiers = key.modifiers;
    let alt = modifiers.contains(KeyModifiers::ALT);
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);
    let shift = modifiers.contains(KeyModifiers::SHIFT);
    if ctrl && alt && matches!(key.code, KeyCode::Char('q' | 'Q' | 'й' | 'Й')) {
        return Action::Quit;
    }
    if ctrl && alt {
        return match key.code {
            KeyCode::Left => Action::PaneLeft,
            KeyCode::Right => Action::PaneRight,
            KeyCode::Up => Action::PaneUp,
            KeyCode::Down => Action::PaneDown,
            _ => Action::Forward,
        };
    }
    if ctrl && shift && matches!(key.code, KeyCode::Char('v' | 'V' | 'м' | 'М')) {
        return Action::PasteClipboard;
    }
    if !alt {
        return Action::Forward;
    }
    alt_action(key.code)
}

fn alt_action(code: KeyCode) -> Action {
    match code {
        KeyCode::Char('t' | 'T' | 'е' | 'Е') => Action::NewTab,
        KeyCode::Char('w' | 'W' | 'ц' | 'Ц') => Action::CloseTab,
        KeyCode::Char('q' | 'Q' | 'й' | 'Й') => Action::Quit,
        KeyCode::Char('v' | 'V' | 'м' | 'М') => Action::SplitVertical,
        KeyCode::Char('s' | 'S' | 'ы' | 'Ы') => Action::SplitHorizontal,
        KeyCode::Char('x' | 'X' | 'ч' | 'Ч') => Action::ClosePane,
        KeyCode::Char('o' | 'O' | 'щ' | 'Щ') => Action::NextPane,
        KeyCode::Char('h' | 'H' | 'р' | 'Р') => Action::PaneLeft,
        KeyCode::Char('j' | 'J' | 'о' | 'О') => Action::PaneDown,
        KeyCode::Char('k' | 'K' | 'л' | 'Л') => Action::PaneUp,
        KeyCode::Char('l' | 'L' | 'д' | 'Д') => Action::PaneRight,
        KeyCode::Char('b' | 'B' | 'и' | 'И') => Action::ToggleSidebar,
        KeyCode::Char('=' | '+') => Action::SidebarWider,
        KeyCode::Char('-' | '_') => Action::SidebarNarrower,
        KeyCode::Char(']' | 'n' | 'N' | 'ъ' | 'Ъ' | 'т' | 'Т') => Action::NextTab,
        KeyCode::Char('[' | 'p' | 'P' | 'х' | 'Х' | 'з' | 'З') => Action::PrevTab,
        KeyCode::Char(c @ '1'..='9') => Action::Tab(c as u8 - b'0'),
        KeyCode::Right => Action::NextTab,
        KeyCode::Left => Action::PrevTab,
        _ => Action::Forward,
    }
}

pub(super) fn default_label(action: Action) -> Option<&'static str> {
    match action {
        Action::NewTab => Some("Alt+t"),
        Action::CloseTab => Some("Alt+w"),
        Action::NextTab => Some("Alt+[/]"),
        Action::PrevTab => Some("Alt+["),
        Action::SplitVertical => Some("Alt+v"),
        Action::SplitHorizontal => Some("Alt+s"),
        Action::ClosePane => Some("Alt+x"),
        Action::NextPane => Some("Alt+hjkl"),
        Action::ToggleSidebar => Some("Alt+b"),
        Action::Quit => Some("Alt+q"),
        _ => None,
    }
}
