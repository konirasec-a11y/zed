use crate::assets::icons::{IconCategory, uml_icon};
use crate::elements::uml::{UmlClassifier, UmlFigure, UmlRelationGlyph};
use bellatrix_mde_cordis::ModelingTool;
use bellatrix_mde_cordis::presentation_contract::{FigurePresentation, RelationshipStyle};
use bellatrix_mde_cordis::projection::SemanticIdentity;
use gpui::SharedString;

#[derive(Clone, Copy)]
pub struct UmlVisualDefinition {
    icon: &'static [u8],
    category: IconCategory,
}

impl UmlVisualDefinition {
    pub fn for_element(kind: &str, identity: &SemanticIdentity) -> Self {
        Self::for_kind(
            kind,
            identity
                .metaclass
                .as_deref()
                .unwrap_or(&identity.type_label),
            identity.relationship,
        )
    }

    pub fn for_kind(kind: &str, type_label: &str, relationship: bool) -> Self {
        let name = match kind {
            "mdj.ERDDataModel" => "Package",
            "mdj.ERDEntity" => "Class",
            "mdj.ERDColumn" => "Attribute",
            "mdj.ERDRelationship" | "mdj.ERDRelationshipEnd" => "Association",
            "rust.module" => "Package",
            "rust.struct" => "Class",
            "rust.enum" => "Enumeration",
            "rust.trait" => "Interface",
            "rust.function" | "rust.method" => "Operation",
            "rust.field" => "Attribute",
            "depends_on" => "Dependency",
            "uml.final_node" => "ActivityFinalNode",
            "uml.nary_association" => "NaryAssociationNode",
            "view.association_class_attachment" => "AssociationClassAttachment",
            _ => kind
                .strip_prefix("uml.")
                .or_else(|| kind.strip_prefix("mdj."))
                .unwrap_or(type_label),
        };
        let (icon, category) = uml_icon(name, relationship);
        Self { icon, category }
    }

    pub fn for_tool(tool: ModelingTool) -> Self {
        // Tools create semantic types; presentation variants stay on the canvas.
        match tool {
            ModelingTool::Element(kind) => Self::for_kind(kind.model_kind(), "", false),
            ModelingTool::Relation(kind) => Self::for_kind(kind.model_kind(), "", true),
            ModelingTool::ProvidedInterface | ModelingTool::RequiredInterface => {
                Self::for_kind("uml.interface", "", false)
            }
        }
    }

    pub fn for_diagram(kind: &str) -> Self {
        Self::for_kind("", kind, false)
    }

    pub fn for_project() -> Self {
        Self::for_kind("", "Project", false)
    }

    pub fn for_quick_action(id: &str) -> Option<Self> {
        let name = match id {
            "uml.add-parameter" => "Parameter",
            "uml.add-attribute" => "Attribute",
            "uml.add-operation" => "Operation",
            "uml.add-reception" => "Reception",
            "uml.add-note" => "Note",
            "uml.add-constraint" => "Constraint",
            "uml.add-subinterface" | "uml.add-superinterface" => "Interface",
            "uml.add-subclass" | "uml.add-superclass" | "uml.add-realizing-class" => "Class",
            "uml.add-template-parameter" => "TemplateParameter",
            "uml.add-provided-interface" => {
                return Some(Self::for_tool(ModelingTool::ProvidedInterface));
            }
            "uml.add-required-interface" => {
                return Some(Self::for_tool(ModelingTool::RequiredInterface));
            }
            "uml.add-associated-class" => "Association",
            "uml.add-aggregated-class" => "Aggregation",
            "uml.add-composited-class" => "Composition",
            "uml.add-port" => "Port",
            "uml.add-part" => "Part",
            _ => return None,
        };
        Some(Self::for_kind("", name, false))
    }

    pub fn uses_figure(self, presentation: &FigurePresentation) -> bool {
        UmlFigure::supports(presentation)
    }

    pub fn icon(self) -> &'static [u8] {
        self.icon
    }
    fn color(self, panel: gpui::Hsla) -> gpui::Hsla {
        let color = match (self.category, panel.l < 0.5) {
            (IconCategory::Project, true) => 0xf28b67,
            (IconCategory::Project, false) => 0xb6432b,
            (IconCategory::Element, true) => 0xe4be55,
            (IconCategory::Element, false) => 0x896d00,
            (IconCategory::Relation, true) => 0x63b9f2,
            (IconCategory::Relation, false) => 0x0876b2,
            (IconCategory::Diagram, true) => 0x5aca8b,
            (IconCategory::Diagram, false) => 0x00833c,
        };
        gpui::rgb(color).into()
    }

    pub fn control_icon(self, size: ui::IconSize, panel: gpui::Hsla) -> ui::Icon {
        ui::Icon::from_external_svg(String::from_utf8_lossy(self.icon()).into_owned().into())
            .size(size)
            .color(ui::Color::Custom(self.color(panel)))
    }

    pub fn render_icon(self, size: f32, panel: gpui::Hsla) -> gpui::Svg {
        use gpui::{prelude::*, px, svg};
        svg()
            .data(self.icon())
            .size(px(size))
            .flex_none()
            .text_color(self.color(panel))
    }

    // The projection, not the semantic icon, decides occurrence-specific forms
    // such as an Interface rectangle versus a provided/required lollipop.
    pub fn figure(
        self,
        presentation: FigurePresentation,
        name: impl Into<SharedString>,
    ) -> UmlFigure {
        UmlFigure::new(presentation, name)
    }
    pub fn classifier(self, name: impl Into<SharedString>) -> UmlClassifier {
        UmlClassifier::new(name)
    }
    pub fn relationship(
        self,
        style: RelationshipStyle,
        width: f32,
        height: f32,
        from: (f32, f32),
        to: (f32, f32),
    ) -> UmlRelationGlyph {
        UmlRelationGlyph::new(style, width, height, from, to)
    }
}
