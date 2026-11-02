//! GPUI/Zed icon descriptions independent of a particular Workbench.
use gpui::{AnyElement, IntoElement, svg};
use std::sync::Arc;
use ui::{Icon, IconName};

#[derive(Clone, Debug)]
pub enum BellatrixIcon {
    Zed(IconName),
    Asset(String),
    Svg(Arc<[u8]>),
}

impl BellatrixIcon {
    pub fn zed(name: IconName) -> Self { Self::Zed(name) }
    pub fn asset(path: impl Into<String>) -> Self { Self::Asset(path.into()) }
    pub fn svg(bytes: impl Into<Arc<[u8]>>) -> Self { Self::Svg(bytes.into()) }

    /// Zed's Icon renders named and asset icons; GPUI handles user supplied SVG bytes.
    pub fn view(&self) -> AnyElement {
        match self {
            Self::Zed(name) => Icon::new(*name).into_any_element(),
            Self::Asset(path) => Icon::from_path(path.clone()).into_any_element(),
            Self::Svg(bytes) => svg().data(bytes.as_ref()).into_any_element(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_zed_and_inline_svg_without_gpui_kit() {
        let svg: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M2 2h20v20H2z"/></svg>"#;
        assert!(matches!(BellatrixIcon::svg(svg), BellatrixIcon::Svg(_)));
        let _ = BellatrixIcon::svg(svg).view();
        let _ = BellatrixIcon::zed(IconName::FileTree).view();
    }
}
