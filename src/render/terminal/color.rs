use crossterm::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PackedColor(u32);

impl PackedColor {
    pub(super) const DEFAULT: Self = Self(0xFFFF_FFFF);
    pub(super) const INVALID: Self = Self(0xDEAD_BEEF);

    pub(super) fn from_vt(color: vt100::Color) -> Self {
        match color {
            vt100::Color::Default => Self::DEFAULT,
            vt100::Color::Idx(index) => Self(0x0100_0000 | u32::from(index)),
            vt100::Color::Rgb(r, g, b) => {
                Self(0x0200_0000 | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b))
            }
        }
    }

    pub(super) fn to_crossterm(self) -> Option<Color> {
        if self == Self::DEFAULT {
            return None;
        }
        match (self.0 >> 24) & 0xff {
            1 => Some(Color::AnsiValue((self.0 & 0xff) as u8)),
            2 => Some(Color::Rgb {
                r: ((self.0 >> 16) & 0xff) as u8,
                g: ((self.0 >> 8) & 0xff) as u8,
                b: (self.0 & 0xff) as u8,
            }),
            _ => None,
        }
    }
}
