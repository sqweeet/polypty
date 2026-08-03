//! Lightweight coding-agent identification and live state detection.
//!
//! Process identity is authoritative for deciding whether a pane is an agent.
//! Once identified, only the live terminal tail, OSC title, and recent
//! unattributed PTY activity are used to classify its display state. Output
//! caused by local input or resize is filtered by the tab before it gets here.

use std::path::Path;
use std::time::Duration;

pub const SCAN_INTERVAL: Duration = Duration::from_millis(120);
pub const ACTIVITY_WINDOW: Duration = Duration::from_millis(1_200);

/// What locally initiated the latest PTY reaction. The reducer treats these
/// differently: edits mean idle, resize preserves state, and a submitted
/// prompt may start work once child output confirms it was handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentInteraction {
    None,
    Editing,
    SubmitPending,
    Submitted,
    Resizing,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AgentObservation {
    pub previous: AgentState,
    pub quiet_for: Duration,
    pub interaction: AgentInteraction,
}

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
    /// Number of agent panes represented by this workspace status.
    pub panes: usize,
    /// At least one represented pane is running a different agent kind.
    pub mixed_kinds: bool,
}

impl AgentStatus {
    pub const fn single(kind: AgentKind, state: AgentState) -> Self {
        Self {
            kind,
            state,
            panes: 1,
            mixed_kinds: false,
        }
    }
}

