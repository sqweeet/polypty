mod args;
mod cli;
mod client;
mod model;
mod output;
mod path;
mod server;

pub(crate) use model::{
    AgentSummary, ControlRequest, ControlResponse, PaneCapture, SessionSummary, TabSummary,
};
pub(crate) use path::socket_path;
pub(crate) use server::ControlServer;

pub(crate) const SESSION_NAME: &str = "main";

pub(crate) fn dispatch_cli() -> anyhow::Result<bool> {
    cli::dispatch()
}

#[cfg(test)]
mod tests;
