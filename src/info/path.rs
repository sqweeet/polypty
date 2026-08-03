use std::path::PathBuf;

/// Replace `$HOME` with `~` and keep long paths compact.
pub fn shorten_path(path: &str) -> String {
    let path = path.trim();
    if path.is_empty() {
        return String::new();
    }
    let mut compact = abbreviate_home(path);
    const MAX_WIDTH: usize = 28;
    if unicode_width::UnicodeWidthStr::width(compact.as_str()) <= MAX_WIDTH {
        return compact;
    }
    let parts: Vec<&str> = compact.split('/').filter(|part| !part.is_empty()).collect();
    if parts.len() <= 3 {
        return truncate_middle(&compact, MAX_WIDTH);
    }
    compact = format!("{}/…/{}", parts[0], parts[parts.len() - 2..].join("/"));
    if unicode_width::UnicodeWidthStr::width(compact.as_str()) <= MAX_WIDTH {
        compact
    } else {
        truncate_middle(&compact, MAX_WIDTH)
    }
}

fn abbreviate_home(path: &str) -> String {
    let Some(home) = home_dir() else {
        return path.to_string();
    };
    if path == home {
        return "~".into();
    }
    path.strip_prefix(&format!("{home}/"))
        .map(|rest| format!("~/{rest}"))
        .unwrap_or_else(|| path.to_string())
}

fn truncate_middle(value: &str, max_width: usize) -> String {
    if unicode_width::UnicodeWidthStr::width(value) <= max_width {
        return value.to_string();
    }
    if max_width <= 3 {
        return "…".into();
    }
    let keep = max_width - 1;
    let left = keep / 2;
    let right = keep - left;
    let characters: Vec<char> = value.chars().collect();
    if characters.len() <= keep {
        return value.to_string();
    }
    let mut output: String = characters.iter().take(left).collect();
    output.push('…');
    output.extend(
        characters
            .iter()
            .rev()
            .take(right)
            .collect::<Vec<_>>()
            .into_iter()
            .rev(),
    );
    output
}

fn home_dir() -> Option<String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .and_then(|path| path.to_str().map(str::to_owned))
}
