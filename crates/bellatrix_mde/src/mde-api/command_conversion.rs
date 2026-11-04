//! Private conversion from integration intents at the MDE API boundary.
use bellatrix_mde_protocol::command::*;

pub(crate) fn element_kind(value: ModelingElementKind) -> crate::commands::ModelingElementKind {
    match value {
        ModelingElementKind::Port => crate::commands::ModelingElementKind::Port,
        ModelingElementKind::Part => crate::commands::ModelingElementKind::Part,
        ModelingElementKind::DeploymentSpecification => {
            crate::commands::ModelingElementKind::DeploymentSpecification
        }
        ModelingElementKind::CollaborationUse => {
            crate::commands::ModelingElementKind::CollaborationUse
        }
        ModelingElementKind::DestructionOccurrenceSpecification => {
            crate::commands::ModelingElementKind::DestructionOccurrenceSpecification
        }
        ModelingElementKind::Continuation => crate::commands::ModelingElementKind::Continuation,
        ModelingElementKind::Class => crate::commands::ModelingElementKind::Class,
        ModelingElementKind::Interface => crate::commands::ModelingElementKind::Interface,
        ModelingElementKind::Component => crate::commands::ModelingElementKind::Component,
        ModelingElementKind::Artifact => crate::commands::ModelingElementKind::Artifact,
        ModelingElementKind::Signal => crate::commands::ModelingElementKind::Signal,
        ModelingElementKind::DataType => crate::commands::ModelingElementKind::DataType,
        ModelingElementKind::PrimitiveType => crate::commands::ModelingElementKind::PrimitiveType,
        ModelingElementKind::Enumeration => crate::commands::ModelingElementKind::Enumeration,
        ModelingElementKind::Frame => crate::commands::ModelingElementKind::Frame,
        ModelingElementKind::NaryAssociation => {
            crate::commands::ModelingElementKind::NaryAssociation
        }
        ModelingElementKind::AssociationClass => {
            crate::commands::ModelingElementKind::AssociationClass
        }
        ModelingElementKind::Package => crate::commands::ModelingElementKind::Package,
        ModelingElementKind::Model => crate::commands::ModelingElementKind::Model,
        ModelingElementKind::Subsystem => crate::commands::ModelingElementKind::Subsystem,
        ModelingElementKind::Comment => crate::commands::ModelingElementKind::Comment,
        ModelingElementKind::Template => crate::commands::ModelingElementKind::Template,
        ModelingElementKind::Node => crate::commands::ModelingElementKind::Node,
        ModelingElementKind::Device => crate::commands::ModelingElementKind::Device,
        ModelingElementKind::ExecutionEnvironment => {
            crate::commands::ModelingElementKind::ExecutionEnvironment
        }
        ModelingElementKind::Collaboration => crate::commands::ModelingElementKind::Collaboration,
        ModelingElementKind::InstanceSpecification => {
            crate::commands::ModelingElementKind::InstanceSpecification
        }
        ModelingElementKind::Profile => crate::commands::ModelingElementKind::Profile,
        ModelingElementKind::Stereotype => crate::commands::ModelingElementKind::Stereotype,
        ModelingElementKind::Actor => crate::commands::ModelingElementKind::Actor,
        ModelingElementKind::UseCase => crate::commands::ModelingElementKind::UseCase,
        ModelingElementKind::Activity => crate::commands::ModelingElementKind::Activity,
        ModelingElementKind::StateMachine => crate::commands::ModelingElementKind::StateMachine,
        ModelingElementKind::Interaction => crate::commands::ModelingElementKind::Interaction,
        ModelingElementKind::OpaqueAction => crate::commands::ModelingElementKind::OpaqueAction,
        ModelingElementKind::InitialNode => crate::commands::ModelingElementKind::InitialNode,
        ModelingElementKind::ActivityFinalNode => {
            crate::commands::ModelingElementKind::ActivityFinalNode
        }
        ModelingElementKind::FlowFinalNode => crate::commands::ModelingElementKind::FlowFinalNode,
        ModelingElementKind::DecisionNode => crate::commands::ModelingElementKind::DecisionNode,
        ModelingElementKind::MergeNode => crate::commands::ModelingElementKind::MergeNode,
        ModelingElementKind::ForkNode => crate::commands::ModelingElementKind::ForkNode,
        ModelingElementKind::JoinNode => crate::commands::ModelingElementKind::JoinNode,
        ModelingElementKind::CentralBufferNode => {
            crate::commands::ModelingElementKind::CentralBufferNode
        }
        ModelingElementKind::DataStoreNode => crate::commands::ModelingElementKind::DataStoreNode,
        ModelingElementKind::SendSignalAction => {
            crate::commands::ModelingElementKind::SendSignalAction
        }
        ModelingElementKind::AcceptEventAction => {
            crate::commands::ModelingElementKind::AcceptEventAction
        }
        ModelingElementKind::ActivityPartition => {
            crate::commands::ModelingElementKind::ActivityPartition
        }
        ModelingElementKind::Region => crate::commands::ModelingElementKind::Region,
        ModelingElementKind::State => crate::commands::ModelingElementKind::State,
        ModelingElementKind::Pseudostate => crate::commands::ModelingElementKind::Pseudostate,
        ModelingElementKind::FinalState => crate::commands::ModelingElementKind::FinalState,
        ModelingElementKind::Lifeline => crate::commands::ModelingElementKind::Lifeline,
        ModelingElementKind::BehaviorExecutionSpecification => {
            crate::commands::ModelingElementKind::BehaviorExecutionSpecification
        }
        ModelingElementKind::InteractionUse => crate::commands::ModelingElementKind::InteractionUse,
        ModelingElementKind::CombinedFragment => {
            crate::commands::ModelingElementKind::CombinedFragment
        }
        ModelingElementKind::StateInvariant => crate::commands::ModelingElementKind::StateInvariant,
        ModelingElementKind::InputPin => crate::commands::ModelingElementKind::InputPin,
        ModelingElementKind::OutputPin => crate::commands::ModelingElementKind::OutputPin,
        ModelingElementKind::ValuePin => crate::commands::ModelingElementKind::ValuePin,
        ModelingElementKind::InteractionOperand => {
            crate::commands::ModelingElementKind::InteractionOperand
        }
        ModelingElementKind::Choice => crate::commands::ModelingElementKind::Choice,
        ModelingElementKind::Junction => crate::commands::ModelingElementKind::Junction,
        ModelingElementKind::StateFork => crate::commands::ModelingElementKind::StateFork,
        ModelingElementKind::StateJoin => crate::commands::ModelingElementKind::StateJoin,
        ModelingElementKind::ShallowHistory => crate::commands::ModelingElementKind::ShallowHistory,
        ModelingElementKind::DeepHistory => crate::commands::ModelingElementKind::DeepHistory,
        ModelingElementKind::EntryPoint => crate::commands::ModelingElementKind::EntryPoint,
        ModelingElementKind::ExitPoint => crate::commands::ModelingElementKind::ExitPoint,
        ModelingElementKind::Terminate => crate::commands::ModelingElementKind::Terminate,
    }
}

