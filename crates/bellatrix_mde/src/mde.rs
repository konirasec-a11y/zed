//! Single MDE domain: persisted objects, UML semantics, commands and validation.
//! Adapters own transport/presentation, never another implementation of the model.
pub const MODELING_NAMESPACE: &str = "bellatrix.mde";

#[path = "KERNEL/model.rs"]
mod model;
pub(crate) use model::wire;
pub use model::*;

#[path = "KERNEL/commands.rs"]
pub mod commands;
#[path = "KERNEL/construction_script.rs"]
mod construction_script;
#[path = "KERNEL/execution_trace.rs"]
mod execution_trace;
pub use execution_trace::{ExecutionEvent, ExecutionTrace, TraceOutcome, TracePhase, execution_trace_jsonl, validate_structural_diagram_traced, validate_structural_model_traced};
#[path = "KERNEL/generation.rs"]
mod generation;
#[path = "KERNEL/graph.rs"]
mod graph;
#[path = "view-model/hierarchy.rs"]
mod hierarchy;
#[path = "view-model/visual_consistency.rs"]
mod visual_consistency;
pub use visual_consistency::{VisualComponent, visual_components};
#[path = "KERNEL/index.rs"]
mod index;
#[path = "view-model/inspector.rs"]
mod inspector;
#[path = "KERNEL/operation_graph.rs"]
mod operation_graph;
#[path = "KERNEL/ownership.rs"]
mod ownership;
#[path = "KERNEL/path.rs"]
mod path;
#[path = "KERNEL/plan.rs"]
mod plan;
#[path = "view-model/presentation.rs"]
mod presentation;
#[path = "view-model/instance_presentation.rs"]
mod instance_presentation;
#[path = "view-model/interaction_presentation.rs"]
mod interaction_presentation;
pub use interaction_presentation::format_operand_guard;
pub use instance_presentation::{format_instance_heading, collect_instance_slot_labels};
#[path = "KERNEL/publish.rs"]
mod publish;
#[path = "view-model/quick_edit.rs"]
mod quick_edit;
#[path = "view-model/relationships.rs"]
mod relationships;
#[path = "view-model/relationship_tools.rs"]
mod relationship_tools;
#[path = "view-model/relationship_notation.rs"]
mod relationship_notation;
pub use relationship_notation::{format_relationship_label, resolve_relationship_style};
#[path = "KERNEL/scope.rs"]
mod scope;
#[path = "KERNEL/structural_diagrams.rs"]
mod structural_diagrams;
#[path = "KERNEL/structural_rules.rs"]
mod structural_rules;
#[path = "KERNEL/templates.rs"]
mod templates;
#[path = "KERNEL/uml.rs"]
pub mod uml;
#[path = "view-model/uml_geometry.rs"]
mod uml_geometry;
#[path = "view-model/uml_notation.rs"]
mod uml_notation;
#[path = "KERNEL/uml_types.rs"]
mod uml_types;
pub use templates::{
    template_parametered_element, template_parameters,
};

pub use construction_script::{ConstructionScriptError, python_construction_script, python_construction_script_traced};
pub use generation::{
    GenerationGraphError, GenerationModel, GenerationReference, GenerationUnit,
    analyze_generation_graph, analyze_generation_graph_traced,
};
pub use graph::{Graph, GraphError};
pub use hierarchy::{HierarchyRow, project_hierarchy};
pub use index::{GraphIndex, GraphLookupError};
pub use inspector::{
    ElementInspector, InspectorMember, InspectorProperty, InspectorReferenceChoice, PropertyEditor,
    inspect_element,
};
pub use operation_graph::{InputPort, Operation, OperationDefinition, OperationGraph, OperationGraphError, OperationImplementation, OperationValue, OperationValues, PortType};
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

#[path = "view-model/visual_draft_routing.rs"]
mod visual_draft_routing;
pub use visual_draft_routing::{reroute_visual_drafts, resolve_visual_center};

#[path = "KERNEL/inspector_edit.rs"]
mod inspector_edit;
pub use inspector_edit::set_inspected_property;
#[path = "view-model/template_presentation.rs"]
mod template_presentation;

#[path = "KERNEL/property_edit.rs"]
mod property_edit;
pub use template_presentation::{template_parameter_label, template_parameter_label_with_references};

#[path = "KERNEL/explorer_placement.rs"]
mod explorer_placement;
pub use explorer_placement::explorer_parent_map;
