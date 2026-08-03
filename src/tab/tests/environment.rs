use crate::tab::environment::child_terminal_environment;

#[test]
fn child_terminal_capabilities_are_stable() {
    assert_eq!(
        child_terminal_environment(),
        [("TERM", "xterm-256color"), ("COLORTERM", "truecolor")]
    );
}