pub(crate) fn member_kind(value: ModelingMemberKind) -> crate::commands::ModelingMemberKind {
    match value {
        ModelingMemberKind::Attribute => crate::commands::ModelingMemberKind::Attribute,
        ModelingMemberKind::Operation => crate::commands::ModelingMemberKind::Operation,
        ModelingMemberKind::Parameter => crate::commands::ModelingMemberKind::Parameter,
        ModelingMemberKind::Reception => crate::commands::ModelingMemberKind::Reception,
    }
}

pub(crate) fn relation_kind(value: ModelingRelationKind) -> crate::ModelingRelationKind {
    match value {
        ModelingRelationKind::ElementImport => crate::ModelingRelationKind::ElementImport,
        ModelingRelationKind::ProfileApplication => crate::ModelingRelationKind::ProfileApplication,
        ModelingRelationKind::Extension => crate::ModelingRelationKind::Extension,
        ModelingRelationKind::Connector => crate::ModelingRelationKind::Connector,
        ModelingRelationKind::NoteLink => crate::ModelingRelationKind::NoteLink,
        ModelingRelationKind::ConstraintLink => crate::ModelingRelationKind::ConstraintLink,
        ModelingRelationKind::MessageSynchronous => crate::ModelingRelationKind::MessageSynchronous,
        ModelingRelationKind::MessageAsynchronous => {
            crate::ModelingRelationKind::MessageAsynchronous
        }
        ModelingRelationKind::MessageReply => crate::ModelingRelationKind::MessageReply,
        ModelingRelationKind::MessageFound => crate::ModelingRelationKind::MessageFound,
        ModelingRelationKind::MessageLost => crate::ModelingRelationKind::MessageLost,
        ModelingRelationKind::GeneralOrdering => crate::ModelingRelationKind::GeneralOrdering,
        ModelingRelationKind::AssociationClassAttachment => {
            crate::ModelingRelationKind::AssociationClassAttachment
        }
        ModelingRelationKind::Association => crate::ModelingRelationKind::Association,
        ModelingRelationKind::DirectedAssociation => {
            crate::ModelingRelationKind::DirectedAssociation
        }
        ModelingRelationKind::Aggregation => crate::ModelingRelationKind::Aggregation,
        ModelingRelationKind::Composition => crate::ModelingRelationKind::Composition,
        ModelingRelationKind::Dependency => crate::ModelingRelationKind::Dependency,
        ModelingRelationKind::Generalization => crate::ModelingRelationKind::Generalization,
        ModelingRelationKind::InterfaceRealization => {
            crate::ModelingRelationKind::InterfaceRealization
        }
        ModelingRelationKind::TemplateBinding => crate::ModelingRelationKind::TemplateBinding,
        ModelingRelationKind::Realization => crate::ModelingRelationKind::Realization,
        ModelingRelationKind::Containment => crate::ModelingRelationKind::Containment,
        ModelingRelationKind::Usage => crate::ModelingRelationKind::Usage,
        ModelingRelationKind::Abstraction => crate::ModelingRelationKind::Abstraction,
        ModelingRelationKind::Include => crate::ModelingRelationKind::Include,
        ModelingRelationKind::Extend => crate::ModelingRelationKind::Extend,
        ModelingRelationKind::PackageImport => crate::ModelingRelationKind::PackageImport,
        ModelingRelationKind::PackageMerge => crate::ModelingRelationKind::PackageMerge,
        ModelingRelationKind::Deployment => crate::ModelingRelationKind::Deployment,
        ModelingRelationKind::Manifestation => crate::ModelingRelationKind::Manifestation,
        ModelingRelationKind::ControlFlow => crate::ModelingRelationKind::ControlFlow,
        ModelingRelationKind::ObjectFlow => crate::ModelingRelationKind::ObjectFlow,
        ModelingRelationKind::Transition => crate::ModelingRelationKind::Transition,
    }
}

