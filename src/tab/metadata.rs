use std::time::{Duration, Instant};

use crate::info::{self, TabInfo};

use super::Tab;

pub(super) struct SessionMetadata {
    cwd: Option<String>,
    process: Option<String>,
    last_probe: Instant,
}

impl SessionMetadata {
    pub fn new(now: Instant) -> Self {
        Self {
            cwd: None,
            process: None,
            last_probe: now - Duration::from_secs(10),
        }
    }

    pub fn needs_probe(&self) -> bool {
        self.last_probe.elapsed() >= Duration::from_millis(400)
    }

    pub fn process(&self) -> Option<&str> {
        self.process.as_deref()
    }
}

impl Tab {
    pub(super) fn refresh_info(&mut self, force: bool) -> bool {
        self.metadata.last_probe = Instant::now();
        if let Some(pid) = self.transport.shell_pid() {
            let foreground = self.transport.foreground_process_group();
            let (cwd, process) = info::probe_session(pid, foreground);
            self.apply_probed_metadata(cwd, process);
        }
        let previous = self.info.clone();
        self.recompose();
        force || self.info != previous
    }

    fn apply_probed_metadata(&mut self, cwd: Option<String>, process: Option<String>) {
        if let Some(cwd) = cwd {
            if self.terminal.cwd().is_none() {
                self.metadata.cwd = Some(cwd);
            } else if self.metadata.cwd.is_none() {
                self.metadata.cwd = self.terminal.cwd().map(str::to_owned);
            }
        } else if self.metadata.cwd.is_none() {
            self.metadata.cwd = self.terminal.cwd().map(str::to_owned);
        }
        if let Some(process) = process {
            self.metadata.process = Some(process);
        }
    }

    pub(super) fn sync_terminal_metadata(&mut self) {
        if !self.custom_title {
            if let Some(title) = self.terminal.title() {
                if title != self.title {
                    self.title = title.to_owned();
                }
            }
        }
        if let Some(cwd) = self.terminal.cwd() {
            if self.metadata.cwd.as_deref() != Some(cwd) {
                self.metadata.cwd = Some(cwd.to_owned());
            }
        }
    }

    pub(super) fn recompose(&mut self) {
        if let Some(cwd) = self.terminal.cwd() {
            self.metadata.cwd = Some(cwd.to_owned());
        }
        let mut info = info::compose_info(
            &self.title,
            self.metadata.cwd.as_deref(),
            self.metadata.process.as_deref(),
            self.custom_title,
        );
        info.agent = self.agent.status();
        self.info = info;
    }
}

pub(super) fn initial_info() -> TabInfo {
    TabInfo {
        primary: "shell".into(),
        secondary: String::new(),
        agent: None,
    }
}
