//! Single MDE domain: persisted objects, UML semantics, commands and validation.
//! Adapters own transport/presentation, never another implementation of the model.
pub const MODELING_NAMESPACE: &str = "bellatrix.mde";

#[path = "model/model.rs"]
mod model;
pub(crate) use model::wire;
pub use model::*;

#[path = "application/commands/executor.rs"]
mod command_executor;
#[path = "application/commands.rs"]
pub mod commands;
#[path = "application/diagram/operations.rs"]
mod diagram_operations;
#[path = "view-model/diagram/diagram_operations.rs"]
mod diagram_operation_policy;
#[path = "KERNEL/compiler/emission/construction_script.rs"]
mod construction_script;
#[path = "observability/execution_trace.rs"]
mod execution_trace;
pub use execution_trace::{ExecutionEvent, ExecutionTrace, TraceOutcome, TracePhase, execution_trace_jsonl, validate_structural_diagram_traced, validate_structural_model_traced};
#[path = "KERNEL/compiler/analysis/generation_graph.rs"]
mod generation;
#[path = "KERNEL/graph/graph.rs"]
mod graph;
#[path = "view-model/hierarchy/hierarchy.rs"]
mod hierarchy;
#[path = "view-model/hierarchy/visual_consistency.rs"]
mod visual_consistency;
pub use visual_consistency::{VisualComponent, visual_components};
#[path = "KERNEL/graph/index.rs"]
mod index;
#[path = "view-model/inspector/inspector.rs"]
mod inspector;
#[path = "KERNEL/compiler/operations/operation_graph.rs"]
mod operation_graph;
#[path = "KERNEL/I_O/generated_files.rs"]
mod generated_files;
#[path = "KERNEL/compiler/artifacts/path.rs"]
mod path;
#[path = "KERNEL/compiler/artifacts/plan.rs"]
mod plan;
#[path = "view-model/notation/presentation.rs"]
mod presentation;
#[path = "view-model/notation/instance_presentation.rs"]
mod instance_presentation;
#[path = "view-model/notation/interaction_presentation.rs"]
mod interaction_presentation;
pub use interaction_presentation::format_operand_guard;
pub use instance_presentation::{format_instance_heading, collect_instance_slot_labels};
#[path = "KERNEL/I_O/publish.rs"]
mod publish;
#[path = "view-model/authoring/quick_edit.rs"]
mod quick_edit;
#[path = "view-model/diagram/relationships.rs"]
mod relationships;
#[path = "view-model/authoring/relationship_tools.rs"]
mod relationship_tools;
#[path = "view-model/notation/relationship_notation.rs"]
mod relationship_notation;
pub use relationship_notation::{format_relationship_label, resolve_relationship_style};
#[path = "KERNEL/graph/scope.rs"]
mod scope;
#[path = "KERNEL/uml/validation/structural_diagrams.rs"]
mod structural_diagrams;
#[path = "KERNEL/uml/validation/structural_rules.rs"]
mod structural_rules;
#[path = "KERNEL/uml/semantics/templates.rs"]
mod templates;
#[path = "KERNEL/uml/uml.rs"]
pub mod uml;
#[path = "view-model/diagram/uml_geometry.rs"]
mod uml_geometry;
#[path = "view-model/notation/uml_notation.rs"]
mod uml_notation;
#[path = "KERNEL/uml/capabilities.rs"]
mod uml_types;
pub use templates::{
    template_parametered_element, template_parameters,
};

pub use construction_script::{ConstructionScriptError, python_construction_script};
pub use generation::{
    GenerationGraphError, GenerationModel, GenerationReference, GenerationUnit,
    analyze_generation_graph,
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

#[path = "view-model/diagram/visual_draft_routing.rs"]
mod visual_draft_routing;
pub use visual_draft_routing::{reroute_visual_drafts, resolve_visual_center};

#[path = "application/edits/inspector_edit.rs"]
mod inspector_edit;
pub use inspector_edit::set_inspected_property;
#[path = "view-model/notation/template_presentation.rs"]
mod template_presentation;

#[path = "application/edits/property_edit.rs"]
mod property_edit;
pub use template_presentation::{template_parameter_label, template_parameter_label_with_references};

#[path = "application/edits/explorer_placement.rs"]
mod explorer_placement;
pub use explorer_placement::explorer_parent_map;

#[path = "KERNEL/compiler/artifacts/ownership_manifest.rs"]
mod ownership_manifest;

#[path = "kernel-api/validation.rs"]
mod kernel_api_validation;
pub use kernel_api_validation::{validate_graph_model, validate_uml_model};

#[path = "kernel-api/contract.rs"]
mod kernel_api_contract;
#[path = "kernel-api/graph.rs"]
mod kernel_api_graph;
pub use kernel_api_graph::{reachable_graph_nodes, shortest_graph_path};

#[path = "kernel-api/document.rs"]
mod kernel_api_document;
pub use kernel_api_document::decode_model_document;

#[path = "kernel-api/generation.rs"]
mod kernel_api_generation;
pub use kernel_api_generation::analyze_generation_dependencies;