pub(crate) fn diagram_kind(value: StructuralDiagramKind) -> crate::StructuralDiagramKind {
    match value {
        StructuralDiagramKind::Class => crate::StructuralDiagramKind::Class,
        StructuralDiagramKind::Object => crate::StructuralDiagramKind::Object,
        StructuralDiagramKind::Package => crate::StructuralDiagramKind::Package,
        StructuralDiagramKind::Component => crate::StructuralDiagramKind::Component,
        StructuralDiagramKind::CompositeStructure => {
            crate::StructuralDiagramKind::CompositeStructure
        }
        StructuralDiagramKind::Deployment => crate::StructuralDiagramKind::Deployment,
        StructuralDiagramKind::Profile => crate::StructuralDiagramKind::Profile,
        StructuralDiagramKind::UseCase => crate::StructuralDiagramKind::UseCase,
        StructuralDiagramKind::Activity => crate::StructuralDiagramKind::Activity,
        StructuralDiagramKind::StateMachine => crate::StructuralDiagramKind::StateMachine,
        StructuralDiagramKind::Sequence => crate::StructuralDiagramKind::Sequence,
        StructuralDiagramKind::Communication => crate::StructuralDiagramKind::Communication,
        StructuralDiagramKind::InteractionOverview => {
            crate::StructuralDiagramKind::InteractionOverview
        }
        StructuralDiagramKind::Timing => crate::StructuralDiagramKind::Timing,
    }
}

pub(crate) fn quick_create(value: UmlQuickCreate) -> crate::UmlQuickCreate {
    match value {
        UmlQuickCreate::Note => crate::UmlQuickCreate::Note,
        UmlQuickCreate::Constraint => crate::UmlQuickCreate::Constraint,
        UmlQuickCreate::Reception => crate::UmlQuickCreate::Reception,
        UmlQuickCreate::Subtype => crate::UmlQuickCreate::Subtype,
        UmlQuickCreate::Supertype => crate::UmlQuickCreate::Supertype,
        UmlQuickCreate::RealizingClass => crate::UmlQuickCreate::RealizingClass,
        UmlQuickCreate::TemplateParameter => crate::UmlQuickCreate::TemplateParameter,
        UmlQuickCreate::ProvidedInterface => crate::UmlQuickCreate::ProvidedInterface,
        UmlQuickCreate::RequiredInterface => crate::UmlQuickCreate::RequiredInterface,
        UmlQuickCreate::AssociatedClass => crate::UmlQuickCreate::AssociatedClass,
        UmlQuickCreate::AggregatedClass => crate::UmlQuickCreate::AggregatedClass,
        UmlQuickCreate::CompositedClass => crate::UmlQuickCreate::CompositedClass,
        UmlQuickCreate::Port => crate::UmlQuickCreate::Port,
        UmlQuickCreate::Part => crate::UmlQuickCreate::Part,
    }
}

pub(crate) fn view_order(value: ViewOrder) -> crate::commands::ViewOrder {
    match value {
        ViewOrder::BringToFront => crate::commands::ViewOrder::BringToFront,
        ViewOrder::SendToBack => crate::commands::ViewOrder::SendToBack,
        ViewOrder::BringForward => crate::commands::ViewOrder::BringForward,
        ViewOrder::SendBackward => crate::commands::ViewOrder::SendBackward,
    }
}

/// Fallible structural conversion of independent presentation DTOs. Check the
/// neutral round trip first so JSON cannot silently coerce non-finite floats to null.
pub(crate) fn presentation<T, U>(value: T) -> Result<U, String>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
    U: serde::de::DeserializeOwned,
{
    let encoded = serde_json::to_value(&value).map_err(|error| error.to_string())?;
    let verified: T = serde_json::from_value(encoded.clone()).map_err(|error| error.to_string())?;
    if verified != value {
        return Err("command presentation cannot be represented without loss".into());
    }
    serde_json::from_value(encoded)
        .map_err(|error| format!("invalid command presentation: {error}"))
}

pub(crate) fn occurrence(value: ViewOccurrence) -> crate::ViewOccurrence {
    crate::ViewOccurrence {
        diagram_id: value.diagram_id,
        view_id: value.view_id,
        member_path: value.member_path,
    }
}

fn association_owner(value: AssociationEndOwner) -> crate::commands::AssociationEndOwner {
    match value {
        AssociationEndOwner::Association => crate::commands::AssociationEndOwner::Association,
        AssociationEndOwner::Classifier(id) => crate::commands::AssociationEndOwner::Classifier(id),
    }
}

