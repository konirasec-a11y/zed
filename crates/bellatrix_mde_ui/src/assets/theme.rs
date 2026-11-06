//! Bellatrix's native Zed GPUI palette.
//! The Zed theme owns appearance and lifecycle; these are semantic color roles.

use gpui::{App, Hsla};
use ui::prelude::ActiveTheme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub panel: u32,
    pub panel_alt: u32,
    pub canvas: u32,
    pub border: u32,
    pub text: u32,
    pub muted: u32,
    pub accent: u32,
    pub selection: u32,
}

impl Palette {
    /// Colors belong to the user's active Zed theme, not to a fixed Bellatrix palette.
    pub fn current(cx: &App) -> Self {
        let c = cx.theme().colors();
        let rgb24 = |color: Hsla| {
            let rgba = color.to_rgb();
            ((rgba.r.clamp(0.0, 1.0) * 255.0).round() as u32) << 16
                | ((rgba.g.clamp(0.0, 1.0) * 255.0).round() as u32) << 8
                | ((rgba.b.clamp(0.0, 1.0) * 255.0).round() as u32)
        };
        Self {
            panel: rgb24(c.panel_background),
            panel_alt: rgb24(c.surface_background),
            canvas: rgb24(c.editor_background),
            border: rgb24(c.border),
            text: rgb24(c.text),
            muted: rgb24(c.text_muted),
            accent: rgb24(c.text_accent),
            selection: rgb24(c.border_selected),
        }
    }

    /// Diagram silhouettes need foreground contrast, unlike subtle UI separators.
    pub const fn diagram_outline(self, selected: bool) -> u32 {
        if selected { self.selection } else { self.text }
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
