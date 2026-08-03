//! Lightweight coding-agent identification and live state detection.
//!
//! Process identity is authoritative for deciding whether a pane is an agent.
//! Once identified, only the live terminal tail, OSC title, and recent PTY
//! activity are used to classify its display state.

use std::path::Path;
use std::time::Duration;

pub const SCAN_INTERVAL: Duration = Duration::from_millis(120);
pub const ACTIVITY_WINDOW: Duration = Duration::from_millis(1_200);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKind {
    Codex,
    Claude,
    OpenCode,
    Gemini,
    Cursor,
    Copilot,
    Kimi,
    Amp,
    Pi,
    Devin,
    Droid,
    Kiro,
    Grok,
}

impl AgentKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::OpenCode => "opencode",
            Self::Gemini => "gemini",
            Self::Cursor => "cursor",
            Self::Copilot => "copilot",
            Self::Kimi => "kimi",
            Self::Amp => "amp",
            Self::Pi => "pi",
            Self::Devin => "devin",
            Self::Droid => "droid",
            Self::Kiro => "kiro",
            Self::Grok => "grok",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Ready,
    Working,
    Blocked,
}

impl AgentState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Working => "working",
            Self::Blocked => "blocked",
        }
    }

    pub fn priority(self) -> u8 {
        match self {
            Self::Ready => 1,
            Self::Working => 2,
            Self::Blocked => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentStatus {
    pub kind: AgentKind,
    pub state: AgentState,
}

/// Roll pane statuses up without allowing an equal-priority background pane
/// to replace the first (normally active) pane.
pub fn rollup(statuses: impl IntoIterator<Item = AgentStatus>) -> Option<AgentStatus> {
    statuses
        .into_iter()
        .fold(None, |current, next| match current {
            Some(current) if current.state.priority() >= next.state.priority() => Some(current),
            _ => Some(next),
        })
}

pub fn identify_name(value: &str) -> Option<AgentKind> {
    let name = normalized_basename(value);
    match name.as_str() {
        "codex" => Some(AgentKind::Codex),
        "claude" | "claude-code" => Some(AgentKind::Claude),
        "opencode" | "opencode2" | "open-code" => Some(AgentKind::OpenCode),
        "gemini" | "gemini-cli" => Some(AgentKind::Gemini),
        "cursor" | "cursor-agent" => Some(AgentKind::Cursor),
        "copilot" | "ghcs" | "github-copilot" => Some(AgentKind::Copilot),
        "kimi" | "kimi-code" => Some(AgentKind::Kimi),
        "amp" | "amp-local" => Some(AgentKind::Amp),
        "pi" => Some(AgentKind::Pi),
        "devin" | "devin-cli" => Some(AgentKind::Devin),
        "droid" => Some(AgentKind::Droid),
        "kiro" | "kiro-cli" => Some(AgentKind::Kiro),
        "grok" | "grok-build" => Some(AgentKind::Grok),
        _ => None,
    }
}

/// Identify direct binaries and common JS/Python package wrappers without
/// treating arbitrary `-c`/`-e` source text as a process identity.
pub fn identify_process(comm: &str, argv: &[String]) -> Option<AgentKind> {
    if let Some(kind) = identify_name(comm) {
        return Some(kind);
    }
    if let Some(kind) = argv.first().and_then(|arg| identify_name(arg)) {
        return Some(kind);
    }

    let runtime = normalized_basename(comm);
    let is_runtime = matches!(
        runtime.as_str(),
        "node" | "nodejs" | "bun" | "deno" | "python" | "python3" | "uv"
    );
    if !is_runtime {
        return None;
    }

    if argv
        .get(1)
        .is_some_and(|arg| matches!(arg.as_str(), "-c" | "-e" | "--eval"))
    {
        return None;
    }

    // Only inspect the structural script/module slot. Scanning every runtime
    // argument would turn `node app.js --prompt /tmp/codex` into a false hit.
    let mut args = argv.iter().skip(1);
    while let Some(arg) = args.next() {
        if matches!(arg.as_str(), "-m" | "--module") {
            return args.next().and_then(|module| {
                identify_name(module).or_else(|| identify_package_path(module))
            });
        }
        if arg.starts_with('-') {
            continue;
        }
        return identify_name(arg).or_else(|| identify_package_path(arg));
    }
    None
}

fn normalized_basename(value: &str) -> String {
    let normalized = value.trim().replace('\\', "/");
    let basename = Path::new(&normalized)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&normalized)
        .to_ascii_lowercase();
    let mut name = basename
        .strip_suffix(".exe")
        .or_else(|| basename.strip_suffix(".cmd"))
        .or_else(|| basename.strip_suffix(".ps1"))
        .unwrap_or(&basename)
        .trim_start_matches('.')
        .to_string();
    for suffix in ["-wrapped", "_wrapped"] {
        if let Some(stripped) = name.strip_suffix(suffix) {
            name = stripped.to_string();
        }
    }
    name
}

fn identify_package_path(value: &str) -> Option<AgentKind> {
    let path = value.to_ascii_lowercase().replace('\\', "/");
    let markers = [
        (AgentKind::Codex, ["/@openai/codex/", "/codex/bin/"]),
        (
            AgentKind::Claude,
            ["/@anthropic-ai/claude-code/", "/claude-code/cli"],
        ),
        (AgentKind::OpenCode, ["/opencode-ai/", "/opencode/bin/"]),
        (AgentKind::Gemini, ["/@google/gemini-cli/", "/gemini-cli/"]),
        (
            AgentKind::Copilot,
            ["/@github/copilot/", "/github-copilot/"],
        ),
    ];
    markers.into_iter().find_map(|(kind, needles)| {
        needles
            .iter()
            .any(|needle| path.contains(needle))
            .then_some(kind)
    })
}

pub fn detect_state(
    kind: AgentKind,
    screen: &vt100::Screen,
    osc_title: &str,
    quiet_for: Duration,
) -> AgentState {
    let live = screen_tail(screen, 12).to_ascii_lowercase();
    let title = osc_title.trim();
    let lower_title = title.to_ascii_lowercase();

    if contains_any(
        &live,
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
    ) || lower_title.contains("action required")
    {
        return AgentState::Blocked;
    }

    let recent_lines = live
        .lines()
        .rev()
        .filter(|line| !line.trim().is_empty())
        .take(4)
        .collect::<Vec<_>>()
        .join("\n");
    if contains_any(
        &recent_lines,
        &[
            "esc to interrupt",
            "ctrl+c to interrupt",
            "press esc to interrupt",
            "working (",
        ],
    ) || title.trim_start().chars().next().is_some_and(is_spinner)
    {
        return AgentState::Working;
    }

    let visible_prompt = live
        .lines()
        .rev()
        .take(4)
        .any(|line| matches!(line.trim_start().chars().next(), Some('›' | '❯')));
    let title_reports_idle = match kind {
        AgentKind::Codex => !title.is_empty(),
        AgentKind::Claude => title.starts_with('✳'),
        _ => false,
    };
    if visible_prompt
        || title_reports_idle
        || contains_any(&live, &["ask anything", "type a message"])
    {
        return AgentState::Ready;
    }

    if quiet_for <= ACTIVITY_WINDOW {
        AgentState::Working
    } else {
        AgentState::Ready
    }
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

fn is_spinner(ch: char) -> bool {
    ('\u{2801}'..='\u{28ff}').contains(&ch)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parser_with(text: &str) -> vt100::Parser {
        let mut parser = vt100::Parser::new(24, 100, 0);
        parser.process(text.replace('\n', "\r\n").as_bytes());
        parser
    }

    fn argv(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn identifies_direct_and_wrapped_agent_processes() {
        assert_eq!(identify_name(".codex-wrapped"), Some(AgentKind::Codex));
        assert_eq!(identify_name("opencode2.exe"), Some(AgentKind::OpenCode));
        assert_eq!(
            identify_process(
                "node",
                &argv(&["node", "/opt/node_modules/@anthropic-ai/claude-code/cli.js"])
            ),
            Some(AgentKind::Claude)
        );
        assert_eq!(
            identify_process("python3", &argv(&["python3", "/tmp/codex"])),
            Some(AgentKind::Codex)
        );
    }

    #[test]
    fn ignores_agent_words_inside_eval_payloads() {
        assert_eq!(
            identify_process(
                "node",
                &argv(&["node", "-e", "setTimeout(() => {}, 1000)", "/tmp/codex"])
            ),
            None
        );
        assert_eq!(
            identify_process("bash", &argv(&["bash", "-c", "run codex later"])),
            None
        );
        assert_eq!(
            identify_process("node", &argv(&["node", "app.js", "--prompt", "/tmp/codex"])),
            None
        );
    }

    #[test]
    fn visible_agent_signals_override_recent_activity() {
        let blocked = parser_with("Allow command?\nPress Enter to confirm");
        assert_eq!(
            detect_state(
                AgentKind::Codex,
                blocked.screen(),
                "Action Required",
                Duration::ZERO,
            ),
            AgentState::Blocked
        );
        let working = parser_with("• Working (esc to interrupt)");
        assert_eq!(
            detect_state(
                AgentKind::Codex,
                working.screen(),
                "⠋ Codex",
                Duration::from_secs(5),
            ),
            AgentState::Working
        );
        let ready = parser_with("────────────────\n❯ ");
        assert_eq!(
            detect_state(
                AgentKind::Claude,
                ready.screen(),
                "✳ Claude",
                Duration::ZERO,
            ),
            AgentState::Ready
        );
    }

    #[test]
    fn activity_falls_back_to_ready_after_a_quiet_window() {
        let blank = parser_with("");
        assert_eq!(
            detect_state(
                AgentKind::Gemini,
                blank.screen(),
                "",
                Duration::from_millis(200)
            ),
            AgentState::Working
        );
        assert_eq!(
            detect_state(
                AgentKind::Gemini,
                blank.screen(),
                "",
                Duration::from_secs(2)
            ),
            AgentState::Ready
        );
    }

    #[test]
    fn old_blocker_above_live_tail_does_not_win() {
        let mut rows = vec!["Allow command?".to_string()];
        rows.extend((0..13).map(|index| format!("row {index}")));
        rows.push("❯ ".to_string());
        let parser = parser_with(&rows.join("\n"));
        assert_eq!(
            detect_state(
                AgentKind::Claude,
                parser.screen(),
                "✳ Claude",
                Duration::ZERO
            ),
            AgentState::Ready
        );
    }

    #[test]
    fn rollup_prefers_urgent_state_and_preserves_first_tie() {
        let codex_working = AgentStatus {
            kind: AgentKind::Codex,
            state: AgentState::Working,
        };
        let claude_working = AgentStatus {
            kind: AgentKind::Claude,
            state: AgentState::Working,
        };
        let opencode_blocked = AgentStatus {
            kind: AgentKind::OpenCode,
            state: AgentState::Blocked,
        };

        assert_eq!(rollup([codex_working, claude_working]), Some(codex_working));
        assert_eq!(
            rollup([codex_working, opencode_blocked]),
            Some(opencode_blocked)
        );
    }
}
