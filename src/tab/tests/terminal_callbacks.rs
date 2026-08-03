use crate::tab::terminal::callbacks::{TerminalCallbacks, TEST_COLOR_REPLIES};

#[test]
fn replies_to_dynamic_color_queries_without_forwarding_unknown_osc() {
    let mut parser = vt100::Parser::new_with_callbacks(24, 80, 0, TerminalCallbacks::default());
    parser.process(b"\x1b]10;?\x1b\\\x1b]11;?\x07\x1b]12;?\x1b\\");
    parser.process(b"\x1b]52;c;?\x1b\\\x1b]999;?\x1b\\");
    let expected: Vec<u8> = TEST_COLOR_REPLIES.concat();
    assert_eq!(parser.callbacks_mut().take_responses(), expected);
}

#[test]
fn replies_to_status_cursor_and_device_attribute_queries() {
    let mut parser = vt100::Parser::new_with_callbacks(24, 80, 0, TerminalCallbacks::default());
    parser.process(b"\x1b[3;4H\x1b[5n\x1b[6n\x1b[c\x1b[>c");
    assert_eq!(
        parser.callbacks_mut().take_responses(),
        b"\x1b[0n\x1b[3;4R\x1b[?1;2c\x1b[>0;0;0c"
    );
}
