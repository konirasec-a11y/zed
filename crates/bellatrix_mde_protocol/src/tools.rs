//! Read-only toolbox metadata computed in MDE, never UML policy in the UI.
use crate::{ModelingElementKind, ModelingRelationKind, ModelingTool, StructuralDiagramKind};

#[derive(Clone, Debug, Default)]
pub struct ToolCatalog {
    pub elements: Vec<ElementToolMetadata>,
    pub relations: Vec<RelationToolMetadata>,
    pub diagrams: Vec<DiagramToolMetadata>,
    pub interface_forms: Vec<InterfaceToolMetadata>,
    pub groups: Vec<ToolGroup>,
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
