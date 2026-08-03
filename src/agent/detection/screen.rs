use crate::agent::{AgentKind, AgentState};

#[derive(Debug, Default)]
pub(super) struct ScreenEvidence {
    pub ready: bool,
    pub working: bool,
    pub blocked: bool,
}

impl ScreenEvidence {
    pub fn capture(kind: AgentKind, screen: &vt100::Screen) -> Self {
        let live = screen_tail(screen, 12).to_ascii_lowercase();
        let lines: Vec<_> = live
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        let prompt_index = lines.iter().rposition(|line| is_ready_line(line));
        let recent_start = lines.len().saturating_sub(6);
        let mut evidence = Self::default();

        for (index, line) in lines.iter().enumerate().skip(recent_start) {
            let state = if prompt_index.is_some_and(|prompt| index >= prompt) {
                Some(AgentState::Ready)
            } else {
                state_from_status_line(kind, line)
            };
            match state {
                Some(AgentState::Ready) => evidence.ready = true,
                Some(AgentState::Working) => evidence.working = true,
                Some(AgentState::Blocked) => evidence.blocked = true,
                None => {}
            }
        }
        evidence
    }
}

fn is_ready_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    let anchored = trimmed.trim_start_matches(|ch: char| !ch.is_alphanumeric());
    matches!(trimmed.chars().next(), Some('›' | '❯'))
        || starts_with_any(anchored, &["ask anything", "type a message"])
}

fn state_from_status_line(kind: AgentKind, line: &str) -> Option<AgentState> {
    let anchored = line
        .trim()
        .trim_start_matches(|ch: char| !ch.is_alphanumeric());
    if starts_with_any(
        anchored,
        &[
            "allow command?",
            "permission required",
            "do you want to proceed?",
            "press enter to confirm",
            "enter to submit answer",
            "enter to submit all",
            "waiting for permission",
            "do you want to allow",
            "review your answers",
        ],
    ) {
        return Some(AgentState::Blocked);
    }

    let codex_status = anchored.starts_with("working (") && anchored.contains("interrupt");
    let interrupt_footer = starts_with_any(
        anchored,
        &[
            "esc to interrupt",
            "ctrl+c to interrupt",
            "press esc to interrupt",
        ],
    );
    let opencode_status = kind == AgentKind::OpenCode
        && anchored.starts_with("build")
        && anchored.contains("esc interrupt")
        && contains_any(anchored, &["■", "⬝", "[⋯]"]);
    (codex_status || interrupt_footer || opencode_status).then_some(AgentState::Working)
}

fn screen_tail(screen: &vt100::Screen, max_rows: usize) -> String {
    let (_, cols) = screen.size();
    let rows: Vec<String> = screen.rows(0, cols).collect();
    let cursor_row = usize::from(screen.cursor_position().0);
    let last_nonempty = rows.iter().rposition(|row| !row.trim().is_empty());
    let end = last_nonempty
        .unwrap_or(cursor_row)
        .max(cursor_row)
        .saturating_add(1)
        .min(rows.len());
    rows[end.saturating_sub(max_rows)..end].join("\n")
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn starts_with_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.starts_with(needle))
}
