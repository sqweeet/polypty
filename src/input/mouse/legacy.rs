use vt100::MouseProtocolEncoding;

pub(super) fn encode(code: u32, x: u32, y: u32, encoding: MouseProtocolEncoding) -> Vec<u8> {
    let values = [code + 32, x + 32, y + 32];
    let mut bytes = b"\x1b[M".to_vec();
    match encoding {
        MouseProtocolEncoding::Default => {
            for value in values {
                let Ok(byte) = u8::try_from(value) else {
                    return Vec::new();
                };
                bytes.push(byte);
            }
        }
        MouseProtocolEncoding::Utf8 => {
            for value in values {
                let Some(character) = char::from_u32(value) else {
                    return Vec::new();
                };
                let mut buffer = [0_u8; 4];
                bytes.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
            }
        }
        MouseProtocolEncoding::Sgr => unreachable!("SGR is handled by the caller"),
    }
    bytes
}
