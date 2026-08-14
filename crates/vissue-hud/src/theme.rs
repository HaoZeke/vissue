//! Teal and coral from `assets/logo.svg` / `assets/logo-crest.svg`.

use iced::Color;

/// Navy field behind the mark (`#1d3d58`).
pub const NAVY: Color = Color::from_rgb8(0x1d, 0x3d, 0x58);
/// Cream faces (`#f4f1ea`).
pub const CREAM: Color = Color::from_rgb8(0xf4, 0xf1, 0xea);
/// Ready node teal (`#0e7a76`).
pub const TEAL: Color = Color::from_rgb8(0x0e, 0x7a, 0x76);
/// Crest coral (`#e87a5c`).
pub const CORAL: Color = Color::from_rgb8(0xe8, 0x7a, 0x5c);

/// iced theme using the mark colours.
pub fn theme() -> iced::Theme {
    iced::Theme::custom(
        "vissue".to_string(),
        iced::theme::Palette {
            background: NAVY,
            text: CREAM,
            primary: TEAL,
            success: TEAL,
            warning: CORAL,
            danger: CORAL,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_uses_logo_teal() {
        let t = theme();
        let p = t.palette();
        assert_eq!(p.primary, TEAL);
        assert_eq!(p.danger, CORAL);
        assert_eq!(p.background, NAVY);
    }
}
