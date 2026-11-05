//! Bellatrix semantic colors with optional host theme integration.

#[cfg(feature = "zed_theme")]
use ::theme::ActiveTheme;
use gpui::{App, Global, Hsla, rgb};

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

impl Global for Palette {}

impl Default for Palette {
    fn default() -> Self {
        Self {
            panel: 0x222427,
            panel_alt: 0x292b30,
            canvas: 0x24272c,
            border: 0x373b41,
            text: 0xc8cbd1,
            muted: 0x9ba0a9,
            accent: 0x88b1fa,
            selection: 0x83b5ff,
        }
    }
}

impl Palette {
    pub fn current(cx: &App) -> Self {
        if let Some(palette) = cx.try_global::<Self>() {
            return *palette;
        }
        #[cfg(feature = "zed_theme")]
        if cx.try_global::<::theme::GlobalTheme>().is_some() {
            return Self::from_host_theme(cx);
        }
        Self::default()
    }

    pub fn set(self, cx: &mut App) {
        cx.set_global(self);
    }

    #[cfg(feature = "zed_theme")]
    fn from_host_theme(cx: &App) -> Self {
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

/// Semantic roles used by Bellatrix application surfaces.
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
        if let Some(palette) = cx.try_global::<Palette>() {
            return Self::from(*palette);
        }
        #[cfg(feature = "zed_theme")]
        if cx.try_global::<::theme::GlobalTheme>().is_some() {
            return Self::from_host_theme(cx);
        }
        Self::from(Palette::default())
    }

    #[cfg(feature = "zed_theme")]
    fn from_host_theme(cx: &App) -> Self {
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

impl From<Palette> for WorkbenchPalette {
    fn from(palette: Palette) -> Self {
        Self {
            canvas: rgb(palette.canvas).into(),
            surface: rgb(palette.panel_alt).into(),
            panel: rgb(palette.panel).into(),
            border: rgb(palette.border).into(),
            text: rgb(palette.text).into(),
            muted: rgb(palette.muted).into(),
            selection: rgb(palette.selection).into(),
            accent: rgb(palette.accent).into(),
        }
    }
}
