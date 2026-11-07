use crate::contracts::protocol::command::{
    ModelingElementKind, ModelingMemberKind, StructuralDiagramKind, ViewOccurrence,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct ExplorerProjection {
    pub added: Vec<ExplorerRow>,
    pub name: Vec<ExplorerRow>,
    pub member_creation: BTreeMap<String, Vec<(ModelingMemberKind, &'static str)>>,
    pub contextual_diagrams: BTreeMap<String, Vec<StructuralDiagramKind>>,
    pub model_creation: BTreeMap<String, Vec<ModelingElementKind>>,
    pub project_model_creation: Vec<ModelingElementKind>,
    pub project_diagrams: Vec<StructuralDiagramKind>,
    pub parent_occurrences: BTreeMap<String, ViewOccurrence>,
    pub type_labels: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualComponent {
    pub diagram_id: String,
    pub view_id: String,
    pub subject_id: Option<String>,
    pub parent_view_id: Option<String>,
    pub label: String,
    pub kind: String,
}

#[derive(Clone, Debug)]
pub struct ExplorerRow {
    pub key: String,
    pub diagram_context: Option<String>,
    pub element_id: String,
    pub label: String,
    pub kind: String,
    pub depth: usize,
    pub diagram: bool,
    pub view: Option<VisualComponent>,
}
