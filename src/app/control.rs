mod snapshot;
mod target;

use anyhow::{bail, Result};

use crate::control::{ControlRequest, ControlResponse, PaneCapture};

use super::App;

impl App {
    pub(crate) fn handle_control(&mut self, request: ControlRequest) -> ControlResponse {
        self.run_control(request)
            .unwrap_or_else(ControlResponse::error)
    }

    fn run_control(&mut self, request: ControlRequest) -> Result<ControlResponse> {
        match request {
            ControlRequest::Ping => Ok(ControlResponse::Pong {
                pid: std::process::id(),
            }),
            ControlRequest::ListSessions => Ok(ControlResponse::Sessions {
                sessions: vec![self.session_summary()],
            }),
            ControlRequest::ListTabs => Ok(ControlResponse::Tabs {
                tabs: self.tab_summaries(),
            }),
            ControlRequest::NewTab => {
                self.spawn_workspace()?;
                Ok(ControlResponse::Tab {
                    tab: self.active_tab_summary(),
                })
            }
            ControlRequest::SelectTab { target } => self.select_control_tab(&target),
            ControlRequest::CloseTab { target } => self.close_control_tab(&target),
            ControlRequest::CapturePane { tab, pane } => self.capture_control_pane(tab, pane),
            ControlRequest::SendKeys {
                tab,
                pane,
                text,
                enter,
            } => self.send_control_keys(tab, pane, text, enter),
        }
    }

    fn select_control_tab(&mut self, target: &str) -> Result<ControlResponse> {
        let index = self.resolve_tab(target)?;
        self.select_workspace(index)?;
        Ok(ControlResponse::Tab {
            tab: self.tab_summary(index),
        })
    }

    fn close_control_tab(&mut self, target: &str) -> Result<ControlResponse> {
        let index = self.resolve_tab(target)?;
        let final_tab = self.book.len() == 1;
        self.close_workspace(index)?;
        let message = if final_tab {
            "exit confirmation shown"
        } else {
            "tab closed"
        };
        Ok(ControlResponse::Ack {
            message: message.into(),
        })
    }

    fn capture_control_pane(
        &self,
        tab: Option<String>,
        pane: Option<u64>,
    ) -> Result<ControlResponse> {
        let (index, pane) = self.resolve_pane(tab.as_deref(), pane)?;
        let workspace = self.book.get(index).expect("resolved tab exists");
        let screen = workspace.pane_screen(pane).expect("resolved pane exists");
        Ok(ControlResponse::Capture {
            capture: PaneCapture {
                tab: index + 1,
                pane,
                text: capture_screen(screen),
            },
        })
    }

    fn send_control_keys(
        &mut self,
        tab: Option<String>,
        pane: Option<u64>,
        text: String,
        enter: bool,
    ) -> Result<ControlResponse> {
        let (index, pane) = self.resolve_pane(tab.as_deref(), pane)?;
        let mut bytes = text.into_bytes();
        if enter {
            bytes.push(b'\r');
        }
        let workspace = self.book.get_mut(index).expect("resolved tab exists");
        if !workspace.write_pane(pane, &bytes)? {
            bail!("pane %{pane} no longer exists");
        }
        Ok(ControlResponse::Ack {
            message: format!(
                "sent {} bytes to tab {} pane %{pane}",
                bytes.len(),
                index + 1
            ),
        })
    }
}

fn capture_screen(screen: &vt100::Screen) -> String {
    let (_, cols) = screen.size();
    let mut rows = screen
        .rows(0, cols)
        .map(|row| row.trim_end().to_owned())
        .collect::<Vec<_>>();
    while rows.last().is_some_and(String::is_empty) {
        rows.pop();
    }
    rows.join("\n")
}
