pub(super) fn modifier(ctrl: bool, alt: bool, shift: bool) -> u8 {
    1 + u8::from(shift) + 2 * u8::from(alt) + 4 * u8::from(ctrl)
}

pub(super) fn arrow(letter: u8, ctrl: bool, alt: bool, shift: bool, app_cursor: bool) -> Vec<u8> {
    let modifier = modifier(ctrl, alt, shift);
    if modifier == 1 {
        let introducer = if app_cursor { b'O' } else { b'[' };
        vec![0x1b, introducer, letter]
    } else {
        format!("\x1b[1;{modifier}{}", letter as char).into_bytes()
    }
}

pub(super) fn modded_letter(letter: u8, ctrl: bool, alt: bool, shift: bool) -> Vec<u8> {
    let modifier = modifier(ctrl, alt, shift);
    if modifier == 1 {
        vec![0x1b, b'[', letter]
    } else {
        format!("\x1b[1;{modifier}{}", letter as char).into_bytes()
    }
}

pub(super) fn modded(suffix: &[u8], ctrl: bool, alt: bool, shift: bool) -> Vec<u8> {
    let modifier = modifier(ctrl, alt, shift);
    let mut output = vec![0x1b, b'['];
    if modifier == 1 {
        output.extend_from_slice(suffix);
    } else if let Some((last, head)) = suffix.split_last() {
        output.extend_from_slice(head);
        output.push(b';');
        output.extend_from_slice(modifier.to_string().as_bytes());
        output.push(*last);
    }
    output
}

pub(super) fn backtab(ctrl: bool, alt: bool) -> Vec<u8> {
    if !ctrl && !alt {
        return b"\x1b[Z".to_vec();
    }
    let modifier = modifier(ctrl, alt, true);
    format!("\x1b[1;{modifier}Z").into_bytes()
}
