use super::*;
use crate::agent::{identify_name, identify_process, AgentKind};

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
