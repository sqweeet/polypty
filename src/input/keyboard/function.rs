use super::csi::modifier;

pub(super) fn encode_function(number: u8, ctrl: bool, alt: bool, shift: bool) -> Vec<u8> {
    let modifier = modifier(ctrl, alt, shift);
    if let 1..=4 = number {
        let final_byte = b'P' + (number - 1);
        return if modifier == 1 {
            vec![0x1b, b'O', final_byte]
        } else {
            format!("\x1b[1;{modifier}{}", final_byte as char).into_bytes()
        };
    }

    let base = match number {
        5 => "15",
        6 => "17",
        7 => "18",
        8 => "19",
        9 => "20",
        10 => "21",
        11 => "23",
        12 => "24",
        _ => return Vec::new(),
    };
    if modifier == 1 {
        format!("\x1b[{base}~").into_bytes()
    } else {
        format!("\x1b[{base};{modifier}~").into_bytes()
    }
}
