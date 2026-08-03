use super::decode::percent_decode;

pub(super) enum OscUpdate {
    Title(String),
    Cwd(String),
}

pub(super) fn parse(sequence: &[u8]) -> Option<OscUpdate> {
    if sequence.len() < 4 || sequence[0] != 0x1b || sequence[1] != b']' {
        return None;
    }
    let end = if sequence.ends_with(&[0x1b, b'\\']) {
        sequence.len() - 2
    } else if sequence.ends_with(&[0x07]) {
        sequence.len() - 1
    } else {
        return None;
    };
    let body = &sequence[2..end];
    let separator = body.iter().position(|byte| *byte == b';')?;
    let (command, value) = (&body[..separator], &body[separator + 1..]);
    match command {
        b"0" | b"2" => Some(OscUpdate::Title(sanitize_title(value))),
        b"7" => parse_cwd(value).map(OscUpdate::Cwd),
        _ => None,
    }
}

fn sanitize_title(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .chars()
        .filter(|character| !character.is_control())
        .collect()
}

fn parse_cwd(bytes: &[u8]) -> Option<String> {
    let value = std::str::from_utf8(bytes).ok()?.trim();
    let rest = value.strip_prefix("file://")?;
    let path = &rest[rest.find('/')?..];
    let decoded = percent_decode(path)?;
    (!decoded.is_empty()).then_some(decoded)
}
