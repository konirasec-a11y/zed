//! Single MDE domain: persisted objects, UML semantics, commands and validation.
//! Adapters own transport/presentation, never another implementation of the model.
mod model;
pub use model::*;
pub(crate) use model::wire;

mod graph;
mod inspector;
mod quick_edit;
mod index;
mod ownership;
mod path;
mod plan;
mod publish;
mod scope;
mod uml_types;
mod uml_geometry;
mod relationships;
mod structural_diagrams;
mod generation;
mod hierarchy;
pub mod uml;
pub mod commands;

pub use graph::{Graph, GraphError};
pub use hierarchy::{HierarchyRow, project_hierarchy};
pub use structural_diagrams::{
    RootMetaclass, StructuralDiagnostic, StructuralDiagramKind,
    StructuralSeverity, element_root_kind, relationship_root_kind, validate_structural_diagram,
};
pub use generation::{GenerationModel, GenerationUnit, GenerationReference, GenerationGraphError, analyze_generation_graph};
pub use inspector::{ElementInspector, InspectorMember, InspectorProperty, PropertyEditor, inspect_element, set_inspected_property};
pub use quick_edit::{QuickEditAction, QuickEditAvailability, QuickEditControl, QuickEditSide, quick_edit_for};
pub use index::{GraphIndex, GraphLookupError};
pub use path::{PathError, physical_path, validate_segment};
pub use plan::{Artifact, Diagnostic, Plan, PlanError};
pub use publish::{publish_new, publish_replace};
pub use scope::{Scope, ScopeError, select_elements};
pub use uml_types::{UmlCapability, UmlQuickCommand, UmlType};
pub use uml_geometry::{ClassifierContents, InterfaceLayout, FoldedNoteLayout, classifier_contents, interface_layout, folded_note_layout};
pub use relationships::{
    ModelingRelationKind, RelationshipGraph, RelationshipPresentation,
    RelationshipMarker, RelationshipStyle, QuickRelationRule,
    relationship_presentation, relationship_style_for_notation,
    relation_kind_for_creation, quick_relation_rule,
    route as route_relationship, projected_relationship_paths, reroute_relationships,
};
