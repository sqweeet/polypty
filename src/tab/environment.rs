use portable_pty::CommandBuilder;

pub(super) fn child_command(id: u64, shell: Option<&str>) -> CommandBuilder {
    let mut command = CommandBuilder::new(selected_shell(shell));
    for (name, value) in child_terminal_environment() {
        command.env(name, value);
    }
    command.env("MUX", "1");
    command.env("MUX_TAB", id.to_string());
    if let Ok(cwd) = std::env::current_dir() {
        command.cwd(cwd);
    }
    command
}

pub(super) fn child_terminal_environment() -> [(&'static str, &'static str); 2] {
    // Advertise only capabilities implemented by vt100 + mux's input encoder.
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
