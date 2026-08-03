use super::AgentKind;

pub(super) struct AgentProfile {
    pub kind: AgentKind,
    pub label: &'static str,
    aliases: &'static [&'static str],
    package_markers: &'static [&'static str],
    pub explicit_screen_state: bool,
}

pub(super) struct AgentCatalog;

const PROFILES: &[AgentProfile] = &[
    AgentProfile {
        kind: AgentKind::Codex,
        label: "codex",
        aliases: &["codex"],
        package_markers: &["/@openai/codex/", "/codex/bin/"],
        explicit_screen_state: true,
    },
    AgentProfile {
        kind: AgentKind::Claude,
        label: "claude",
        aliases: &["claude", "claude-code"],
        package_markers: &["/@anthropic-ai/claude-code/", "/claude-code/cli"],
        explicit_screen_state: true,
    },
    AgentProfile {
        kind: AgentKind::OpenCode,
        label: "opencode",
        aliases: &["opencode", "opencode2", "open-code"],
        package_markers: &["/opencode-ai/", "/opencode/bin/"],
        explicit_screen_state: true,
    },
    AgentProfile {
        kind: AgentKind::Gemini,
        label: "gemini",
        aliases: &["gemini", "gemini-cli"],
        package_markers: &["/@google/gemini-cli/", "/gemini-cli/"],
        explicit_screen_state: false,
    },
    AgentProfile {
        kind: AgentKind::Cursor,
        label: "cursor",
        aliases: &["cursor", "cursor-agent"],
        package_markers: &[],
        explicit_screen_state: false,
    },
    AgentProfile {
        kind: AgentKind::Copilot,
        label: "copilot",
        aliases: &["copilot", "ghcs", "github-copilot"],
        package_markers: &["/@github/copilot/", "/github-copilot/"],
        explicit_screen_state: false,
    },
    AgentProfile {
        kind: AgentKind::Kimi,
        label: "kimi",
        aliases: &["kimi", "kimi-code"],
        package_markers: &[],
        explicit_screen_state: false,
    },
    AgentProfile {
        kind: AgentKind::Amp,
        label: "amp",
        aliases: &["amp", "amp-local"],
        package_markers: &[],
        explicit_screen_state: false,
    },
    AgentProfile {
        kind: AgentKind::Pi,
        label: "pi",
        aliases: &["pi"],
        package_markers: &[],
        explicit_screen_state: false,
    },
    AgentProfile {
        kind: AgentKind::Devin,
        label: "devin",
        aliases: &["devin", "devin-cli"],
        package_markers: &[],
        explicit_screen_state: false,
    },
    AgentProfile {
        kind: AgentKind::Droid,
        label: "droid",
        aliases: &["droid"],
        package_markers: &[],
        explicit_screen_state: false,
    },
    AgentProfile {
        kind: AgentKind::Kiro,
        label: "kiro",
        aliases: &["kiro", "kiro-cli"],
        package_markers: &[],
        explicit_screen_state: false,
    },
    AgentProfile {
        kind: AgentKind::Grok,
        label: "grok",
        aliases: &["grok", "grok-build"],
        package_markers: &[],
        explicit_screen_state: false,
    },
];

impl AgentCatalog {
    pub fn profile(kind: AgentKind) -> &'static AgentProfile {
        PROFILES
            .iter()
            .find(|profile| profile.kind == kind)
            .expect("every AgentKind must have a profile")
    }

    pub fn identify_alias(name: &str) -> Option<AgentKind> {
        PROFILES
            .iter()
            .find(|profile| profile.aliases.contains(&name))
            .map(|profile| profile.kind)
    }

    pub fn identify_package(path: &str) -> Option<AgentKind> {
        PROFILES
            .iter()
            .find(|profile| {
                profile
                    .package_markers
                    .iter()
                    .any(|marker| path.contains(marker))
            })
            .map(|profile| profile.kind)
    }
}
