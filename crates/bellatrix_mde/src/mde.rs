//! Single MDE domain: persisted objects, UML semantics, commands and validation.
//! Adapters own transport/presentation, never another implementation of the model.
pub(crate) use bellatrix_mde_contracts::MODELING_NAMESPACE;

pub(crate) use bellatrix_mde_contracts::model as contracts;
pub(crate) use bellatrix_mde_contracts::model::*;
#[path = "persistence/io.rs"]
mod io;
#[path = "persistence/mdj.rs"]
mod mdj;
#[path = "persistence/mdj_conversion.rs"]
mod mdj_conversion;
#[path = "persistence/wire.rs"]
pub(crate) mod wire;
pub(crate) use io::{Format, ModelIoError, decode, encode};

#[path = "application/commands/executor.rs"]
mod command_executor;
#[path = "application/commands.rs"]
pub(crate) mod commands;
#[path = "KERNEL/compiler/emission/construction_script.rs"]
mod construction_script;
#[path = "view-model/diagram/diagram_operations.rs"]
mod diagram_operation_policy;
#[path = "application/diagram/operations.rs"]
mod diagram_operations;
#[path = "observability/execution_trace.rs"]
mod execution_trace;
pub(crate) use execution_trace::{
    ExecutionTrace, TraceOutcome, TracePhase, execution_trace_jsonl,
    validate_structural_diagram_traced, validate_structural_model_traced,
};
#[path = "KERNEL/compiler/analysis/generation_graph.rs"]
mod generation;
#[path = "KERNEL/graph/graph.rs"]
mod graph;
#[path = "view-model/hierarchy/hierarchy.rs"]
mod hierarchy;
#[path = "KERNEL/compiler/projection/rust_emission.rs"]
mod kernel_rust_emission;
#[path = "KERNEL/compiler/projection/uml_rust_struct.rs"]
mod kernel_uml_rust_struct;
#[path = "view-model/hierarchy/visual_consistency.rs"]
mod visual_consistency;
pub(crate) use visual_consistency::visual_components;
#[path = "KERNEL/I_O/generated_files.rs"]
mod generated_files;
#[path = "KERNEL/graph/index.rs"]
mod index;
#[path = "view-model/inspector/inspector.rs"]
mod inspector;
#[path = "view-model/notation/instance_presentation.rs"]
mod instance_presentation;
#[path = "view-model/notation/interaction_presentation.rs"]
mod interaction_presentation;
#[path = "KERNEL/compiler/operations/operation_graph.rs"]
mod operation_graph;
#[path = "KERNEL/compiler/artifacts/path.rs"]
mod path;
#[path = "KERNEL/compiler/artifacts/plan.rs"]
mod plan;
#[path = "view-model/notation/presentation.rs"]
mod presentation;
pub(crate) use instance_presentation::{collect_instance_slot_labels, format_instance_heading};
pub(crate) use interaction_presentation::format_operand_guard;
#[path = "view-model/notation/endpoint_markers.rs"]
mod endpoint_markers;
#[path = "KERNEL/I_O/publish.rs"]
mod publish;
#[path = "view-model/authoring/quick_edit.rs"]
mod quick_edit;
#[path = "view-model/authoring/relationship_tools.rs"]
mod relationship_tools;
#[path = "view-model/diagram/relationships.rs"]
mod relationships;
pub(crate) use endpoint_markers::resolve_relationship_view;
#[path = "view-model/notation/relationship_notation.rs"]
mod relationship_notation;
pub(crate) use relationship_notation::{format_relationship_label, resolve_relationship_style};
#[path = "KERNEL/graph/scope.rs"]
mod scope;
#[path = "KERNEL/uml/validation/structural_diagrams.rs"]
mod structural_diagrams;
#[path = "KERNEL/uml/validation/structural_rules.rs"]
mod structural_rules;
#[path = "KERNEL/uml/semantics/templates.rs"]
mod templates;
#[path = "KERNEL/uml/uml.rs"]
pub(crate) mod uml;
#[path = "view-model/diagram/uml_geometry.rs"]
mod uml_geometry;
#[path = "view-model/notation/uml_notation.rs"]
mod uml_notation;
#[path = "KERNEL/uml/capabilities.rs"]
mod uml_types;
pub(crate) use templates::{template_parametered_element, template_parameters};

