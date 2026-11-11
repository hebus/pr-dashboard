//! Small stateless UI building blocks shared by the views.

use gpui::{
    div, prelude::*, px, rgb, rgba, svg, Div, FontWeight, IntoElement, Rgba, SharedString,
    Stateful,
};
use gpui_component::tooltip::Tooltip;
use pr_dashboard_core::logic::{avatar_color_index, avatar_initials};
use pr_dashboard_core::providers::common::Reviewer;

use crate::theme::Theme;

/// Monospace family per platform (GPUI has no generic "monospace" alias).
pub const MONO: &str = if cfg!(target_os = "windows") {
    "Consolas"
} else if cfg!(target_os = "macos") {
    "Menlo"
} else {
    "DejaVu Sans Mono"
};

/// An SVG icon from `assets/icons`, tinted with `color`. Unlike text, an SVG
/// does not inherit its parent's color, so it is always explicit.
pub fn icon(path: &'static str, size: f32, color: Rgba) -> impl IntoElement {
    svg().path(path).size(px(size)).flex_none().text_color(color)
}

/// A flat icon button; the caller adds `.on_click(...)`.
pub fn icon_button(id: &'static str, path: &'static str, color: Rgba, t: &Theme) -> Stateful<Div> {
    let hover_bg = t.bg_hover;
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .p_1p5()
        .rounded_md()
        .cursor_pointer()
        .hover(move |s| s.bg(hover_bg))
        .child(icon(path, 16., color))
}

/// A text button with an optional leading icon; the caller adds `.on_click(...)`.
/// `bg: None` is a flat button that only gets a background on hover.
pub fn text_button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    icon_path: Option<&'static str>,
    fg: Rgba,
    bg: Option<Rgba>,
    hover_bg: Rgba,
) -> Stateful<Div> {
    div()
        .id(id.into())
        .flex()
        .items_center()
        .gap_1p5()
        .px_3()
        .py_1()
        .rounded_md()
        .cursor_pointer()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(fg)
        .when_some(bg, |e, bg| e.bg(bg))
        .hover(move |s| s.bg(hover_bg))
        .when_some(icon_path, |e, path| e.child(icon(path, 12., fg)))
        .child(label.into())
}

/// Rounded status badge (review state, conflicts, branch...).
pub fn badge(
    bg: Rgba,
    fg: Rgba,
    border: Rgba,
    icon_path: Option<&'static str>,
    text: impl Into<SharedString>,
) -> Div {
    div()
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .py_0p5()
        .rounded_full()
        .border_1()
        .border_color(border)
        .bg(bg)
        .text_color(fg)
        .text_size(px(11.))
        .font_weight(FontWeight::MEDIUM)
        .when_some(icon_path, |e, path| e.child(icon(path, 10., fg)))
        .child(text.into())
}

/// Small solid counter ("3" on an amber background).
pub fn count_pill(color: Rgba, n: usize) -> Div {
    div()
        .px_1p5()
        .py_0p5()
        .rounded_full()
        .bg(color)
        .text_color(rgb(0xffffff))
        .text_size(px(10.))
        .font_weight(FontWeight::BOLD)
        .child(n.to_string())
}

const AVATAR_COLORS: [(u32, u32); 8] = [
    (0xdbeafe, 0x1d4ed8),
    (0xdcfce7, 0x15803d),
    (0xfef9c3, 0xa16207),
    (0xfce7f3, 0xbe185d),
    (0xede9fe, 0x6d28d9),
    (0xffedd5, 0xc2410c),
    (0xccfbf1, 0x0f766e),
    (0xe0f2fe, 0x0369a1),
];

/// Initials of the display name, on a color derived from the login (which, unlike
/// the display name, never changes).
pub fn avatar(who: &Reviewer, size: f32) -> impl IntoElement {
    let (bg, fg) = AVATAR_COLORS[avatar_color_index(&who.login, AVATAR_COLORS.len())];
    div()
        .size(px(size))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(rgb(bg))
        .text_color(rgb(fg))
        .text_size(px((size * 0.45).floor().max(8.)))
        .font_weight(FontWeight::SEMIBOLD)
        .child(avatar_initials(&who.name))
}

