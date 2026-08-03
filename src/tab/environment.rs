use portable_pty::CommandBuilder;
use std::path::Path;

pub(super) fn child_command(
    pane_id: u64,
    tab_id: u64,
    shell: Option<&str>,
    control_socket: Option<&Path>,
) -> CommandBuilder {
    let mut command = CommandBuilder::new(selected_shell(shell));
    for (name, value) in child_terminal_environment() {
        command.env(name, value);
    }
    command.env("POLYPTY", "1");
    command.env("POLYPTY_TAB", tab_id.to_string());
    command.env("POLYPTY_PANE", pane_id.to_string());
    command.env("POLYPTY_SESSION", crate::control::SESSION_NAME);
    if let Some(path) = control_socket {
        command.env("POLYPTY_SOCKET", path.as_os_str());
    }
    if let Ok(cwd) = std::env::current_dir() {
        command.cwd(cwd);
    }
    command
}

pub(super) fn child_terminal_environment() -> [(&'static str, &'static str); 2] {
    // Advertise only capabilities implemented by vt100 + polypty's input encoder.
    [("TERM", "xterm-256color"), ("COLORTERM", "truecolor")]
}

pub(super) fn selected_shell(configured: Option<&str>) -> String {
    configured.map(str::to_owned).unwrap_or_else(|| {
        std::env::var("SHELL").unwrap_or_else(|_| {
            if cfg!(target_os = "windows") {
                "powershell.exe".into()
            } else {
                "/bin/bash".into()
            }
        })
    })
}