pub(crate) fn association_edit(value: AssociationEndEdit) -> crate::commands::AssociationEndEdit {
    use crate::commands as domain;
    match value {
        AssociationEndEdit::Insert { position, end } => domain::AssociationEndEdit::Insert {
            position,
            end: domain::AssociationEndDraft {
                name: end.name,
                type_id: end.type_id,
                owner: association_owner(end.owner),
                aggregation: match end.aggregation {
                    Aggregation::None => crate::uml::schema::Aggregation::None,
                    Aggregation::Shared => crate::uml::schema::Aggregation::Shared,
                    Aggregation::Composite => crate::uml::schema::Aggregation::Composite,
                },
                multiplicity: end.multiplicity,
            },
        },
        AssociationEndEdit::Attach {
            position,
            property_id,
            owner,
        } => domain::AssociationEndEdit::Attach {
            position,
            property_id,
            owner: owner.map(association_owner),
        },
        AssociationEndEdit::Remove {
            property_id,
            disposition,
        } => domain::AssociationEndEdit::Remove {
            property_id,
            disposition: match disposition {
                RemovedAssociationEnd::KeepClassifierOwned => {
                    domain::RemovedAssociationEnd::KeepClassifierOwned
                }
                RemovedAssociationEnd::ReparentToClassifier(id) => {
                    domain::RemovedAssociationEnd::ReparentToClassifier(id)
                }
                RemovedAssociationEnd::DeleteAssociationOwned => {
                    domain::RemovedAssociationEnd::DeleteAssociationOwned
                }
            },
        },
        AssociationEndEdit::Move {
            property_id,
            position,
        } => domain::AssociationEndEdit::Move {
            property_id,
            position,
        },
        AssociationEndEdit::Reparent { property_id, owner } => {
            domain::AssociationEndEdit::Reparent {
                property_id,
                owner: association_owner(owner),
            }
        }
        AssociationEndEdit::Retype {
            property_id,
            type_id,
        } => domain::AssociationEndEdit::Retype {
            property_id,
            type_id,
        },
    }
}

