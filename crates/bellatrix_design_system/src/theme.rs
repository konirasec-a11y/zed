//! Bellatrix's native Zed GPUI palette, derived from docs/design/src/styles/tokens.css.
//! The Zed theme owns appearance and lifecycle; these are semantic color roles.

use gpui::{App, Hsla, rgb};
use ui::prelude::ActiveTheme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub bar: u32,
    pub rail: u32,
    pub panel: u32,
    pub panel_alt: u32,
    pub canvas: u32,
    pub border: u32,
    pub text: u32,
    pub muted: u32,
    pub accent: u32,
    pub selected: u32,
    pub selection: u32,
}

impl Palette {
    pub fn current(cx: &App) -> Self {
        Self::for_mode(if cx.theme().appearance.is_light() {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        })
    }

    pub const fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Dark => Self {
                bar: 0x1b1d20,
                rail: 0x1e2023,
                panel: 0x222427,
                panel_alt: 0x292b30,
                canvas: 0x24272c,
                border: 0x373b41,
                text: 0xf0f1f3,
                muted: 0x9ba0a9,
                accent: 0x88b1fa,
                selected: 0x2c3749,
                selection: 0x83b5ff,
            },
            ThemeMode::Light => Self {
                bar: 0xf7f8fa,
                rail: 0xf7f8fa,
                panel: 0xfdfdfd,
                panel_alt: 0xf3f5f7,
                canvas: 0xfafbfc,
                border: 0xdee2e7,
                text: 0x20262e,
                muted: 0x697380,
                accent: 0x3974cf,
                selected: 0xe7effa,
                selection: 0x3a7fe2,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Density {
    Comfortable,
    Compact,
}

impl Density {
    pub const fn toolbar_height(self) -> f32 {
        match self {
            Self::Compact => 30.,
            Self::Comfortable => 36.,
        }
    }

    pub const fn row_height(self) -> f32 {
        match self {
            Self::Compact => 26.,
            Self::Comfortable => 32.,
        }
    }

    pub const fn activity_rail_width(self) -> f32 {
        match self {
            Self::Compact => 40.,
            Self::Comfortable => 44.,
        }
    }
}

/// Zed-native roles used by the MDE Workbench and other Bellatrix UI Items.
#[derive(Clone, Copy)]
pub struct WorkbenchPalette {
    pub canvas: Hsla,
    pub surface: Hsla,
    pub panel: Hsla,
    pub border: Hsla,
    pub text: Hsla,
    pub muted: Hsla,
    pub selection: Hsla,
    pub accent: Hsla,
}

impl WorkbenchPalette {
    pub fn current(cx: &App) -> Self {
        let p = Palette::current(cx);
        Self {
            canvas: Hsla::from(rgb(p.canvas)),
            surface: Hsla::from(rgb(p.panel)),
            panel: Hsla::from(rgb(p.panel_alt)),
            border: Hsla::from(rgb(p.border)),
            text: Hsla::from(rgb(p.text)),
            muted: Hsla::from(rgb(p.muted)),
            selection: Hsla::from(rgb(p.selection)),
            accent: Hsla::from(rgb(p.accent)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(rgb: u32) -> f64 {
        let channel = |shift| {
            let c = ((rgb >> shift) & 0xffu32) as f64 / 255.;
            if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        channel(16) * 0.2126 + channel(8) * 0.7152 + channel(0) * 0.0722
    }

    #[test]
    fn neutral_palettes_preserve_text_contrast() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let p = Palette::for_mode(mode);
            let (a, b) = (luminance(p.text), luminance(p.panel));
            let ratio = (a.max(b) + 0.05) / (a.min(b) + 0.05);
            assert!(ratio >= 4.5, "normal text must meet WCAG AA, got {ratio}");
            assert_ne!(p.accent, p.selected);
            assert_ne!(p.selection, p.selected);
        }
    }

    #[test]
    fn density_sizes_are_ordered() {
        assert!(Density::Compact.row_height() < Density::Comfortable.row_height());
        assert!(Density::Compact.toolbar_height() < Density::Comfortable.toolbar_height());
    }
}