/// Roll pane statuses up without allowing an equal-priority background pane
/// to replace the first (normally active) pane.
pub fn rollup(statuses: impl IntoIterator<Item = AgentStatus>) -> Option<AgentStatus> {
    let mut selected: Option<AgentStatus> = None;
    let mut first_kind = None;
    let mut panes = 0usize;
    let mut mixed_kinds = false;

    for next in statuses {
        panes = panes.saturating_add(next.panes.max(1));
        mixed_kinds |= next.mixed_kinds;
        if let Some(first_kind) = first_kind {
            mixed_kinds |= first_kind != next.kind;
        } else {
            first_kind = Some(next.kind);
        }

        selected = match selected {
            Some(current) if current.state.priority() >= next.state.priority() => Some(current),
            _ => Some(next),
        };
    }

    selected.map(|mut status| {
        status.panes = panes;
        status.mixed_kinds = mixed_kinds;
        status
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
    fresh_osc_title: Option<&str>,
    observation: AgentObservation,
) -> AgentState {
    let live = screen_tail(screen, 12).to_ascii_lowercase();
    let title = fresh_osc_title.unwrap_or_default().trim();
    let lower_title = title.to_ascii_lowercase();

    // Tab keeps OSC signals only for a bounded freshness window. Strong recent
    // signals are safe even while a local redraw is guarded.
    if is_action_required_title(&lower_title) {
        return AgentState::Blocked;
    }
    if title.trim_start().chars().next().is_some_and(is_spinner) {
        return AgentState::Working;
    }
    if kind == AgentKind::Claude && title.starts_with('✳') {
        return AgentState::Ready;
    }

    match observation.interaction {
        // set_size() changes wrapping before the child finishes repainting.
        // Reading that intermediate frame can expose a stale Working line.
        AgentInteraction::Resizing => return observation.previous,
        // Editing a queued follow-up must not stop a genuinely working agent;
        // editing an idle composer likewise cannot start one.
        AgentInteraction::Editing => return observation.previous,
        AgentInteraction::None | AgentInteraction::SubmitPending | AgentInteraction::Submitted => {}
    }

    // Only the six live footer rows can contain status. If a composer begins
    // anywhere in the twelve-row tail, its wrapped continuation is user text
    // and cannot become a status just because it contains marker words.
    let lines: Vec<_> = live
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let prompt_index = lines.iter().rposition(|line| is_ready_line(line));
    let recent_start = lines.len().saturating_sub(6);
    let mut saw_ready = false;
    let mut saw_working = false;
    let mut saw_blocked = false;
    for (index, line) in lines.iter().enumerate().skip(recent_start) {
        let state = if prompt_index.is_some_and(|prompt| index >= prompt) {
            Some(AgentState::Ready)
        } else {
            state_from_status_line(kind, line)
        };
        let Some(state) = state else {
            continue;
        };
        match state {
            AgentState::Ready => saw_ready = true,
            AgentState::Working => saw_working = true,
            AgentState::Blocked => saw_blocked = true,
        }
    }
    let activity_is_recent = observation.quiet_for <= ACTIVITY_WINDOW;
    // Codex can keep its composer visible while a separate status row reports
    // active work. Require a live transition/activity to distinguish it from
    // an old status exposed above a newly idle prompt.
    let screen_state = if saw_blocked {
        Some(AgentState::Blocked)
    } else if saw_working
        && (!saw_ready
            || observation.interaction == AgentInteraction::Submitted
            || (observation.previous == AgentState::Working && activity_is_recent))
    {
        Some(AgentState::Working)
    } else if saw_ready {
        Some(AgentState::Ready)
    } else {
        None
    };

    if observation.interaction == AgentInteraction::SubmitPending {
        return screen_state.unwrap_or(observation.previous);
    }
    if observation.interaction == AgentInteraction::Submitted {
        // Enter alone is not enough: AgentActivity reports Submitted only once
        // child output arrived. A still-visible composer wins (e.g. empty
        // Enter); otherwise the accepted prompt starts the working lifecycle.
        return screen_state.unwrap_or(AgentState::Working);
    }
    if let Some(state) = screen_state {
        return state;
    }

    let explicit_state_agent = matches!(
        kind,
        AgentKind::Codex | AgentKind::Claude | AgentKind::OpenCode
    );
    if activity_is_recent && (!explicit_state_agent || observation.previous == AgentState::Working)
    {
        AgentState::Working
    } else {
        AgentState::Ready
    }
}

fn is_ready_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    let anchored = trimmed.trim_start_matches(|ch: char| !ch.is_alphanumeric());
    matches!(trimmed.chars().next(), Some('›' | '❯'))
        || starts_with_any(anchored, &["ask anything", "type a message"])
}

fn state_from_status_line(kind: AgentKind, line: &str) -> Option<AgentState> {
    let trimmed = line.trim();
    let anchored = trimmed.trim_start_matches(|ch: char| !ch.is_alphanumeric());
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
    if codex_status || interrupt_footer || opencode_status {
        return Some(AgentState::Working);
    }

    None
}

fn is_action_required_title(title: &str) -> bool {
    title.trim_matches(|ch: char| !ch.is_alphanumeric()) == "action required"
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

    fn detect(
        kind: AgentKind,
        parser: &vt100::Parser,
        title: &str,
        previous: AgentState,
        quiet_for: Duration,
    ) -> AgentState {
        detect_with_interaction(
            kind,
            parser,
            title,
            previous,
            quiet_for,
            AgentInteraction::None,
        )
    }

    fn detect_with_interaction(
        kind: AgentKind,
        parser: &vt100::Parser,
        title: &str,
        previous: AgentState,
        quiet_for: Duration,
        interaction: AgentInteraction,
    ) -> AgentState {
        detect_state(
            kind,
            parser.screen(),
            (!title.is_empty()).then_some(title),
            AgentObservation {
                previous,
                quiet_for,
                interaction,
            },
        )
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
            detect(
                AgentKind::Codex,
                &blocked,
                "Action Required",
                AgentState::Ready,
                Duration::ZERO,
            ),
            AgentState::Blocked
        );
        let working = parser_with("• Working (esc to interrupt)");
        assert_eq!(
            detect(
                AgentKind::Codex,
                &working,
                "⠋ Codex",
                AgentState::Ready,
                Duration::from_secs(5),
            ),
            AgentState::Working
        );
        let ready = parser_with("────────────────\n❯ ");
        assert_eq!(
            detect(
                AgentKind::Claude,
                &ready,
                "✳ Claude",
                AgentState::Working,
                Duration::ZERO,
            ),
            AgentState::Ready
        );
    }

    #[test]
    fn structured_working_row_outweighs_a_visible_composer() {
        let working_with_composer =
            parser_with("• Working (esc to interrupt)\n› editing a follow-up");
        assert_eq!(
            detect(
                AgentKind::Codex,
                &working_with_composer,
                "",
                AgentState::Working,
                Duration::ZERO,
            ),
            AgentState::Working
        );
        assert_eq!(
            detect(
                AgentKind::Codex,
                &working_with_composer,
                "",
                AgentState::Ready,
                Duration::MAX,
            ),
            AgentState::Ready
        );

        let prompt_keywords =
            parser_with("› explain esc to interrupt, working ( and permission required");
        assert_eq!(
            detect(
                AgentKind::Codex,
                &prompt_keywords,
                "",
                AgentState::Ready,
                Duration::ZERO,
            ),
            AgentState::Ready
        );

        let mut rows = vec!["• Working (esc to interrupt)".to_string()];
        rows.extend((0..7).map(|index| format!("completed row {index}")));
        let stale = parser_with(&rows.join("\n"));
        assert_eq!(
            detect(
                AgentKind::Codex,
                &stale,
                "",
                AgentState::Working,
                Duration::MAX,
            ),
            AgentState::Ready
        );
    }

    #[test]
    fn project_title_words_do_not_become_an_action_required_signal() {
        let blank = parser_with("");
        assert_eq!(
            detect(
                AgentKind::Codex,
                &blank,
                "fix action required detector",
                AgentState::Ready,
                Duration::MAX,
            ),
            AgentState::Ready
        );
        assert_eq!(
            detect(
                AgentKind::Codex,
                &blank,
                "[!] Action Required",
                AgentState::Ready,
                Duration::MAX,
            ),
            AgentState::Blocked
        );
    }

    #[test]
    fn opencode_interrupt_footer_is_working() {
        let parser = parser_with("BUILD  ■⬝⬝  esc interrupt");
        assert_eq!(
            detect(
                AgentKind::OpenCode,
                &parser,
                "",
                AgentState::Ready,
                Duration::MAX,
            ),
            AgentState::Working
        );
    }

    #[test]
    fn live_spinner_remains_authoritative_while_composer_is_visible() {
        let parser = parser_with("› queued follow-up");
        assert_eq!(
            detect(
                AgentKind::Codex,
                &parser,
                "⠋ Codex",
                AgentState::Ready,
                Duration::MAX,
            ),
            AgentState::Working
        );
    }

    #[test]
    fn activity_falls_back_to_ready_after_a_quiet_window() {
        let blank = parser_with("");
        assert_eq!(
            detect(
                AgentKind::Gemini,
                &blank,
                "",
                AgentState::Ready,
                Duration::from_millis(200),
            ),
            AgentState::Working
        );
        assert_eq!(
            detect(
                AgentKind::Gemini,
                &blank,
                "",
                AgentState::Working,
                Duration::from_secs(2),
            ),
            AgentState::Ready
        );
    }

    #[test]
    fn generic_activity_cannot_promote_explicit_state_agents() {
        let blank = parser_with("");
        for kind in [AgentKind::Codex, AgentKind::Claude, AgentKind::OpenCode] {
            assert_eq!(
                detect(kind, &blank, "", AgentState::Ready, Duration::ZERO),
                AgentState::Ready
            );
            assert_eq!(
                detect(kind, &blank, "", AgentState::Working, Duration::ZERO),
                AgentState::Working
            );
            assert_eq!(
                detect(
                    kind,
                    &blank,
                    "",
                    AgentState::Working,
                    Duration::from_secs(2),
                ),
                AgentState::Ready
            );
        }
    }

    #[test]
    fn local_interactions_reduce_without_reading_partial_redraws() {
        let stale_working = parser_with("• Working (esc to interrupt)");
        assert_eq!(
            detect_with_interaction(
                AgentKind::Codex,
                &stale_working,
                "",
                AgentState::Ready,
                Duration::ZERO,
                AgentInteraction::Editing,
            ),
            AgentState::Ready
        );
        assert_eq!(
            detect_with_interaction(
                AgentKind::Codex,
                &stale_working,
                "",
                AgentState::Ready,
                Duration::ZERO,
                AgentInteraction::Resizing,
            ),
            AgentState::Ready
        );
        assert_eq!(
            detect_with_interaction(
                AgentKind::Codex,
                &stale_working,
                "",
                AgentState::Working,
                Duration::MAX,
                AgentInteraction::Resizing,
            ),
            AgentState::Working
        );
    }

    #[test]
    fn acknowledged_submit_promotes_only_after_the_composer_disappears() {
        let blank = parser_with("");
        assert_eq!(
            detect_with_interaction(
                AgentKind::OpenCode,
                &blank,
                "",
                AgentState::Ready,
                Duration::MAX,
                AgentInteraction::SubmitPending,
            ),
            AgentState::Ready
        );
        assert_eq!(
            detect_with_interaction(
                AgentKind::OpenCode,
                &blank,
                "",
                AgentState::Ready,
                Duration::ZERO,
                AgentInteraction::Submitted,
            ),
            AgentState::Working
        );

        let prompt = parser_with("› explain esc to interrupt and permission required");
        assert_eq!(
            detect_with_interaction(
                AgentKind::Codex,
                &prompt,
                "",
                AgentState::Ready,
                Duration::ZERO,
                AgentInteraction::Submitted,
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
            detect(
                AgentKind::Claude,
                &parser,
                "✳ Claude",
                AgentState::Blocked,
                Duration::ZERO,
            ),
            AgentState::Ready
        );
    }

    #[test]
    fn rollup_counts_panes_and_preserves_priority_and_first_tie() {
        let codex_working = AgentStatus::single(AgentKind::Codex, AgentState::Working);
        let codex_ready = AgentStatus::single(AgentKind::Codex, AgentState::Ready);
        let claude_working = AgentStatus::single(AgentKind::Claude, AgentState::Working);
        let opencode_blocked = AgentStatus::single(AgentKind::OpenCode, AgentState::Blocked);

        assert_eq!(
            rollup([codex_working, codex_ready]),
            Some(AgentStatus {
                panes: 2,
                ..codex_working
            })
        );
        assert_eq!(
            rollup([codex_working, claude_working]),
            Some(AgentStatus {
                panes: 2,
                mixed_kinds: true,
                ..codex_working
            })
        );
        assert_eq!(
            rollup([codex_working, opencode_blocked]),
            Some(AgentStatus {
                panes: 2,
                mixed_kinds: true,
                ..opencode_blocked
            })
        );
    }
}
