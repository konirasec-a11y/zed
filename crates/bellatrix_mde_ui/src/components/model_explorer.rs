pub(crate) use crate::view_model::explorer::*;

#[derive(Clone)]
pub(crate) enum ExplorerMoveTarget {
    Element(String),
    Diagram(String),
}

#[derive(Clone)]
pub(crate) struct ExplorerDiagramDrag {
    pub project_id: bellatrix_mde_cordis::project::ProjectId,
    pub diagram_id: String,
    pub revision: u64,
    pub label: String,
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
    pub project_id: bellatrix_mde_cordis::project::ProjectId,
    pub element_id: String,
    pub target_diagram_id: Option<String>,
    pub source_view_id: Option<String>,
    pub revision: u64,
    pub label: String,
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
    crate::elements::uml::UmlVisualDefinition::for_element(kind, identity)
        .render_icon(20.0, palette.panel)
}

pub(crate) fn explorer_diagram_icon(
    kind: &str,
    palette: &crate::pages::mde_page::WorkbenchPalette,
) -> impl gpui::IntoElement {
    crate::elements::uml::UmlVisualDefinition::for_diagram(kind).render_icon(20.0, palette.panel)
}

pub(crate) fn explorer_project_icon(
    palette: &crate::pages::mde_page::WorkbenchPalette,
) -> impl gpui::IntoElement {
    crate::elements::uml::UmlVisualDefinition::for_project().render_icon(20.0, palette.panel)
}