/// An avatar that shows "Display Name (login)" in a tooltip on hover (the React
/// app's `title=`). `id` must be unique among siblings.
pub fn avatar_with_tooltip(
    id: impl Into<SharedString>,
    who: &Reviewer,
    size: f32,
) -> impl IntoElement {
    let label = SharedString::from(who.full_label());
    div()
        .id(id.into())
        .flex_none()
        .tooltip(move |window, cx| Tooltip::new(label.clone()).build(window, cx))
        .child(avatar(who, size))
}

/// `#rrggbb` + alpha, from the 6-digit hex that the providers normalize
/// (`normalize_hex` never returns anything else).
pub fn hex_with_alpha(hex: &str, alpha: u8) -> Rgba {
    let value = u32::from_str_radix(hex, 16).unwrap_or(0x8a8a8a) & 0x00ff_ffff;
    rgba((value << 8) | alpha as u32)
}

pub fn hex_color(hex: &str) -> Rgba {
    hex_with_alpha(hex, 0xff)
}

/// WCAG relative luminance (0 = black, 1 = white).
fn relative_luminance(c: Rgba) -> f32 {
    let lin = |v: f32| if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
    0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
}

/// WCAG contrast ratio, from 1 (identical) to 21 (black on white).
fn contrast_ratio(a: Rgba, b: Rgba) -> f32 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Minimum contrast for normal-size text (WCAG AA).
const MIN_TEXT_CONTRAST: f32 = 4.5;

/// A label's own color, nudged toward black (light theme) or white (dark theme)
/// just enough to stay readable on `background`. GitHub and GitLab let users
/// pick any color, so a pale label would otherwise be invisible on a light
/// background (and a near-black one on a dark background). Colors that already
/// read well are returned unchanged.
pub fn readable_label_color(hex: &str, background: Rgba) -> Rgba {
    let base = hex_color(hex);
    let target = if relative_luminance(background) > 0.5 {
        rgb(0x000000)
    } else {
        rgb(0xffffff)
    };
    for step in 0..=10 {
        let k = step as f32 / 10.;
        let mixed = Rgba {
            r: base.r + (target.r - base.r) * k,
            g: base.g + (target.g - base.g) * k,
            b: base.b + (target.b - base.b) * k,
            a: 1.,
        };
        if contrast_ratio(mixed, background) >= MIN_TEXT_CONTRAST {
            return mixed;
        }
    }
    target
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_text_is_always_readable_in_both_themes() {
        let light_bg = rgb(0xf6f8fa);
        let dark_bg = rgb(0x161b22);
        for hex in ["ffffff", "ededed", "fef2c0", "fbca04", "000000", "0e8a16", "5319e7", "1d76db"] {
            for bg in [light_bg, dark_bg] {
                let text = readable_label_color(hex, bg);
                assert!(
                    contrast_ratio(text, bg) >= MIN_TEXT_CONTRAST,
                    "label {hex} unreadable on {bg:?}: {text:?}"
                );
            }
        }
    }

    #[test]
    fn label_colors_that_already_read_well_are_left_alone() {
        // Dark blue on a light background, light blue on a dark one.
        assert_eq!(readable_label_color("0b3d91", rgb(0xf6f8fa)), hex_color("0b3d91"));
        assert_eq!(readable_label_color("79c0ff", rgb(0x161b22)), hex_color("79c0ff"));
        // A pale label on a light background must actually change.
        assert_ne!(readable_label_color("ededed", rgb(0xf6f8fa)), hex_color("ededed"));
    }

    #[test]
    fn hex_with_alpha_packs_rgba() {
        assert_eq!(hex_with_alpha("1d76db", 0x26), rgba(0x1d76db26));
        assert_eq!(hex_color("ffffff"), rgba(0xffffffff));
        // Garbage falls back to the neutral gray used by `normalize_hex`.
        assert_eq!(hex_color("zzzzzz"), rgba(0x8a8a8aff));
    }
}
