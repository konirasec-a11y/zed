pub(crate) use crate::view_model::explorer::*;

#[derive(Clone)]
pub(crate) enum ExplorerMoveTarget {
    Element(String),
    Diagram(String),
}

#[derive(Clone)]
pub(crate) struct ExplorerDiagramDrag {
    pub project_id: bellatrix_project::ProjectId,
    pub diagram_id: String,
    pub revision: u64,
    pub label: String,
    pub parent_checks: std::rc::Rc<
        std::cell::RefCell<
            Vec<(
                Option<String>,
                Option<bellatrix_mde_cordis::ViewOccurrence>,
                Result<(), String>,
            )>,
        >,
    >,
}

impl gpui::Render for ExplorerDiagramDrag {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::prelude::*;
        let palette = crate::pages::mde_page::WorkbenchPalette::current(cx);
        gpui::div()
            .px_2()
            .py_1()
            .bg(palette.surface)
            .text_color(palette.text)
            .child(format!("Move diagram: {}", self.label))
    }
}

#[derive(Clone)]
pub(crate) struct ExplorerElementDrag {
    pub project_id: bellatrix_project::ProjectId,
    pub element_id: String,
    pub target_diagram_id: Option<String>,
    pub source_view_id: Option<String>,
    pub revision: u64,
    pub label: String,
    pub ownership_checks:
        std::rc::Rc<std::cell::RefCell<std::collections::HashMap<String, Result<(), String>>>>,
}

impl gpui::Render for ExplorerElementDrag {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::prelude::*;
        let palette = crate::pages::mde_page::WorkbenchPalette::current(cx);
        gpui::div()
            .px_2()
            .py_1()
            .bg(palette.surface)
            .text_color(palette.text)
            .child(self.label.clone())
    }
}

pub(crate) fn explorer_view_icon(
    id: &str,
    kind: &str,
    identity: &bellatrix_mde_cordis::projection::SemanticIdentity,
    selected: bool,
    palette: &crate::pages::mde_page::WorkbenchPalette,
) -> impl gpui::IntoElement {
    use gpui::{div, prelude::*};
    div()
        .id(format!("mde-explorer-view-icon-{id}"))
        .flex()
        .items_center()
        .gap_1()
        .tooltip(ui::Tooltip::text("Diagram occurrence"))
        .child(
            ui::Icon::new(ui::IconName::Eye)
                .size(ui::IconSize::Small)
                .color(if selected {
                    ui::Color::Selected
                } else {
                    ui::Color::Muted
                }),
        )
        .child(explorer_element_icon(kind, identity, palette))
}

pub(crate) fn explorer_element_icon(
    kind: &str,
    identity: &bellatrix_mde_cordis::projection::SemanticIdentity,
    palette: &crate::pages::mde_page::WorkbenchPalette,
) -> impl gpui::IntoElement {
    let visual = crate::elements::uml::UmlVisualDefinition::for_element(kind, identity);
    explorer_colored_icon(visual.icon(), visual.category(), palette)
}

pub(crate) fn explorer_diagram_icon(
    kind: &str,
    palette: &crate::pages::mde_page::WorkbenchPalette,
) -> impl gpui::IntoElement {
    use crate::assets::uml::staruml::{ExplorerIconCategory, explorer_glyph};
    let name = match kind {
        "UMLStateMachineDiagram" => "StatechartDiagram",
        "UMLInteractionDiagram" => "SequenceDiagram",
        _ => kind.strip_prefix("UML").unwrap_or(kind),
    };
    let data = explorer_glyph(name)
        .map(|(data, _)| data)
        .unwrap_or_else(|| {
            crate::assets::symbols::glyph(crate::assets::symbols::ShapeKind::Document)
        });
    explorer_colored_icon(data, ExplorerIconCategory::Diagram, palette)
}

pub(crate) fn explorer_project_icon(
    palette: &crate::pages::mde_page::WorkbenchPalette,
) -> impl gpui::IntoElement {
    use crate::assets::uml::staruml::{ExplorerIconCategory, explorer_glyph};
    let data = explorer_glyph("Project")
        .map(|(data, _)| data)
        .unwrap_or_else(|| {
            crate::assets::symbols::glyph(crate::assets::symbols::ShapeKind::Package)
        });
    explorer_colored_icon(data, ExplorerIconCategory::Project, palette)
}

fn explorer_colored_icon(
    data: &'static [u8],
    category: crate::assets::uml::staruml::ExplorerIconCategory,
    palette: &crate::pages::mde_page::WorkbenchPalette,
) -> impl gpui::IntoElement {
    use crate::assets::uml::staruml::ExplorerIconCategory;
    use gpui::{prelude::*, px, rgb, svg};
    // The source colors are adjusted for contrast instead of becoming muted on dark themes.
    let dark = palette.panel.l < 0.5;
    let color = match (category, dark) {
        (ExplorerIconCategory::Project, true) => 0xf28b67,
        (ExplorerIconCategory::Project, false) => 0xb6432b,
        (ExplorerIconCategory::Element, true) => 0xe4be55,
        (ExplorerIconCategory::Element, false) => 0x896d00,
        (ExplorerIconCategory::Relation, true) => 0x63b9f2,
        (ExplorerIconCategory::Relation, false) => 0x0876b2,
        (ExplorerIconCategory::Diagram, true) => 0x5aca8b,
        (ExplorerIconCategory::Diagram, false) => 0x00833c,
    };
    svg()
        .data(data)
        .size(px(20.0))
        .flex_none()
        .text_color(rgb(color))
}