pub(crate) fn element_kind_contract(
    value: crate::commands::ModelingElementKind,
) -> ModelingElementKind {
    match value {
        crate::commands::ModelingElementKind::Port => ModelingElementKind::Port,
        crate::commands::ModelingElementKind::Part => ModelingElementKind::Part,
        crate::commands::ModelingElementKind::DeploymentSpecification => {
            ModelingElementKind::DeploymentSpecification
        }
        crate::commands::ModelingElementKind::CollaborationUse => {
            ModelingElementKind::CollaborationUse
        }
        crate::commands::ModelingElementKind::DestructionOccurrenceSpecification => {
            ModelingElementKind::DestructionOccurrenceSpecification
        }
        crate::commands::ModelingElementKind::Continuation => ModelingElementKind::Continuation,
        crate::commands::ModelingElementKind::Class => ModelingElementKind::Class,
        crate::commands::ModelingElementKind::Interface => ModelingElementKind::Interface,
        crate::commands::ModelingElementKind::Component => ModelingElementKind::Component,
        crate::commands::ModelingElementKind::Artifact => ModelingElementKind::Artifact,
        crate::commands::ModelingElementKind::Signal => ModelingElementKind::Signal,
        crate::commands::ModelingElementKind::DataType => ModelingElementKind::DataType,
        crate::commands::ModelingElementKind::PrimitiveType => ModelingElementKind::PrimitiveType,
        crate::commands::ModelingElementKind::Enumeration => ModelingElementKind::Enumeration,
        crate::commands::ModelingElementKind::Frame => ModelingElementKind::Frame,
        crate::commands::ModelingElementKind::NaryAssociation => {
            ModelingElementKind::NaryAssociation
        }
        crate::commands::ModelingElementKind::AssociationClass => {
            ModelingElementKind::AssociationClass
        }
        crate::commands::ModelingElementKind::Package => ModelingElementKind::Package,
        crate::commands::ModelingElementKind::Model => ModelingElementKind::Model,
        crate::commands::ModelingElementKind::Subsystem => ModelingElementKind::Subsystem,
        crate::commands::ModelingElementKind::Comment => ModelingElementKind::Comment,
        crate::commands::ModelingElementKind::Template => ModelingElementKind::Template,
        crate::commands::ModelingElementKind::Node => ModelingElementKind::Node,
        crate::commands::ModelingElementKind::Device => ModelingElementKind::Device,
        crate::commands::ModelingElementKind::ExecutionEnvironment => {
            ModelingElementKind::ExecutionEnvironment
        }
        crate::commands::ModelingElementKind::Collaboration => ModelingElementKind::Collaboration,
        crate::commands::ModelingElementKind::InstanceSpecification => {
            ModelingElementKind::InstanceSpecification
        }
        crate::commands::ModelingElementKind::Profile => ModelingElementKind::Profile,
        crate::commands::ModelingElementKind::Stereotype => ModelingElementKind::Stereotype,
        crate::commands::ModelingElementKind::Actor => ModelingElementKind::Actor,
        crate::commands::ModelingElementKind::UseCase => ModelingElementKind::UseCase,
        crate::commands::ModelingElementKind::Activity => ModelingElementKind::Activity,
        crate::commands::ModelingElementKind::StateMachine => ModelingElementKind::StateMachine,
        crate::commands::ModelingElementKind::Interaction => ModelingElementKind::Interaction,
        crate::commands::ModelingElementKind::OpaqueAction => ModelingElementKind::OpaqueAction,
        crate::commands::ModelingElementKind::InitialNode => ModelingElementKind::InitialNode,
        crate::commands::ModelingElementKind::ActivityFinalNode => {
            ModelingElementKind::ActivityFinalNode
        }
        crate::commands::ModelingElementKind::FlowFinalNode => ModelingElementKind::FlowFinalNode,
        crate::commands::ModelingElementKind::DecisionNode => ModelingElementKind::DecisionNode,
        crate::commands::ModelingElementKind::MergeNode => ModelingElementKind::MergeNode,
        crate::commands::ModelingElementKind::ForkNode => ModelingElementKind::ForkNode,
        crate::commands::ModelingElementKind::JoinNode => ModelingElementKind::JoinNode,
        crate::commands::ModelingElementKind::CentralBufferNode => {
            ModelingElementKind::CentralBufferNode
        }
        crate::commands::ModelingElementKind::DataStoreNode => ModelingElementKind::DataStoreNode,
        crate::commands::ModelingElementKind::SendSignalAction => {
            ModelingElementKind::SendSignalAction
        }
        crate::commands::ModelingElementKind::AcceptEventAction => {
            ModelingElementKind::AcceptEventAction
        }
        crate::commands::ModelingElementKind::ActivityPartition => {
            ModelingElementKind::ActivityPartition
        }
        crate::commands::ModelingElementKind::Region => ModelingElementKind::Region,
        crate::commands::ModelingElementKind::State => ModelingElementKind::State,
        crate::commands::ModelingElementKind::Pseudostate => ModelingElementKind::Pseudostate,
        crate::commands::ModelingElementKind::FinalState => ModelingElementKind::FinalState,
        crate::commands::ModelingElementKind::Lifeline => ModelingElementKind::Lifeline,
        crate::commands::ModelingElementKind::BehaviorExecutionSpecification => {
            ModelingElementKind::BehaviorExecutionSpecification
        }
        crate::commands::ModelingElementKind::InteractionUse => ModelingElementKind::InteractionUse,
        crate::commands::ModelingElementKind::CombinedFragment => {
            ModelingElementKind::CombinedFragment
        }
        crate::commands::ModelingElementKind::StateInvariant => ModelingElementKind::StateInvariant,
        crate::commands::ModelingElementKind::InputPin => ModelingElementKind::InputPin,
        crate::commands::ModelingElementKind::OutputPin => ModelingElementKind::OutputPin,
        crate::commands::ModelingElementKind::ValuePin => ModelingElementKind::ValuePin,
        crate::commands::ModelingElementKind::InteractionOperand => {
            ModelingElementKind::InteractionOperand
        }
        crate::commands::ModelingElementKind::Choice => ModelingElementKind::Choice,
        crate::commands::ModelingElementKind::Junction => ModelingElementKind::Junction,
        crate::commands::ModelingElementKind::StateFork => ModelingElementKind::StateFork,
        crate::commands::ModelingElementKind::StateJoin => ModelingElementKind::StateJoin,
        crate::commands::ModelingElementKind::ShallowHistory => ModelingElementKind::ShallowHistory,
        crate::commands::ModelingElementKind::DeepHistory => ModelingElementKind::DeepHistory,
        crate::commands::ModelingElementKind::EntryPoint => ModelingElementKind::EntryPoint,
        crate::commands::ModelingElementKind::ExitPoint => ModelingElementKind::ExitPoint,
        crate::commands::ModelingElementKind::Terminate => ModelingElementKind::Terminate,
    }
}

pub(crate) fn member_kind_contract(
    value: crate::commands::ModelingMemberKind,
) -> ModelingMemberKind {
    match value {
        crate::commands::ModelingMemberKind::Attribute => ModelingMemberKind::Attribute,
        crate::commands::ModelingMemberKind::Operation => ModelingMemberKind::Operation,
        crate::commands::ModelingMemberKind::Parameter => ModelingMemberKind::Parameter,
        crate::commands::ModelingMemberKind::Reception => ModelingMemberKind::Reception,
    }
}

