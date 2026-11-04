//! Read-only toolbox metadata computed in MDE, never UML policy in the UI.
use crate::{ModelingElementKind, ModelingRelationKind, ModelingTool, StructuralDiagramKind};

#[derive(Clone, Debug, Default)]
pub struct ToolCatalog {
    pub elements: Vec<ElementToolMetadata>,
    pub relations: Vec<RelationToolMetadata>,
    pub diagrams: Vec<DiagramToolMetadata>,
    pub interface_forms: Vec<InterfaceToolMetadata>,
    pub groups: Vec<ToolGroup>,
    pub profiles: std::collections::BTreeMap<String, DiagramToolProfile>,
}
#[derive(Clone, Debug)]
pub struct DiagramToolProfile {
    pub label: String,
    pub available_tools: Vec<ModelingTool>,
    pub unavailable_tools: Vec<UnavailableToolMetadata>,
    pub allows_new_views: bool,
}
#[derive(Clone, Debug)]
pub struct UnavailableToolMetadata {
    pub label: String,
    pub reason: String,
}
#[derive(Clone, Debug)]
pub struct ToolGroup { pub name: String, pub tools: Vec<ModelingTool> }
#[derive(Clone, Debug)]
pub struct ElementToolMetadata {
    pub kind: ModelingElementKind,
    pub label: String,
    pub model_kind: String,
    pub default_name: String,
    pub requires_owner: bool,
    pub requires_nested_owner: bool,
    pub pseudostate_kind: Option<String>,
    pub initial_size: (f64, f64),
    pub sizes_by_diagram: std::collections::BTreeMap<String, (f64, f64)>,
}
#[derive(Clone, Debug)]
pub struct RelationToolMetadata {
    pub kind: ModelingRelationKind,
    pub label: String,
    pub model_kind: String,
    pub endpoint_instruction: String,
    pub uses_occurrence_anchors: bool,
    pub is_visual_only: bool,
    pub message_sort: Option<String>,
    pub message_kind: Option<String>,
}
#[derive(Clone, Debug)]
pub struct DiagramToolMetadata {
    pub kind: StructuralDiagramKind,
    pub uml_di_type: String,
    pub interaction_kind: Option<String>,
    pub context_metaclass: Option<String>,
}
#[derive(Clone, Debug)]
pub struct InterfaceToolMetadata {
    pub tool: ModelingTool,
    pub label: String,
    pub presentation: String,
}
impl ToolCatalog {
    pub fn allows_new_views(&self, diagram_id: &str) -> bool {
        self.profiles.get(diagram_id).is_none_or(|profile| profile.allows_new_views)
    }
    pub fn allows_tool(&self, diagram_id: Option<&str>, tool: ModelingTool) -> bool {
        self.label(tool).is_some()
            && diagram_id.and_then(|id| self.profiles.get(id))
                .is_none_or(|profile| profile.available_tools.contains(&tool))
    }
    pub fn element(&self, kind: ModelingElementKind) -> Option<&ElementToolMetadata> {
        self.elements.iter().find(|entry| entry.kind == kind)
    }
    pub fn relation(&self, kind: ModelingRelationKind) -> Option<&RelationToolMetadata> {
        self.relations.iter().find(|entry| entry.kind == kind)
    }
    pub fn diagram(&self, kind: StructuralDiagramKind) -> Option<&DiagramToolMetadata> {
        self.diagrams.iter().find(|entry| entry.kind == kind)
    }
    pub fn label(&self, tool: ModelingTool) -> Option<&str> {
        match tool {
            ModelingTool::Element(kind) => self.element(kind).map(|entry| entry.label.as_str()),
            ModelingTool::Relation(kind) => self.relation(kind).map(|entry| entry.label.as_str()),
            _ => self.interface_forms.iter().find(|entry| entry.tool == tool).map(|entry| entry.label.as_str()),
        }
    }
}
