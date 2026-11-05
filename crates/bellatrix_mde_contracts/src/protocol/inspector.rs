use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickEditControl {
    Input,
    Visibility,
    Button,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickEditAvailability {
    Implemented,
    NeedsDomainContract,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuickEditSide {
    Main,
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuickEditAction {
    pub id: &'static str,
    pub title: &'static str,
    pub control: QuickEditControl,
    pub side: QuickEditSide,
    pub row: u8,
    pub availability: QuickEditAvailability,
    pub shortcut: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyEditor {
    Text,
    Multiline,
    Boolean,
    Number,
    Choice(Vec<String>),
    Reference,
    Stereotype,
    ReadOnly,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InspectorProperty {
    pub path: String,
    pub label: String,
    pub group: String,
    pub editor: PropertyEditor,
    pub value: Value,
    pub editable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectorMember {
    pub id: String,
    pub name: String,
    pub display_label: String,
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectorReferenceChoice {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElementInspector {
    pub element_id: String,
    pub kind: String,
    pub presentation_kind: String,
    pub type_label: String,
    pub name: String,
    pub display_label: String,
    pub quick_actions: Vec<QuickEditAction>,
    pub properties: Vec<InspectorProperty>,
    pub reference_choices_by_path: BTreeMap<String, Vec<InspectorReferenceChoice>>,
    pub attributes: Vec<InspectorMember>,
    pub association_ends: Vec<InspectorMember>,
    pub operations: Vec<InspectorMember>,
    pub parameters: Vec<InspectorMember>,
    pub receptions: Vec<InspectorMember>,
    pub template_parameters: Vec<InspectorMember>,
    pub ports: Vec<InspectorMember>,
    pub parts: Vec<InspectorMember>,
}

impl ElementInspector {
    pub fn reference_choices_for(&self, path: &str) -> &[InspectorReferenceChoice] {
        self.reference_choices_by_path
            .get(path)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
}
