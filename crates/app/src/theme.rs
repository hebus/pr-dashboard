//! Color tokens, ported 1:1 from the `--c-*` CSS variables of `src/index.css`.

use gpui::{rgb, Global, Rgba};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    Light,
    Dark,
}

impl ThemeMode {
    pub fn parse(s: &str) -> Self {
        if s.eq_ignore_ascii_case("light") {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub bg: Rgba,
    pub bg_subtle: Rgba,
    pub bg_hover: Rgba,
    pub bg_inset: Rgba,
    pub border: Rgba,
    pub text: Rgba,
    pub text_muted: Rgba,
    pub text_subtle: Rgba,
    pub accent: Rgba,

    pub green: Rgba,
    pub green_bg: Rgba,
    pub green_border: Rgba,
    pub green_btn: Rgba,
    pub green_btn_hover: Rgba,

    pub card_partial_bg: Rgba,
    pub card_partial_bg_hover: Rgba,
    pub card_approved_bg: Rgba,
    pub card_approved_bg_hover: Rgba,

    pub amber: Rgba,
    pub amber_bg: Rgba,
    pub amber_border: Rgba,

    pub red: Rgba,
    pub red_bg: Rgba,
    pub red_border: Rgba,

    pub indigo: Rgba,
    pub indigo_bg: Rgba,
    pub indigo_border: Rgba,

    pub violet: Rgba,
    pub violet_bg: Rgba,
    pub violet_border: Rgba,
}

impl Global for Theme {}

impl Theme {
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::light(),
            ThemeMode::Dark => Self::dark(),
        }
    }

    pub fn light() -> Self {
        Theme {
            bg: rgb(0xffffff),
            bg_subtle: rgb(0xf6f8fa),
            bg_hover: rgb(0xf3f4f6),
            bg_inset: rgb(0xeaeef2),
            border: rgb(0xd0d7de),
            text: rgb(0x1f2328),
            text_muted: rgb(0x57606a),
            text_subtle: rgb(0x6e7781),
            accent: rgb(0x0969da),
            green: rgb(0x1a7f37),
            green_bg: rgb(0xdafbe1),
            green_border: rgb(0x1a7f37),
            green_btn: rgb(0x1a7f37),
            green_btn_hover: rgb(0x116329),
            card_partial_bg: rgb(0xf0fdf4),
            card_partial_bg_hover: rgb(0xe2faea),
            card_approved_bg: rgb(0xd4f8de),
            card_approved_bg_hover: rgb(0xc1f2cc),
            amber: rgb(0x9a6700),
            amber_bg: rgb(0xfff8c5),
            amber_border: rgb(0xbf8700),
            red: rgb(0xcf222e),
            red_bg: rgb(0xffebe9),
            red_border: rgb(0xcf222e),
            indigo: rgb(0x4f46e5),
            indigo_bg: rgb(0xeef2ff),
            indigo_border: rgb(0x4f46e5),
            violet: rgb(0x8250df),
            violet_bg: rgb(0xfbefff),
            violet_border: rgb(0x8250df),
        }
    }

    pub fn dark() -> Self {
        Theme {
            bg: rgb(0x0d1117),
            bg_subtle: rgb(0x161b22),
            bg_hover: rgb(0x1c2128),
            bg_inset: rgb(0x21262d),
            border: rgb(0x30363d),
            text: rgb(0xe6edf3),
            text_muted: rgb(0x8b949e),
            text_subtle: rgb(0x484f58),
            accent: rgb(0x58a6ff),
            green: rgb(0x3fb950),
            green_bg: rgb(0x1a4428),
            green_border: rgb(0x238636),
            green_btn: rgb(0x238636),
            green_btn_hover: rgb(0x2ea043),
            card_partial_bg: rgb(0x0b1e12),
            card_partial_bg_hover: rgb(0x0e2416),
            card_approved_bg: rgb(0x0f2e1a),
            card_approved_bg_hover: rgb(0x143d22),
            amber: rgb(0xd29922),
            amber_bg: rgb(0x2d2005),
            amber_border: rgb(0x9e6a03),
            red: rgb(0xf85149),
            red_bg: rgb(0x3d1212),
            red_border: rgb(0xda3633),
            indigo: rgb(0xa5b4fc),
            indigo_bg: rgb(0x1e1b4b),
            indigo_border: rgb(0x6366f1),
            violet: rgb(0xd8b4fe),
            violet_bg: rgb(0x2e1065),
            violet_border: rgb(0xa855f7),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mode_with_dark_as_fallback() {
        assert_eq!(ThemeMode::parse("light"), ThemeMode::Light);
        assert_eq!(ThemeMode::parse("LIGHT"), ThemeMode::Light);
        assert_eq!(ThemeMode::parse("dark"), ThemeMode::Dark);
        assert_eq!(ThemeMode::parse("whatever"), ThemeMode::Dark);
        assert_eq!(ThemeMode::Dark.toggled().as_str(), "light");
    }
}
