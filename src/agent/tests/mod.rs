use std::time::Duration;

use super::{detect_state, AgentInteraction, AgentKind, AgentObservation, AgentState};

mod detection_activity;
mod detection_screen;
mod identity;
mod interactions;
mod rollup;
mod title_signals;

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
