use std::io::{self, Write};

use anyhow::{bail, Result};

use super::{ControlResponse, SessionSummary, TabSummary};

pub(super) fn print(response: ControlResponse, json: bool) -> Result<()> {
    if let ControlResponse::Error { message } = &response {
        bail!("{message}");
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&response)?);
        return Ok(());
    }
    match response {
        ControlResponse::Pong { pid } => println!("main: alive (pid {pid})"),
        ControlResponse::Sessions { sessions } => {
            for session in sessions {
                println!("{}", format_session(&session));
            }
        }
        ControlResponse::Tabs { tabs } => {
            for tab in tabs {
                println!("{}", format_tab(&tab));
            }
        }
        ControlResponse::Tab { tab } => println!("{}", format_tab(&tab)),
        ControlResponse::Capture { capture } => {
            print!("{}", capture.text);
            if !capture.text.ends_with('\n') {
                println!();
            }
            io::stdout().flush()?;
        }
        ControlResponse::Ack { message } => println!("{message}"),
        ControlResponse::Error { .. } => unreachable!(),
    }
    Ok(())
}

fn format_session(session: &SessionSummary) -> String {
    let state = if session.attached {
        "attached"
    } else {
        "detached"
    };
    format!(
        "{}: {} tabs (active {}, {state}, pid {})",
        session.name, session.tabs, session.active_tab, session.pid
    )
}

fn format_tab(tab: &TabSummary) -> String {
    let title = if tab.title.is_empty() {
        "shell"
    } else {
        &tab.title
    };
    let mut line = format!(
        "{}: {}  panes={} [{}]",
        tab.index,
        title,
        tab.panes.len(),
        tab.panes
            .iter()
            .map(u64::to_string)
            .collect::<Vec<_>>()
            .join(",")
    );
    if !tab.cwd.is_empty() {
        line.push_str(&format!("  {}", tab.cwd));
    }
    if let Some(agent) = &tab.agent {
        line.push_str(&format!("  {}/{}", agent.name, agent.state));
    }
    if tab.active {
        line.push_str("  [active]");
    }
    line
}
