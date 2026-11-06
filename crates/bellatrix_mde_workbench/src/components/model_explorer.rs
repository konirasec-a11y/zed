pub(crate) use crate::workbench_ui::explorer::*;

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
        let palette = crate::workbench_ui::WorkbenchPalette::current(cx);
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
        let palette = crate::workbench_ui::WorkbenchPalette::current(cx);
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
    palette: &crate::workbench_ui::WorkbenchPalette,
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

pub(crate) fn element_glyph(kind: &str, type_label: &str) -> bellatrix_ui_symbols::ShapeKind {
    use bellatrix_ui_symbols::ShapeKind;
    // Member and relationship symbols are metaclass based, never inferred from names.
    match kind {
        "uml.composition" => return ShapeKind::Composition,
        "uml.aggregation" => return ShapeKind::Aggregate,
        _ => {}
    }
    match type_label {
        "Attribute" | "Property" => return ShapeKind::Aggregate,
        "Operation" => return ShapeKind::Process,
        "Reception" => return ShapeKind::AcceptEvent,
        "Parameter" => return ShapeKind::Input,
        "Constraint" => return ShapeKind::Note,
        "Generalization" => return ShapeKind::Inheritance,
        "Frame" => return ShapeKind::Boundary,
        "Rectangle" => return ShapeKind::Process,
        "RoundRect" | "RoundedRectangle" => return ShapeKind::Topic,
        "Ellipse" => return ShapeKind::Usecase,
        _ => {}
    }
    let label = if kind == "rust.module" {
        "Package"
    } else {
        type_label
    };
    bellatrix_ui_symbols::groups(bellatrix_ui_symbols::Family::Uml)
        .iter()
        .flat_map(|group| group.items)
        .find(|symbol| symbol.label.eq_ignore_ascii_case(label))
        .map(|symbol| symbol.kind)
        .unwrap_or(ShapeKind::Document)
}

pub(crate) fn explorer_element_icon(
    kind: &str,
    identity: &bellatrix_mde_cordis::projection::SemanticIdentity,
    palette: &crate::workbench_ui::WorkbenchPalette,
) -> impl gpui::IntoElement {
    use bellatrix_ui_symbols::staruml::{ExplorerIconCategory, explorer_glyph};
    let imported_name = kind.strip_prefix("mdj.").map(|kind| {
        let kind = kind.strip_prefix("UML").unwrap_or(kind);
        kind.strip_suffix("View").unwrap_or(kind)
    });
    let name = match kind {
        "mdj.ERDDataModel" => "Package",
        "mdj.ERDEntity" => "Class",
        "mdj.ERDColumn" => "Attribute",
        "mdj.ERDRelationship" | "mdj.ERDRelationshipEnd" => "Association",
        "uml.attribute" | "uml.property" | "rust.field" => "Attribute",
        "uml.part" => "Part",
        "depends_on" => "Dependency",
        "uml.composition" => "Composition",
        "uml.aggregation" => "Aggregation",
        "uml.nary_association" => "NaryAssociationNode",
        "uml.instance_specification" => "Object",
        "uml.comment" => "Note",
        "uml.opaque_action" | "uml.call_behavior_action" | "uml.call_operation_action" => "Action",
        "uml.final_node" => "ActivityFinalNode",
        "uml.initial_node" => "InitialNode",
        "uml.decision_node" => "DecisionNode",
        "uml.merge_node" => "MergeNode",
        "uml.fork_node" => "ForkNode",
        "uml.join_node" => "JoinNode",
        "uml.message" => "Message",
        "rust.module" => "Package",
        "rust.struct" => "Class",
        "rust.enum" => "Enumeration",
        "rust.trait" => "Interface",
        "rust.function" | "rust.method" => "Operation",
        _ => imported_name.unwrap_or_else(|| {
            identity
                .metaclass
                .as_deref()
                .unwrap_or(&identity.type_label)
        }),
    };
    // These are presentation aliases only; opaque source objects keep their
    // imported identity and never gain canonical UML editing capabilities.
    let name = match name {
        "Label" | "NodeLabel" | "EdgeLabel" => "Text",
        "RoundRect" => "RoundedRectangle",
        _ => name,
    };
    let (data, category) = explorer_glyph(name).unwrap_or_else(|| {
        let category = if identity.relationship {
            ExplorerIconCategory::Relation
        } else {
            ExplorerIconCategory::Element
        };
        (
            bellatrix_ui_symbols::glyph(element_glyph(kind, name)),
            category,
        )
    });
    explorer_colored_icon(data, category, palette)
}

pub(crate) fn explorer_diagram_icon(
    kind: &str,
    palette: &crate::workbench_ui::WorkbenchPalette,
) -> impl gpui::IntoElement {
    use bellatrix_ui_symbols::staruml::{ExplorerIconCategory, explorer_glyph};
    let name = match kind {
        "UMLStateMachineDiagram" => "StatechartDiagram",
        "UMLInteractionDiagram" => "SequenceDiagram",
        _ => kind.strip_prefix("UML").unwrap_or(kind),
    };
    let data = explorer_glyph(name)
        .map(|(data, _)| data)
        .unwrap_or_else(|| bellatrix_ui_symbols::glyph(bellatrix_ui_symbols::ShapeKind::Document));
    explorer_colored_icon(data, ExplorerIconCategory::Diagram, palette)
}

pub(crate) fn explorer_project_icon(
    palette: &crate::workbench_ui::WorkbenchPalette,
) -> impl gpui::IntoElement {
    use bellatrix_ui_symbols::staruml::{ExplorerIconCategory, explorer_glyph};
    let data = explorer_glyph("Project")
        .map(|(data, _)| data)
        .unwrap_or_else(|| bellatrix_ui_symbols::glyph(bellatrix_ui_symbols::ShapeKind::Package));
    explorer_colored_icon(data, ExplorerIconCategory::Project, palette)
}

fn explorer_colored_icon(
    data: &'static [u8],
    category: bellatrix_ui_symbols::staruml::ExplorerIconCategory,
    palette: &crate::workbench_ui::WorkbenchPalette,
) -> impl gpui::IntoElement {
    use bellatrix_ui_symbols::staruml::ExplorerIconCategory;
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
