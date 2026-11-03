//! Bellatrix's native Zed GPUI palette, derived from docs/design/src/styles/tokens.css.
//! The Zed theme owns appearance and lifecycle; these are semantic color roles.

use gpui::{App, Hsla};
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
    /// Colors belong to the user's active Zed theme, not to a fixed
    /// Bellatrix light/dark palette. Retain `for_mode` only for standalone
    /// previews and deterministic contrast tests.
    pub fn current(cx: &App) -> Self {
        let c = cx.theme().colors();
        let rgb24 = |color: Hsla| {
            let rgba = color.to_rgb();
            ((rgba.r.clamp(0.0, 1.0) * 255.0).round() as u32) << 16
                | ((rgba.g.clamp(0.0, 1.0) * 255.0).round() as u32) << 8
                | ((rgba.b.clamp(0.0, 1.0) * 255.0).round() as u32)
        };
        Self {
            bar: rgb24(c.title_bar_background),
            rail: rgb24(c.status_bar_background),
            panel: rgb24(c.panel_background),
            panel_alt: rgb24(c.surface_background),
            canvas: rgb24(c.editor_background),
            border: rgb24(c.border),
            text: rgb24(c.text),
            muted: rgb24(c.text_muted),
            accent: rgb24(c.text_accent),
            selected: rgb24(c.element_selected),
            selection: rgb24(c.border_selected),
        }
    }

    /// Diagram silhouettes need foreground contrast, unlike subtle UI separators.
    pub const fn diagram_outline(self, selected: bool) -> u32 {
        if selected {
            self.selection
        } else {
            self.text
        }
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
        let c = cx.theme().colors();
        Self {
            canvas: c.editor_background,
            surface: c.surface_background,
            panel: c.panel_background,
            border: c.border,
            text: c.text,
            muted: c.text_muted,
            selection: c.border_selected,
            accent: c.text_accent,
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
    fn diagram_outlines_contrast_with_canvas_and_fills() {
        for mode in [ThemeMode::Dark, ThemeMode::Light] {
            let palette = Palette::for_mode(mode);
            for selected in [false, true] {
                let outline = luminance(palette.diagram_outline(selected));
                for background in [palette.canvas, palette.panel, palette.panel_alt] {
                    let background = luminance(background);
                    let ratio = (outline.max(background) + 0.05) / (outline.min(background) + 0.05);
                    assert!(ratio >= 3.0, "{mode:?}, selected={selected}: {ratio}");
                }
            }
            assert_eq!(palette.diagram_outline(false), palette.text);
            assert_eq!(palette.diagram_outline(true), palette.selection);
        }
    }

    #[test]
    fn diagram_outlines_follow_theme_foreground_and_selection() {
        let palette = Palette {
            text: 0x123456,
            selection: 0xabcdef,
            ..Palette::for_mode(ThemeMode::Dark)
        };
        assert_eq!(palette.diagram_outline(false), 0x123456);
        assert_eq!(palette.diagram_outline(true), 0xabcdef);
    }

    #[test]
    fn density_sizes_are_ordered() {
        assert!(Density::Compact.row_height() < Density::Comfortable.row_height());
        assert!(Density::Compact.toolbar_height() < Density::Comfortable.toolbar_height());
    }
}