pub(crate) fn relation_kind_contract(value: crate::ModelingRelationKind) -> ModelingRelationKind {
    match value {
        crate::ModelingRelationKind::ElementImport => ModelingRelationKind::ElementImport,
        crate::ModelingRelationKind::ProfileApplication => ModelingRelationKind::ProfileApplication,
        crate::ModelingRelationKind::Extension => ModelingRelationKind::Extension,
        crate::ModelingRelationKind::Connector => ModelingRelationKind::Connector,
        crate::ModelingRelationKind::NoteLink => ModelingRelationKind::NoteLink,
        crate::ModelingRelationKind::ConstraintLink => ModelingRelationKind::ConstraintLink,
        crate::ModelingRelationKind::MessageSynchronous => ModelingRelationKind::MessageSynchronous,
        crate::ModelingRelationKind::MessageAsynchronous => {
            ModelingRelationKind::MessageAsynchronous
        }
        crate::ModelingRelationKind::MessageReply => ModelingRelationKind::MessageReply,
        crate::ModelingRelationKind::MessageFound => ModelingRelationKind::MessageFound,
        crate::ModelingRelationKind::MessageLost => ModelingRelationKind::MessageLost,
        crate::ModelingRelationKind::GeneralOrdering => ModelingRelationKind::GeneralOrdering,
        crate::ModelingRelationKind::AssociationClassAttachment => {
            ModelingRelationKind::AssociationClassAttachment
        }
        crate::ModelingRelationKind::Association => ModelingRelationKind::Association,
        crate::ModelingRelationKind::DirectedAssociation => {
            ModelingRelationKind::DirectedAssociation
        }
        crate::ModelingRelationKind::Aggregation => ModelingRelationKind::Aggregation,
        crate::ModelingRelationKind::Composition => ModelingRelationKind::Composition,
        crate::ModelingRelationKind::Dependency => ModelingRelationKind::Dependency,
        crate::ModelingRelationKind::Generalization => ModelingRelationKind::Generalization,
        crate::ModelingRelationKind::InterfaceRealization => {
            ModelingRelationKind::InterfaceRealization
        }
        crate::ModelingRelationKind::TemplateBinding => ModelingRelationKind::TemplateBinding,
        crate::ModelingRelationKind::Realization => ModelingRelationKind::Realization,
        crate::ModelingRelationKind::Containment => ModelingRelationKind::Containment,
        crate::ModelingRelationKind::Usage => ModelingRelationKind::Usage,
        crate::ModelingRelationKind::Abstraction => ModelingRelationKind::Abstraction,
        crate::ModelingRelationKind::Include => ModelingRelationKind::Include,
        crate::ModelingRelationKind::Extend => ModelingRelationKind::Extend,
        crate::ModelingRelationKind::PackageImport => ModelingRelationKind::PackageImport,
        crate::ModelingRelationKind::PackageMerge => ModelingRelationKind::PackageMerge,
        crate::ModelingRelationKind::Deployment => ModelingRelationKind::Deployment,
        crate::ModelingRelationKind::Manifestation => ModelingRelationKind::Manifestation,
        crate::ModelingRelationKind::ControlFlow => ModelingRelationKind::ControlFlow,
        crate::ModelingRelationKind::ObjectFlow => ModelingRelationKind::ObjectFlow,
        crate::ModelingRelationKind::Transition => ModelingRelationKind::Transition,
    }
}

pub(crate) fn diagram_kind_contract(value: crate::StructuralDiagramKind) -> StructuralDiagramKind {
    match value {
        crate::StructuralDiagramKind::Class => StructuralDiagramKind::Class,
        crate::StructuralDiagramKind::Object => StructuralDiagramKind::Object,
        crate::StructuralDiagramKind::Package => StructuralDiagramKind::Package,
        crate::StructuralDiagramKind::Component => StructuralDiagramKind::Component,
        crate::StructuralDiagramKind::CompositeStructure => {
            StructuralDiagramKind::CompositeStructure
        }
        crate::StructuralDiagramKind::Deployment => StructuralDiagramKind::Deployment,
        crate::StructuralDiagramKind::Profile => StructuralDiagramKind::Profile,
        crate::StructuralDiagramKind::UseCase => StructuralDiagramKind::UseCase,
        crate::StructuralDiagramKind::Activity => StructuralDiagramKind::Activity,
        crate::StructuralDiagramKind::StateMachine => StructuralDiagramKind::StateMachine,
        crate::StructuralDiagramKind::Sequence => StructuralDiagramKind::Sequence,
        crate::StructuralDiagramKind::Communication => StructuralDiagramKind::Communication,
        crate::StructuralDiagramKind::InteractionOverview => {
            StructuralDiagramKind::InteractionOverview
        }
        crate::StructuralDiagramKind::Timing => StructuralDiagramKind::Timing,
    }
}

pub(crate) fn quick_create_contract(value: crate::UmlQuickCreate) -> UmlQuickCreate {
    match value {
        crate::UmlQuickCreate::Note => UmlQuickCreate::Note,
        crate::UmlQuickCreate::Constraint => UmlQuickCreate::Constraint,
        crate::UmlQuickCreate::Reception => UmlQuickCreate::Reception,
        crate::UmlQuickCreate::Subtype => UmlQuickCreate::Subtype,
        crate::UmlQuickCreate::Supertype => UmlQuickCreate::Supertype,
        crate::UmlQuickCreate::RealizingClass => UmlQuickCreate::RealizingClass,
        crate::UmlQuickCreate::TemplateParameter => UmlQuickCreate::TemplateParameter,
        crate::UmlQuickCreate::ProvidedInterface => UmlQuickCreate::ProvidedInterface,
        crate::UmlQuickCreate::RequiredInterface => UmlQuickCreate::RequiredInterface,
        crate::UmlQuickCreate::AssociatedClass => UmlQuickCreate::AssociatedClass,
        crate::UmlQuickCreate::AggregatedClass => UmlQuickCreate::AggregatedClass,
        crate::UmlQuickCreate::CompositedClass => UmlQuickCreate::CompositedClass,
        crate::UmlQuickCreate::Port => UmlQuickCreate::Port,
        crate::UmlQuickCreate::Part => UmlQuickCreate::Part,
    }
}

