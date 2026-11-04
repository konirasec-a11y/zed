use crate::command::*;
use crate::inspector::{ElementInspector, QuickEditAction};
use crate::presentation::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct ModelingProjection {
    pub tools: crate::ToolCatalog,
    pub root_model: Option<String>,
    pub elements: Vec<Element>,
    pub relations: Vec<Relation>,
    pub diagrams: Vec<Diagram>,
    pub hierarchy: BTreeMap<String, HierarchyPlacement>,
    pub composed_subjects: BTreeSet<String>,
    pub inspections: BTreeMap<String, ElementInspector>,
    pub identities: BTreeMap<String, SemanticIdentity>,
    pub hierarchy_rows: Vec<HierarchyRow>,
    pub explorer: crate::explorer::ExplorerProjection,
}
impl ModelingProjection {
    pub fn inspect_element(&self, id: &str) -> Option<ElementInspector> {
        self.inspections.get(id).cloned()
    }
    pub fn identity(&self, kind: &str) -> SemanticIdentity {
        self.identities
            .get(kind)
            .cloned()
            .unwrap_or_else(|| SemanticIdentity {
                type_label: kind.to_owned(),
                metaclass: None,
                relationship: false,
            })
    }
}
#[derive(Clone, Debug)]
pub struct SemanticIdentity {
    pub type_label: String,
    pub metaclass: Option<String>,
    pub relationship: bool,
}
#[derive(Clone, Debug)]
pub struct Element {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub display_label: String,
    pub name_expression: String,
    pub visibility: Option<String>,
    pub quick_actions: Vec<QuickEditAction>,
    pub quick_create: Vec<UmlQuickCreate>,
    pub is_classifier: bool,
    pub is_comment: bool,
    pub template_parametered_id: Option<String>,
    pub members: Vec<String>,
    pub template_parameters: Vec<String>,
    pub template_label: String,
    pub structured_classifier: bool,
    pub member: MemberPresentation,
    pub minimum_size: (f64, f64),
    pub minimum_header_height: f64,
    pub minimum_interface_height: Option<f64>,
}
#[derive(Clone, Debug)]
pub struct Relation {
    pub id: String,
    pub kind: String,
    pub from: String,
    pub to: String,
    pub presentation: RelationshipPresentation,
    pub source_role: String,
    pub target_role: String,
}
#[derive(Clone, Debug)]
pub struct HierarchyPlacement {
    pub parent_id: Option<String>,
    pub occurrence: Option<ViewOccurrence>,
}
#[derive(Clone, Debug)]
pub struct HierarchyRow {
    pub element_id: String,
    pub label: String,
    pub kind: String,
    pub depth: usize,
}
#[derive(Clone, Debug)]
pub struct Diagram {
    pub id: String,
    pub name: String,
    pub type_label: String,
    pub owner_label: Option<String>,
    pub width: f64,
    pub height: f64,
    pub views: Vec<View>,
    pub uml_di_type: String,
    pub interaction_kind: Option<String>,
    pub model_elements: Vec<String>,
    pub kind: Option<StructuralDiagramKind>,
}
impl Diagram {
    pub fn all_views(&self) -> impl Iterator<Item = &View> {
        let mut pending = self.views.iter().rev().collect::<Vec<_>>();
        std::iter::from_fn(move || {
            let view = pending.pop()?;
            pending.extend(view.children.iter().rev());
            Some(view)
        })
    }
}
#[derive(Clone, Debug)]
pub struct View {
    pub id: String,
    pub subject: Option<String>,
    pub draft: Option<VisualDraftSummary>,
    pub endpoints: Option<ViewEndpoints>,
    pub timing: Option<TimingPresentation>,
    pub label: Option<String>,
    pub rendered_label: RenderedLabel,
    pub interface_icon: Option<InterfaceIconProjection>,
    pub entity_box: Option<EntityBoxProjection>,
    pub geometry: Geometry,
    pub children: Vec<View>,
    pub presentation: ViewPresentation,
    pub style: Style,
    pub z_index: i32,
    pub render_style: RenderStyle,
    pub figure: FigurePresentation,
    pub classifier: ClassifierPresentation,
    pub display_label: String,
    pub member_labels: BTreeMap<String, String>,
    pub relationship: Option<ResolvedRelationship>,
    pub routed_geometry: Option<Geometry>,
}
#[derive(Clone, Debug)]
pub struct VisualDraftSummary {
    pub notation_kind: String,
    pub parent_view_id: Option<String>,
}
#[derive(Clone, Debug)]
pub struct ViewPresentation {
    pub kind: Option<String>,
    pub internal: bool,
    pub name_label: Option<String>,
    pub divider: bool,
    pub has_source: bool,
    pub visible: bool,
    pub style_inheritance: Option<StyleInheritance>,
}

#[derive(Clone, Debug, Default)]
pub struct RenderedLabel {
    pub bound: bool,
    pub visible: Option<bool>,
    pub text: Option<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct EntityBoxProjection {
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct InterfaceIconProjection {
    pub glyphs: Vec<InterfaceIconGlyph>,
    pub label_bounds: [f64; 4],
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug)]
pub enum InterfaceIconGlyph {
    Ball {
        center: crate::command::Point,
        radius: f64,
    },
    Socket {
        center: crate::command::Point,
        radius: f64,
        start_angle: f64,
        sweep_angle: f64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StyleInheritance {
    pub source: StyleInheritanceSource,
    pub properties: Vec<InheritedStyleProperty>,
    pub status: StyleInheritanceStatus,
    pub origins: BTreeMap<InheritedStyleProperty, String>,
    pub diagnostics: BTreeMap<InheritedStyleProperty, StyleInheritanceDiagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StyleInheritanceSource {
    Missing,
    Null,
    Reference(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StyleInheritanceStatus {
    Inactive,
    Resolved,
    Unresolved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum InheritedStyleProperty {
    Fill,
    Stroke,
    TextColor,
    FontFamily,
    FontSize,
    FontWeight,
    FontItalic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StyleInheritanceDiagnostic {
    MissingSource { source_id: String },
    NonViewSource { source_id: String },
    Cycle { view_id: String },
}
#[derive(Clone, Debug)]
pub struct ResolvedRelationship {
    pub kind: String,
    pub presentation: RelationshipPresentation,
    pub style: RelationshipStyle,
    pub markers: Option<(EndpointMarkerComposition, EndpointMarkerComposition)>,
    pub diagnostics: Vec<String>,
    pub communication_message: bool,
}