pub(crate) use construction_script::{ConstructionScriptError, python_construction_script};
pub(crate) use generation::{GenerationGraphError, analyze_generation_graph};
pub(crate) use graph::{Graph, GraphError};
pub(crate) use hierarchy::project_hierarchy;
pub(crate) use index::GraphIndex;
pub(crate) use inspector::{InspectorMember, InspectorProperty, PropertyEditor, inspect_element};
pub(crate) use operation_graph::{
    InputPort, Operation, OperationDefinition, OperationGraph, OperationGraphError,
    OperationImplementation, OperationValue, OperationValues, PortType,
};
pub(crate) use path::{physical_path, validate_segment};
pub(crate) use plan::{Plan, PlanError};
pub(crate) use presentation::{
    ClassifierPresentationOptions, classifier_presentation, element_display_label,
    element_display_label_with_references, format_element_label_for_view, member_presentation,
    semantic_identity,
};
pub(crate) use publish::{publish_new, publish_replace};
pub(crate) use quick_edit::{QuickEditAvailability, QuickEditControl, QuickEditSide, quick_edit_for};
pub(crate) use relationships::{
    ModelingRelationKind, RelationshipEndpointLabels, RelationshipGraph, RelationshipMarker,
    RelationshipPresentation, RelationshipStyle, quick_relation_rule, relation_kind_for_creation,
    relationship_presentation,
    relationship_style_for_notation, reroute_relationships, route as route_relationship,
};
pub(crate) use scope::{Scope, select_elements};
pub(crate) use structural_diagrams::{
    RootMetaclass, StructuralDiagnostic, StructuralDiagramKind, StructuralSeverity,
    element_root_kind, relationship_root_kind, validate_structural_diagram,
    validate_structural_model,
};
pub(crate) use uml_geometry::{
    ClassifierContents, InterfaceLayout, classifier_contents, interface_view_layout,
};
pub(crate) use uml_types::{UmlCapability, UmlQuickCreate, UmlType};

pub(crate) use uml_notation::{
    StereotypePresentation, UmlNotation, resolve_element_notation, resolve_pseudostate_notation,
    stereotype_presentation, uml_header, uml_notation,
};

#[path = "view-model/diagram/visual_draft_routing.rs"]
mod visual_draft_routing;
pub(crate) use visual_draft_routing::{reroute_visual_drafts, resolve_visual_center};

#[path = "application/edits/inspector_edit.rs"]
mod inspector_edit;
pub(crate) use inspector_edit::set_inspected_property;
#[path = "view-model/notation/template_presentation.rs"]
mod template_presentation;

#[path = "application/edits/property_edit.rs"]
mod property_edit;
pub(crate) use template_presentation::template_parameter_label_with_references;

#[path = "application/edits/hierarchy_placement.rs"]
mod hierarchy_placement;
pub(crate) use hierarchy_placement::{validate_diagram_occurrence, validate_model_hierarchy};

#[path = "KERNEL/compiler/artifacts/ownership_manifest.rs"]
mod ownership_manifest;

mod api_contract;

#[path = "kernel-api/validation.rs"]
mod kernel_api_validation;
pub use kernel_api_validation::{validate_graph_model, validate_uml_diagram, validate_uml_model};

#[path = "kernel-api/contract.rs"]
mod kernel_api_contract;
#[path = "kernel-api/graph.rs"]
mod kernel_api_graph;
pub use kernel_api_graph::{reachable_graph_nodes, shortest_graph_path};

#[path = "kernel-api/generation.rs"]
mod kernel_api_generation;
pub use kernel_api_generation::{analyze_generation_dependencies, generate_rust_struct};

#[path = "mde-api/document.rs"]
mod mde_api_document;
pub use mde_api_document::{
    convert_model_document, decode_model_document, encode_model_document, encode_model_section,
    prepare_model_document, validate_model_section,
};

#[path = "mde-api/interchange.rs"]
mod mde_api_interchange;
pub use mde_api_interchange::{export_mdj_document, export_mdj_section, import_mdj_document};

#[path = "mde-api/presentation.rs"]
mod mde_api_presentation;
pub use mde_api_presentation::project_documentation;

#[path = "mde-api/generation.rs"]
mod mde_api_generation;
pub use mde_api_generation::project_rust_generation;

#[path = "mde-api/artifacts.rs"]
mod mde_api_artifacts;
pub use mde_api_artifacts::{prepare_artifact_publication, publish_artifact_manifest};

#[path = "mde-api/headless.rs"]
mod mde_api_headless;
pub use mde_api_headless::report_model_document;

#[path = "mde-api/commands.rs"]
mod mde_api_commands;
pub use mde_api_commands::{apply_model_command, reparent_uml_element};

#[path = "mde-api/projection.rs"]
mod mde_api_projection;
pub use mde_api_projection::project_model_section;
#[path = "mde-api/projection_explorer.rs"]
mod mde_api_projection_explorer;
#[path = "mde-api/projection_inspector.rs"]
mod mde_api_projection_inspector;
#[path = "mde-api/projection_notation.rs"]
mod mde_api_projection_notation;
#[path = "mde-api/projection_tools.rs"]
mod mde_api_projection_tools;

#[path = "mde-api/observability.rs"]
mod mde_api_observability;
pub use mde_api_observability::{MdeExecutionTrace, validate_model_section_diagram};
