/// Encode a printable character with the conventional Ctrl/Alt transforms.
pub(super) fn encode_character(character: char, ctrl: bool, alt: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    if alt {
        bytes.push(0x1b);
    }
    if ctrl {
        if let Some(control) = control_byte(character) {
            bytes.push(control);
        } else {
            push_utf8(&mut bytes, character);
        }
    } else {
        push_utf8(&mut bytes, character);
    }
    bytes
}

fn control_byte(character: char) -> Option<u8> {
    match character {
        '@' | ' ' => Some(0x00),
        'a'..='z' => Some((character as u8) - b'a' + 1),
        'A'..='Z' => Some((character as u8) - b'A' + 1),
        '[' => Some(0x1b),
        '\\' => Some(0x1c),
        ']' => Some(0x1d),
        '^' => Some(0x1e),
        '_' => Some(0x1f),
        '?' => Some(0x7f),
        _ => None,
    }
}

fn push_utf8(bytes: &mut Vec<u8>, character: char) {
    let mut buffer = [0_u8; 4];
    bytes.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
}
