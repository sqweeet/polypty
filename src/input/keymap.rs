use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::Action;

/// Map a host key to a mux action, leaving unbound keys for the child PTY.
pub fn map_key(key: KeyEvent) -> Action {
    let modifiers = key.modifiers;
    let alt = modifiers.contains(KeyModifiers::ALT);
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);
    let shift = modifiers.contains(KeyModifiers::SHIFT);

    if ctrl && alt && matches!(key.code, KeyCode::Char('q' | 'Q' | 'й' | 'Й')) {
        return Action::Quit;
    }
    if ctrl && alt {
        match key.code {
            KeyCode::Left => return Action::PaneLeft,
            KeyCode::Right => return Action::PaneRight,
            KeyCode::Up => return Action::PaneUp,
            KeyCode::Down => return Action::PaneDown,
            _ => {}
        }
    }
    if ctrl && shift && matches!(key.code, KeyCode::Char('v' | 'V' | 'м' | 'М')) {
        return Action::PasteClipboard;
    }
    if !alt {
        return Action::Forward;
    }

    match key.code {
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
