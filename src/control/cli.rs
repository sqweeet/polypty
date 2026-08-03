use anyhow::Result;

use super::{args, client, output, socket_path, ControlRequest};

const HELP: &str = "\
polypty — interactive terminal multiplexer and control client

USAGE:
  polypty
  polypty <COMMAND> [OPTIONS]

COMMANDS:
  list-sessions, ls                 Show the running polypty session
  list-tabs, list-windows           List tabs and pane IDs
  new-tab, new-window               Create and focus a tab
  select-tab, select-window TARGET  Focus a tab (TARGET or -t TARGET)
  close-tab, kill-window TARGET     Close a tab (TARGET or -t TARGET)
  capture-pane [-t TAB] [-p PANE]   Print a pane's current screen
  send-keys [-t TAB] [-p PANE]      Send text; add --enter for Return
  ping                              Check that the session is responsive

OPTIONS:
  --json       Emit machine-readable JSON
  --help       Show this help
  --version    Show polypty version

EXAMPLES:
  polypty list-tabs --json
  polypty capture-pane -t 2
  polypty send-keys -t 2 --enter -- 'cargo test'
";

pub(super) fn dispatch() -> Result<bool> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let Some(first) = args.first() else {
        return Ok(false);
    };
    if matches!(first.as_str(), "--help" | "-h" | "help") {
        print!("{HELP}");
        return Ok(true);
    }
    if matches!(first.as_str(), "--version" | "-V" | "version") {
        println!("polypty {}", env!("CARGO_PKG_VERSION"));
        return Ok(true);
    }
    let invocation = args::parse(&args)?;
    let request = apply_client_context(invocation.request);
    let response = client::exchange(&socket_path()?, &request)?;
    output::print(response, invocation.json)?;
    Ok(true)
}

fn apply_client_context(mut request: ControlRequest) -> ControlRequest {
    let tab = std::env::var("POLYPTY_TAB")
        .ok()
        .filter(|value| !value.is_empty())
        .map(|value| format!("@{value}"));
    let pane = std::env::var("POLYPTY_PANE")
        .ok()
        .and_then(|value| value.parse().ok());
    apply_context(&mut request, tab, pane);
    request
}

pub(super) fn apply_context(request: &mut ControlRequest, tab: Option<String>, pane: Option<u64>) {
    match request {
        ControlRequest::CapturePane {
            tab: request_tab @ None,
            pane: request_pane,
        }
        | ControlRequest::SendKeys {
            tab: request_tab @ None,
            pane: request_pane,
            ..
        } => {
            *request_tab = tab;
            *request_pane = request_pane.or(pane);
        }
        _ => {}
    }
}
