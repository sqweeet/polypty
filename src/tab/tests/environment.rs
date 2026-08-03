use crate::tab::environment::{child_terminal_environment, selected_shell};

#[test]
fn child_terminal_capabilities_are_stable() {
    assert_eq!(
        child_terminal_environment(),
        [("TERM", "xterm-256color"), ("COLORTERM", "truecolor")]
    );
}

#[test]
fn configured_shell_wins_over_environment_default() {
    assert_eq!(
        selected_shell(Some("/bin/custom-shell")),
        "/bin/custom-shell"
    );
}
