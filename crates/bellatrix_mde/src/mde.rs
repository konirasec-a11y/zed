//! Single MDE domain: persisted objects, UML semantics, commands and validation.
//! Adapters own transport/presentation, never another implementation of the model.
pub const MODELING_NAMESPACE: &str = "bellatrix.mde";

mod model;
pub(crate) use model::wire;
pub use model::*;

pub mod commands;
mod generation;
mod graph;
mod hierarchy;
mod index;
mod inspector;
mod ownership;
mod path;
mod plan;
mod presentation;
mod instance_presentation;
pub use instance_presentation::{format_instance_heading, collect_instance_slot_labels};
mod publish;
mod quick_edit;
mod relationships;
mod relationship_notation;
mod scope;
mod structural_diagrams;
mod structural_rules;
mod templates;
pub mod uml;
mod uml_geometry;
mod uml_notation;
mod uml_types;
pub use templates::{
    template_parameter_label, template_parameter_label_with_references,
    template_parametered_element, template_parameters,
};

pub use generation::{
    GenerationGraphError, GenerationModel, GenerationReference, GenerationUnit,
    analyze_generation_graph,
};
pub use graph::{Graph, GraphError};
pub use hierarchy::{HierarchyRow, explorer_parent_map, project_hierarchy};
pub use index::{GraphIndex, GraphLookupError};
pub use inspector::{
    ElementInspector, InspectorMember, InspectorProperty, InspectorReferenceChoice, PropertyEditor,
    inspect_element, set_inspected_property,
};
pub use path::{PathError, physical_path, validate_segment};
pub use plan::{Artifact, Diagnostic, Plan, PlanError};
pub use presentation::{
    ClassifierPresentation, ClassifierPresentationOptions, MemberPresentation, SemanticIdentity,
    classifier_presentation, element_display_label, element_display_label_with_references,
    format_element_label_for_view, member_presentation, semantic_identity,
};
pub use publish::{publish_new, publish_replace};
pub use quick_edit::{
    QuickEditAction, QuickEditAvailability, QuickEditControl, QuickEditSide, quick_edit_for,
};
pub use relationships::{
    ModelingRelationKind, QuickRelationRule, RelationshipEndpointLabels, RelationshipGraph,
    RelationshipMarker, RelationshipPresentation, RelationshipStyle, projected_relationship_paths,
    quick_relation_rule, relation_kind_for_creation, relationship_presentation,
    relationship_style_for_notation, reroute_relationships, route as route_relationship,
};
pub use scope::{Scope, ScopeError, select_elements};
pub use structural_diagrams::{
    RootMetaclass, StructuralDiagnostic, StructuralDiagramKind, StructuralSeverity,
    element_root_kind, relationship_root_kind, validate_structural_diagram,
    validate_structural_model,
};
pub use uml_geometry::{
    ClassifierContents, FoldedNoteLayout, FrameHeadingLayout, InterfaceLayout, PortLabelLayout,
    TemplateSignatureLayout, classifier_contents, folded_note_layout, frame_heading_layout,
    interface_layout, port_label_layout, template_signature_layout,
};
pub use uml_types::{UmlCapability, UmlQuickCreate, UmlType};

pub use uml_notation::{UmlHeader, UmlNotation, resolve_element_notation, resolve_pseudostate_notation, uml_header, uml_notation};

mod visual_draft_routing;
pub use visual_draft_routing::reroute_visual_drafts;
