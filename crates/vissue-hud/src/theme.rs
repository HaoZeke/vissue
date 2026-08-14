//! Catppuccin Mocha tokens and Atkinson Hyperlegible sizes.
//!
//! Colours and the selected-row invert match the seat rofi palette
//! (`~/.config/rofi/colors.rasi`). Atkinson Hyperlegible is the system
//! family when present; iced falls back to the default sans if it is not.
//! `SIZE_BODY` is the app base: iced's default text size, and the size
//! unsized widgets (including markdown headings) inherit.

use iced::{Color, Font};

/// Sidebar and chrome (`#181825`).
pub const MANTLE: Color = Color::from_rgb8(0x18, 0x18, 0x25);
/// Main field (`#1e1e2e`).
pub const BASE: Color = Color::from_rgb8(0x1e, 0x1e, 0x2e);
/// Selected row (`#313244`).
pub const SURFACE0: Color = Color::from_rgb8(0x31, 0x32, 0x44);
/// Hairlines (`#45475a`).
pub const SURFACE1: Color = Color::from_rgb8(0x45, 0x47, 0x5a);
/// Rofi border (`#585b70`, Catppuccin overlay1).
pub const OVERLAY1: Color = Color::from_rgb8(0x58, 0x5b, 0x70);
/// Title text (`#cdd6f4`).
pub const TEXT: Color = Color::from_rgb8(0xcd, 0xd6, 0xf4);
/// Meta line (`#a6adc8`).
pub const SUBTEXT: Color = Color::from_rgb8(0xa6, 0xad, 0xc8);
/// Dim labels (`#6c7086`).
pub const OVERLAY: Color = Color::from_rgb8(0x6c, 0x70, 0x86);
/// Accent and selected filter (`#89b4fa`).
pub const BLUE: Color = Color::from_rgb8(0x89, 0xb4, 0xfa);
/// Priority A (`#f38ba8`).
pub const RED: Color = Color::from_rgb8(0xf3, 0x8b, 0xa8);
/// Priority B (`#fab387`).
pub const PEACH: Color = Color::from_rgb8(0xfa, 0xb3, 0x87);
/// Done and success (`#a6e3a1`).
pub const GREEN: Color = Color::from_rgb8(0xa6, 0xe3, 0xa1);
/// Blocked (`#f9e2af`).
pub const YELLOW: Color = Color::from_rgb8(0xf9, 0xe2, 0xaf);

/// Project name. A step above the base, not a second default.
pub const SIZE_TITLE: f32 = 18.0;
/// App base. iced `default_text_size`. Task titles and inputs.
pub const SIZE_BODY: f32 = 16.0;
/// Chips, meta, ids.
pub const SIZE_META: f32 = 14.0;
/// Hint bar.
pub const SIZE_HINT: f32 = 13.0;

/// Seat family. cosmic-text resolves it from the system fontconfig set.
pub const FACE: Font = Font::with_name("Atkinson Hyperlegible");

/// iced theme using Mocha.
pub fn theme() -> iced::Theme {
    iced::Theme::custom(
        "vissue-mocha".to_string(),
        iced::theme::Palette {
            background: BASE,
            text: TEXT,
            primary: BLUE,
            success: GREEN,
            warning: YELLOW,
            danger: RED,
        },
    )
}

/// Priority pip: A red, B peach, C quiet blue-grey.
pub fn priority_color(priority: &str) -> Color {
    match priority {
        "A" => RED,
        "B" => PEACH,
        _ => OVERLAY,
    }
}

/// State chip colour.
pub fn state_color(state: &str) -> Color {
    match state {
        "STARTED" => BLUE,
        "BLOCKED" => YELLOW,
        "DONE" => GREEN,
        "CANCELLED" => OVERLAY,
        _ => SUBTEXT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_uses_mocha_tokens() {
        let t = theme();
        let p = t.palette();
        assert_eq!(p.primary, BLUE);
        assert_eq!(p.danger, RED);
        assert_eq!(p.background, BASE);
        assert_eq!(p.success, GREEN);
    }

    #[test]
    fn priority_pips_match_todoist_order() {
        assert_eq!(priority_color("A"), RED);
        assert_eq!(priority_color("B"), PEACH);
        assert_eq!(priority_color("C"), OVERLAY);
    }

    #[test]
    fn body_is_the_app_base_and_smaller_than_title() {
        assert!(SIZE_BODY < SIZE_TITLE);
        assert!(SIZE_META < SIZE_BODY);
        assert_eq!(OVERLAY1, Color::from_rgb8(0x58, 0x5b, 0x70));
    }
}
