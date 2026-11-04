use bellatrix_mde_protocol::tools::*;
// Catalog policy is resolved inside MDE; consumers receive only derived metadata.
pub(crate) fn project(diagrams: &[crate::Diagram]) -> ToolCatalog {
    use crate::ModelingRelationKind as R;
    use crate::commands::{ModelingElementKind as E, ModelingTool as T};
    use crate::mde_api_command_conversion as convert;
    let elements = [
        E::Port,
        E::Part,
        E::DeploymentSpecification,
        E::CollaborationUse,
        E::DestructionOccurrenceSpecification,
        E::Continuation,
        E::Class,
        E::Interface,
        E::Component,
        E::Artifact,
        E::Signal,
        E::DataType,
        E::PrimitiveType,
        E::Enumeration,
        E::Frame,
        E::NaryAssociation,
        E::AssociationClass,
        E::Package,
        E::Model,
        E::Subsystem,
        E::Comment,
        E::Template,
        E::Node,
        E::Device,
        E::ExecutionEnvironment,
        E::Collaboration,
        E::InstanceSpecification,
        E::Profile,
        E::Stereotype,
        E::Actor,
        E::UseCase,
        E::Activity,
        E::StateMachine,
        E::Interaction,
        E::OpaqueAction,
        E::InitialNode,
        E::ActivityFinalNode,
        E::FlowFinalNode,
        E::DecisionNode,
        E::MergeNode,
        E::ForkNode,
        E::JoinNode,
        E::CentralBufferNode,
        E::DataStoreNode,
        E::SendSignalAction,
        E::AcceptEventAction,
        E::ActivityPartition,
        E::Region,
        E::State,
        E::Pseudostate,
        E::FinalState,
        E::Lifeline,
        E::BehaviorExecutionSpecification,
        E::InteractionUse,
        E::CombinedFragment,
        E::StateInvariant,
        E::InputPin,
        E::OutputPin,
        E::ValuePin,
        E::InteractionOperand,
        E::Choice,
        E::Junction,
        E::StateFork,
        E::StateJoin,
        E::ShallowHistory,
        E::DeepHistory,
        E::EntryPoint,
        E::ExitPoint,
        E::Terminate,
    ]
    .into_iter()
    .map(|kind| ElementToolMetadata {
        kind: convert::element_kind_contract(kind),
        label: kind.label().into(),
        model_kind: kind.model_kind().into(),
        default_name: kind.default_name().into(),
        requires_owner: kind.requires_owner(),
        requires_nested_owner: kind.requires_nested_owner(),
        pseudostate_kind: kind.resolve_pseudostate_kind().map(str::to_owned),
        initial_size: kind.initial_size(),
        sizes_by_diagram: diagrams
            .iter()
            .map(|diagram| (diagram.id.clone(), kind.initial_size_in_diagram(diagram)))
            .collect(),
    })
    .collect();
    let relations = [
        R::ElementImport,
        R::ProfileApplication,
        R::Extension,
        R::Connector,
        R::NoteLink,
        R::ConstraintLink,
        R::MessageSynchronous,
        R::MessageAsynchronous,
        R::MessageReply,
        R::MessageFound,
        R::MessageLost,
        R::GeneralOrdering,
        R::AssociationClassAttachment,
        R::Association,
        R::DirectedAssociation,
        R::Aggregation,
        R::Composition,
        R::Dependency,
        R::Generalization,
        R::InterfaceRealization,
        R::TemplateBinding,
        R::Realization,
        R::Containment,
        R::Usage,
        R::Abstraction,
        R::Include,
        R::Extend,
        R::PackageImport,
        R::PackageMerge,
        R::Deployment,
        R::Manifestation,
        R::ControlFlow,
        R::ObjectFlow,
        R::Transition,
    ]
    .into_iter()
    .map(|kind| RelationToolMetadata {
        kind: convert::relation_kind_contract(kind),
        label: kind.label().into(),
        model_kind: kind.model_kind().into(),
        endpoint_instruction: kind.endpoint_instruction().into(),
        uses_occurrence_anchors: kind.uses_occurrence_anchors(),
        is_visual_only: kind.is_visual_only(),
        message_sort: kind.resolve_message_sort().map(str::to_owned),
        message_kind: kind.resolve_message_kind().map(str::to_owned),
    })
    .collect();
    let diagram_metadata = crate::StructuralDiagramKind::ALL_DIAGRAMS
        .into_iter()
        .map(|kind| DiagramToolMetadata {
            kind: convert::diagram_kind_contract(kind),
            uml_di_type: kind.uml_di_type().into(),
            interaction_kind: kind.interaction_kind().map(str::to_owned),
            context_metaclass: kind.context_metaclass().map(str::to_owned),
        })
        .collect();
    let interface_forms = T::INTERFACE_FORMS
        .iter()
        .copied()
        .filter_map(|tool| {
            tool.resolve_interface_presentation()
                .map(|presentation| InterfaceToolMetadata {
                    tool: convert::tool_contract(tool),
                    label: tool.label().into(),
                    presentation: presentation.into(),
                })
        })
        .collect();
    let tool_groups: [(&str, Vec<T>); 18] = [
        (
            "Classes · basic",
            E::BASIC.iter().copied().map(T::Element).collect(),
        ),
        (
            "Relationships · basic",
            R::BASIC.iter().copied().map(T::Relation).collect(),
        ),
        (
            "Classes · advanced",
            E::ADVANCED.iter().copied().map(T::Element).collect(),
        ),
        (
            "Relationships · advanced",
            R::ADVANCED.iter().copied().map(T::Relation).collect(),
        ),
        (
            "Packages",
            E::PACKAGES
                .iter()
                .copied()
                .map(T::Element)
                .chain(R::PACKAGES.iter().copied().map(T::Relation))
                .collect(),
        ),
        (
            "Components & deployment",
            E::STRUCTURAL.iter().copied().map(T::Element).collect(),
        ),
        (
            "Profiles",
            E::PROFILES.iter().copied().map(T::Element).collect(),
        ),
        (
            "Behavior roots · notation",
            E::BEHAVIOR.iter().copied().map(T::Element).collect(),
        ),
        (
            "Activity nodes",
            E::ACTIVITY_NODES.iter().copied().map(T::Element).collect(),
        ),
        (
            "States",
            E::STATE_NODES.iter().copied().map(T::Element).collect(),
        ),
        (
            "Interactions",
            E::INTERACTION_NODES
                .iter()
                .copied()
                .map(T::Element)
                .collect(),
        ),
        (
            "Structural relationships",
            R::STRUCTURAL.iter().copied().map(T::Relation).collect(),
        ),
        (
            "Behavior relationships",
            R::BEHAVIOR.iter().copied().map(T::Relation).collect(),
        ),
        (
            "Ports, roles & nested elements",
            E::DETAILS.iter().copied().map(T::Element).collect(),
        ),
        (
            "State pseudostates",
            E::PSEUDOSTATES.iter().copied().map(T::Element).collect(),
        ),
        (
            "Interaction links · visual",
            R::MESSAGES.iter().copied().map(T::Relation).collect(),
        ),
        ("Interface forms · visual", T::INTERFACE_FORMS.to_vec()),
        (
            "Annotations",
            E::ANNOTATIONS.iter().copied().map(T::Element).collect(),
        ),
    ];
    let groups = tool_groups
        .into_iter()
        .map(|(name, tools)| ToolGroup {
            name: name.into(),
            tools: tools.into_iter().map(convert::tool_contract).collect(),
        })
        .collect();
    let profiles = diagrams.iter().filter(|diagram| crate::StructuralDiagramKind::from_diagram(diagram).is_none())
        .map(|diagram| {
            let unavailable_tools = if diagram.uml_di_type == "ERDDiagram" {
                ["Entity", "One to One Relationship", "One to Many Relationship", "Many to Many Relationship"]
                    .into_iter().map(|label| UnavailableToolMetadata {
                        label: label.into(),
                        reason: "ERD creation commands are not implemented yet; imported objects remain inspectable.".into(),
                    }).collect()
            } else { Vec::new() };
            (diagram.id.clone(), DiagramToolProfile {
                label: diagram_type_label(diagram), available_tools: Vec::new(), unavailable_tools,
                allows_new_views: false,
            })
        }).collect();
    ToolCatalog {
        elements,
        relations,
        diagrams: diagram_metadata,
        interface_forms,
        groups,
        profiles,
    }
}

pub(crate) fn diagram_type_label(diagram: &crate::Diagram) -> String {
    if diagram.uml_di_type == "ERDDiagram" {
        "Entity-Relationship".into()
    } else {
        diagram.uml_di_type.clone()
    }
}

pub(crate) fn imported_type_label(kind: &str) -> Option<&'static str> {
    match kind {
        "mdj.ERDDataModel" => Some("Data Model"),
        "mdj.ERDEntity" => Some("Entity"),
        "mdj.ERDColumn" => Some("Column"),
        "mdj.ERDRelationship" => Some("Relationship"),
        "mdj.ERDRelationshipEnd" => Some("Relationship end"),
        "ERDDiagram" => Some("Entity-Relationship diagram"),
        _ => None,
    }
}