pub(crate) fn view_order_contract(value: crate::commands::ViewOrder) -> ViewOrder {
    match value {
        crate::commands::ViewOrder::BringToFront => ViewOrder::BringToFront,
        crate::commands::ViewOrder::SendToBack => ViewOrder::SendToBack,
        crate::commands::ViewOrder::BringForward => ViewOrder::BringForward,
        crate::commands::ViewOrder::SendBackward => ViewOrder::SendBackward,
    }
}

pub(crate) fn tool_contract(value: crate::commands::ModelingTool) -> ModelingTool {
    match value {
        crate::commands::ModelingTool::ProvidedInterface => ModelingTool::ProvidedInterface,
        crate::commands::ModelingTool::RequiredInterface => ModelingTool::RequiredInterface,
        crate::commands::ModelingTool::Element(kind) => {
            ModelingTool::Element(element_kind_contract(kind))
        }
        crate::commands::ModelingTool::Relation(kind) => {
            ModelingTool::Relation(relation_kind_contract(kind))
        }
    }
}

/// Convert transport intent only at the engine API boundary.
pub(crate) fn command(value: Command) -> Result<crate::commands::Command, String> {
    Ok(match value {
        Command::AdoptRootModel => crate::commands::Command::AdoptRootModel,
        Command::SetOwner {
            element_id,
            owner_id,
        } => crate::commands::Command::SetOwner {
            element_id: element_id,
            owner_id: owner_id,
        },
        Command::SetExplorerParent {
            subject_id,
            parent_id,
            parent_occurrence,
        } => crate::commands::Command::SetExplorerParent {
            subject_id: subject_id,
            parent_id: parent_id,
            parent_occurrence: parent_occurrence.map(occurrence),
        },
        Command::CreateDiagram {
            kind,
            name,
            model_elements,
        } => crate::commands::Command::CreateDiagram {
            kind: diagram_kind(kind),
            name: name,
            model_elements: model_elements,
        },
        Command::RenameDiagram { diagram_id, name } => crate::commands::Command::RenameDiagram {
            diagram_id: diagram_id,
            name: name,
        },
        Command::DeleteDiagram { diagram_id } => crate::commands::Command::DeleteDiagram {
            diagram_id: diagram_id,
        },
        Command::CreateExplorerDiagram {
            kind,
            name,
            model_elements,
            parent_id,
            parent_occurrence,
        } => crate::commands::Command::CreateExplorerDiagram {
            kind: diagram_kind(kind),
            name: name,
            model_elements: model_elements,
            parent_id: parent_id,
            parent_occurrence: parent_occurrence.map(occurrence),
        },
        Command::CreateModelElement {
            owner_id,
            kind,
            name,
        } => crate::commands::Command::CreateModelElement {
            owner_id: owner_id,
            kind: element_kind(kind),
            name: name,
        },
        Command::CreateElement {
            diagram_id,
            owner_id,
            kind,
            name,
            x,
            y,
        } => crate::commands::Command::CreateElement {
            diagram_id: diagram_id,
            owner_id: owner_id,
            kind: element_kind(kind),
            name: name,
            x: x,
            y: y,
        },
        Command::CreateElementOrVisualDraft {
            diagram_id,
            owner_id,
            kind,
            name,
            x,
            y,
        } => crate::commands::Command::CreateElementOrVisualDraft {
            diagram_id: diagram_id,
            owner_id: owner_id,
            kind: element_kind(kind),
            name: name,
            x: x,
            y: y,
        },
        Command::CreateRelationOrVisualDraft {
            diagram_id,
            owner_id,
            kind,
            from,
            to,
        } => crate::commands::Command::CreateRelationOrVisualDraft {
            diagram_id: diagram_id,
            owner_id: owner_id,
            kind: relation_kind(kind),
            from: from,
            to: to,
        },
        Command::CreateRelationAtViewsOrVisualDraft {
            diagram_id,
            owner_id,
            kind,
            source_view_id,
            target_view_id,
            from,
            to,
        } => crate::commands::Command::CreateRelationAtViewsOrVisualDraft {
            diagram_id: diagram_id,
            owner_id: owner_id,
            kind: relation_kind(kind),
            source_view_id: source_view_id,
            target_view_id: target_view_id,
            from: from,
            to: to,
        },
        Command::CreateRelationAtViews {
            diagram_id,
            owner_id,
            kind,
            from,
            to,
            endpoints,
        } => crate::commands::Command::CreateRelationAtViews {
            diagram_id: diagram_id,
            owner_id: owner_id,
            kind: relation_kind(kind),
            from: from,
            to: to,
            endpoints: presentation(endpoints)?,
        },
        Command::CreateRelation {
            diagram_id,
            owner_id,
            kind,
            from,
            to,
        } => crate::commands::Command::CreateRelation {
            diagram_id: diagram_id,
            owner_id: owner_id,
            kind: relation_kind(kind),
            from: from,
            to: to,
        },
        Command::AddMember { owner_id, kind } => crate::commands::Command::AddMember {
            owner_id: owner_id,
            kind: member_kind(kind),
        },
        Command::MoveParameter { parameter_id, up } => crate::commands::Command::MoveParameter {
            parameter_id: parameter_id,
            up: up,
        },
        Command::MigrateAssociation { association_id } => {
            crate::commands::Command::MigrateAssociation {
                association_id: association_id,
            }
        }
        Command::SetRedefinedElements {
            element_id,
            redefined_ids,
        } => crate::commands::Command::SetRedefinedElements {
            element_id: element_id,
            redefined_ids: redefined_ids,
        },
        Command::SetSubsettedProperties {
            property_id,
            subsetted_ids,
        } => crate::commands::Command::SetSubsettedProperties {
            property_id: property_id,
            subsetted_ids: subsetted_ids,
        },
        Command::SetNamespaceImport {
            import_id,
            namespace_id,
            target_id,
            is_package,
            alias,
            visibility,
        } => crate::commands::Command::SetNamespaceImport {
            import_id: import_id,
            namespace_id: namespace_id,
            target_id: target_id,
            is_package: is_package,
            alias: alias,
            visibility: visibility,
        },
        Command::SetAssociationEndNavigation {
            association_id,
            property_id,
            navigation,
        } => crate::commands::Command::SetAssociationEndNavigation {
            association_id: association_id,
            property_id: property_id,
            navigation: match navigation {
                NavigationKnowledge::Known(value) => crate::NavigationKnowledge::Known(value),
                NavigationKnowledge::Unverified => crate::NavigationKnowledge::Unverified,
            },
        },
        Command::SetAssociationMemberEnds {
            association_id,
            member_end_ids,
        } => crate::commands::Command::SetAssociationMemberEnds {
            association_id: association_id,
            member_end_ids: member_end_ids,
        },
        Command::EditAssociationEnds {
            association_id,
            edits,
        } => crate::commands::Command::EditAssociationEnds {
            association_id: association_id,
            edits: edits.into_iter().map(association_edit).collect(),
        },
        Command::QuickEdit {
            diagram_id,
            element_id,
            action,
        } => crate::commands::Command::QuickEdit {
            diagram_id: diagram_id,
            element_id: element_id,
            action: quick_create(action),
        },
        Command::CreateVisualDraft {
            diagram_id,
            notation_kind,
            label,
            geometry,
            properties,
            parent_view_id,
            source_view_id,
            target_view_id,
        } => crate::commands::Command::CreateVisualDraft {
            diagram_id: diagram_id,
            notation_kind: notation_kind,
            label: label,
            geometry: presentation(geometry)?,
            properties: properties,
            parent_view_id: parent_view_id,
            source_view_id: source_view_id,
            target_view_id: target_view_id,
        },
        Command::PlaceElementView {
            diagram_id,
            element_id,
            x,
            y,
        } => crate::commands::Command::PlaceElementView {
            diagram_id: diagram_id,
            element_id: element_id,
            x: x,
            y: y,
        },
        Command::RemoveView {
            diagram_id,
            view_id,
        } => crate::commands::Command::RemoveView {
            diagram_id: diagram_id,
            view_id: view_id,
        },
        Command::ReorderView {
            diagram_id,
            view_id,
            order,
        } => crate::commands::Command::ReorderView {
            diagram_id: diagram_id,
            view_id: view_id,
            order: view_order(order),
        },
        Command::SetViewLabel { view_id, label } => crate::commands::Command::SetViewLabel {
            view_id: view_id,
            label: label,
        },
        Command::SetViewTiming { view_id, timing } => crate::commands::Command::SetViewTiming {
            view_id: view_id,
            timing: timing.map(presentation).transpose()?,
        },
        Command::SetViewEndpoints {
            diagram_id,
            view_id,
            endpoints,
        } => crate::commands::Command::SetViewEndpoints {
            diagram_id: diagram_id,
            view_id: view_id,
            endpoints: presentation(endpoints)?,
        },
        Command::SetViewStyle {
            diagram_id,
            view_id,
            style,
        } => crate::commands::Command::SetViewStyle {
            diagram_id: diagram_id,
            view_id: view_id,
            style: presentation(style)?,
        },
        Command::SetViewPath {
            diagram_id,
            view_id,
            geometry,
        } => crate::commands::Command::SetViewPath {
            diagram_id,
            view_id,
            geometry: presentation(geometry)?,
        },
        Command::SetViewRouting {
            diagram_id,
            view_id,
            routing,
        } => crate::commands::Command::SetViewRouting {
            diagram_id: diagram_id,
            view_id: view_id,
            routing: presentation(routing)?,
        },
        Command::MoveView { view_id, dx, dy } => crate::commands::Command::MoveView {
            view_id: view_id,
            dx: dx,
            dy: dy,
        },
        Command::ResizeView {
            view_id,
            width,
            height,
        } => crate::commands::Command::ResizeView {
            view_id: view_id,
            width: width,
            height: height,
        },
        Command::DeleteElement { element_id } => crate::commands::Command::DeleteElement {
            element_id: element_id,
        },
        Command::SetProperty {
            element_id,
            path,
            value,
        } => crate::commands::Command::SetProperty {
            element_id: element_id,
            path: path,
            value: value,
        },
        Command::SetNameExpression {
            element_id,
            expression,
        } => crate::commands::Command::SetNameExpression {
            element_id: element_id,
            expression: expression,
        },
        Command::Rename { element_id, name } => crate::commands::Command::Rename {
            element_id: element_id,
            name: name,
        },
    })
}
