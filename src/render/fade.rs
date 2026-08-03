use crossterm::style::Color;

pub(super) fn blend(from: Color, to: Color, alpha: u8) -> Color {
    let (
        Color::Rgb {
            r: from_r,
            g: from_g,
            b: from_b,
        },
        Color::Rgb {
            r: to_r,
            g: to_g,
            b: to_b,
        },
    ) = (from, to)
    else {
        return to;
    };
    Color::Rgb {
        r: channel(from_r, to_r, alpha),
        g: channel(from_g, to_g, alpha),
        b: channel(from_b, to_b, alpha),
    }
}

fn channel(from: u8, to: u8, alpha: u8) -> u8 {
    let alpha = u16::from(alpha);
    ((u16::from(from) * (255 - alpha) + u16::from(to) * alpha) / 255) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_blend_respects_both_endpoints() {
        let from = Color::Rgb {
            r: 10,
            g: 20,
            b: 30,
        };
        let to = Color::Rgb {
            r: 110,
            g: 120,
            b: 130,
        };
        assert_eq!(blend(from, to, 0), from);
        assert_eq!(blend(from, to, 255), to);
    }
}
