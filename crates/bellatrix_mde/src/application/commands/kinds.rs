use crate::ModelingRelationKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModelingElementKind {
    Port,
    Part,
    DeploymentSpecification,
    CollaborationUse,
    DestructionOccurrenceSpecification,
    Continuation,
    Class,
    Interface,
    Component,
    Artifact,
    Signal,
    DataType,
    PrimitiveType,
    Enumeration,
    Frame,
    NaryAssociation,
    AssociationClass,
    Package,
    Model,
    Subsystem,
    Comment,
    Template,
    Node,
    Device,
    ExecutionEnvironment,
    Collaboration,
    InstanceSpecification,
    Profile,
    Stereotype,
    Actor,
    UseCase,
    Activity,
    StateMachine,
    Interaction,
    OpaqueAction,
    InitialNode,
    ActivityFinalNode,
    FlowFinalNode,
    DecisionNode,
    MergeNode,
    ForkNode,
    JoinNode,
    CentralBufferNode,
    DataStoreNode,
    SendSignalAction,
    AcceptEventAction,
    ActivityPartition,
    Region,
    State,
    Pseudostate,
    FinalState,
    Lifeline,
    BehaviorExecutionSpecification,
    InteractionUse,
    CombinedFragment,
    StateInvariant,
    InputPin,
    OutputPin,
    ValuePin,
    InteractionOperand,
    Choice,
    Junction,
    StateFork,
    StateJoin,
    ShallowHistory,
    DeepHistory,
    EntryPoint,
    ExitPoint,
    Terminate,
}
impl ModelingElementKind {
    pub const STRUCTURAL: &'static [Self] = &[
        Self::DeploymentSpecification,
        Self::Component,
        Self::Artifact,
        Self::Node,
        Self::Device,
        Self::ExecutionEnvironment,
        Self::Collaboration,
        Self::InstanceSpecification,
    ];
    pub const PROFILES: &'static [Self] = &[Self::Profile, Self::Stereotype];
    pub const BEHAVIOR: &'static [Self] = &[
        Self::Actor,
        Self::UseCase,
        Self::Activity,
        Self::StateMachine,
        Self::Interaction,
    ];
    pub const ACTIVITY_NODES: &'static [Self] = &[
        Self::OpaqueAction,
        Self::InitialNode,
        Self::ActivityFinalNode,
        Self::FlowFinalNode,
        Self::DecisionNode,
        Self::MergeNode,
        Self::ForkNode,
        Self::JoinNode,
        Self::CentralBufferNode,
        Self::DataStoreNode,
        Self::SendSignalAction,
        Self::AcceptEventAction,
        Self::ActivityPartition,
    ];
    pub const STATE_NODES: &'static [Self] = &[
        Self::Region,
        Self::State,
        Self::Pseudostate,
        Self::FinalState,
    ];
    pub const INTERACTION_NODES: &'static [Self] = &[
        Self::DestructionOccurrenceSpecification,
        Self::Continuation,
        Self::Lifeline,
        Self::BehaviorExecutionSpecification,
        Self::InteractionUse,
        Self::CombinedFragment,
        Self::StateInvariant,
    ];
    pub const DETAILS: &'static [Self] = &[
        Self::Port,
        Self::Part,
        Self::CollaborationUse,
        Self::InputPin,
        Self::OutputPin,
        Self::ValuePin,
        Self::InteractionOperand,
    ];
    pub const PSEUDOSTATES: &'static [Self] = &[
        Self::Choice,
        Self::Junction,
        Self::StateFork,
        Self::StateJoin,
        Self::ShallowHistory,
        Self::DeepHistory,
        Self::EntryPoint,
        Self::ExitPoint,
        Self::Terminate,
    ];
    pub fn resolve_pseudostate_kind(self) -> Option<&'static str> {
        match self {
            Self::Pseudostate => Some("initial"),
            Self::Choice => Some("choice"),
            Self::Junction => Some("junction"),
            Self::StateFork => Some("fork"),
            Self::StateJoin => Some("join"),
            Self::ShallowHistory => Some("shallowHistory"),
            Self::DeepHistory => Some("deepHistory"),
            Self::EntryPoint => Some("entryPoint"),
            Self::ExitPoint => Some("exitPoint"),
            Self::Terminate => Some("terminate"),
            _ => None,
        }
    }
    pub fn requires_nested_owner(self) -> bool {
        Self::ACTIVITY_NODES.contains(&self)
            || Self::STATE_NODES.contains(&self)
            || Self::INTERACTION_NODES.contains(&self)
            || Self::DETAILS.contains(&self)
            || Self::PSEUDOSTATES.contains(&self)
    }
    pub fn requires_owner(self) -> bool {
        Self::STRUCTURAL.contains(&self) || self.requires_nested_owner()
    }
    pub const BASIC: &'static [Self] = &[Self::Class, Self::Interface];
    pub const ADVANCED: &'static [Self] = &[
        Self::Signal,
        Self::DataType,
        Self::PrimitiveType,
        Self::Enumeration,
        Self::Frame,
        Self::NaryAssociation,
        Self::AssociationClass,
        Self::Template,
    ];
    pub const PACKAGES: &'static [Self] = &[Self::Package, Self::Model, Self::Subsystem];
    pub const ANNOTATIONS: &'static [Self] = &[Self::Comment];
    pub fn label(self) -> &'static str {
        match self {
            Self::Port => "Port",
            Self::Part => "Part",
            Self::DeploymentSpecification => "Deployment Specification",
            Self::CollaborationUse => "Collaboration Use",
            Self::DestructionOccurrenceSpecification => "Destruction Occurrence",
            Self::Continuation => "Continuation",

            Self::Component => "Component",
            Self::Artifact => "Artifact",
            Self::Class => "Class",
            Self::Interface => "Interface",
            Self::Signal => "Signal",
            Self::DataType => "DataType",
            Self::PrimitiveType => "PrimitiveType",
            Self::Enumeration => "Enumeration",
            Self::Frame => "Frame",
            Self::NaryAssociation => "N-ary Association Node",
            Self::AssociationClass => "Association Class",
            Self::Template => "Template",
            Self::Package => "Package",
            Self::Model => "Model",
            Self::Subsystem => "Subsystem",
            Self::Comment => "Comment",
            Self::Node => "Node",
            Self::Device => "Device",
            Self::ExecutionEnvironment => "Execution Environment",
            Self::Collaboration => "Collaboration",
            Self::InstanceSpecification => "Instance",
            Self::Profile => "Profile",
            Self::Stereotype => "Stereotype",
            Self::Actor => "Actor",
            Self::UseCase => "Use Case",
            Self::Activity => "Activity",
            Self::StateMachine => "State Machine",
            Self::Interaction => "Interaction",
            Self::OpaqueAction => "Action",
            Self::InitialNode => "Initial Node",
            Self::ActivityFinalNode => "Activity Final",
            Self::FlowFinalNode => "Flow Final",
            Self::DecisionNode => "Decision",
            Self::MergeNode => "Merge",
            Self::ForkNode => "Fork",
            Self::JoinNode => "Join",
            Self::CentralBufferNode => "Object Node",
            Self::DataStoreNode => "Data Store",
            Self::SendSignalAction => "Send Signal",
            Self::AcceptEventAction => "Accept Event",
            Self::ActivityPartition => "Activity Partition",
            Self::Region => "State Region",
            Self::State => "State",
            Self::Pseudostate => "Initial Pseudostate",
            Self::FinalState => "Final State",
            Self::Lifeline => "Lifeline",
            Self::BehaviorExecutionSpecification => "Execution",
            Self::InteractionUse => "Interaction Use",
            Self::CombinedFragment => "Combined Fragment",
            Self::StateInvariant => "State Invariant",
            Self::InputPin => "Input Pin",
            Self::OutputPin => "Output Pin",
            Self::ValuePin => "Value Pin",
            Self::InteractionOperand => "Interaction Operand",
            Self::Choice => "Choice",
            Self::Junction => "Junction",
            Self::StateFork => "State Fork",
            Self::StateJoin => "State Join",
            Self::ShallowHistory => "Shallow History",
            Self::DeepHistory => "Deep History",
            Self::EntryPoint => "Entry Point",
            Self::ExitPoint => "Exit Point",
            Self::Terminate => "Terminate",
        }
    }
    pub fn model_kind(self) -> &'static str {
        match self {
            Self::Port => "uml.port",
            Self::Part => "uml.part",
            Self::DeploymentSpecification => "uml.deployment_specification",
            Self::CollaborationUse => "uml.collaboration_use",
            Self::DestructionOccurrenceSpecification => "uml.destruction_occurrence_specification",
            Self::Continuation => "uml.continuation",

            // Existing wire identities and Rust generator profiles remain readable.
            Self::Component => "uml.component",
            Self::Artifact => "uml.artifact",
            Self::Class => "rust.struct",
            Self::Interface => "rust.trait",
            Self::Signal => "uml.signal",
            Self::DataType => "uml.data_type",
            Self::PrimitiveType => "uml.primitive_type",
            Self::Enumeration => "uml.enumeration",
            Self::Frame => "uml.frame",
            Self::NaryAssociation => "uml.nary_association",
            Self::AssociationClass => "uml.association_class",
            Self::Template => "uml.template",
            Self::Package => "uml.package",
            Self::Model => "uml.model",
            Self::Subsystem => "uml.subsystem",
            Self::Comment => "uml.comment",
            Self::Node => "uml.node",
            Self::Device => "uml.device",
            Self::ExecutionEnvironment => "uml.execution_environment",
            Self::Collaboration => "uml.collaboration",
            Self::InstanceSpecification => "uml.instance_specification",
            Self::Profile => "uml.profile",
            Self::Stereotype => "uml.stereotype",
            Self::Actor => "uml.actor",
            Self::UseCase => "uml.use_case",
            Self::Activity => "uml.activity",
            Self::StateMachine => "uml.state_machine",
            Self::Interaction => "uml.interaction",
            Self::OpaqueAction => "uml.opaque_action",
            Self::InitialNode => "uml.initial_node",
            Self::ActivityFinalNode => "uml.activity_final_node",
            Self::FlowFinalNode => "uml.flow_final_node",
            Self::DecisionNode => "uml.decision_node",
            Self::MergeNode => "uml.merge_node",
            Self::ForkNode => "uml.fork_node",
            Self::JoinNode => "uml.join_node",
            Self::CentralBufferNode => "uml.central_buffer_node",
            Self::DataStoreNode => "uml.data_store_node",
            Self::SendSignalAction => "uml.send_signal_action",
            Self::AcceptEventAction => "uml.accept_event_action",
            Self::ActivityPartition => "uml.activity_partition",
            Self::Region => "uml.region",
            Self::State => "uml.state",
            Self::Pseudostate => "uml.pseudostate",
            Self::FinalState => "uml.final_state",
            Self::Lifeline => "uml.lifeline",
            Self::BehaviorExecutionSpecification => "uml.behavior_execution_specification",
            Self::InteractionUse => "uml.interaction_use",
            Self::CombinedFragment => "uml.combined_fragment",
            Self::StateInvariant => "uml.state_invariant",
            Self::InputPin => "uml.input_pin",
            Self::OutputPin => "uml.output_pin",
            Self::ValuePin => "uml.value_pin",
            Self::InteractionOperand => "uml.interaction_operand",
            Self::Choice => "uml.pseudostate",
            Self::Junction => "uml.pseudostate",
            Self::StateFork => "uml.pseudostate",
            Self::StateJoin => "uml.pseudostate",
            Self::ShallowHistory => "uml.pseudostate",
            Self::DeepHistory => "uml.pseudostate",
            Self::EntryPoint => "uml.pseudostate",
            Self::ExitPoint => "uml.pseudostate",
            Self::Terminate => "uml.pseudostate",
        }
    }
    pub fn default_name(self) -> &'static str {
        match self {
            Self::Port => "NewPort",
            Self::Part => "NewPart",
            Self::DeploymentSpecification => "NewDeploymentSpecification",
            Self::CollaborationUse => "NewCollaborationUse",
            Self::DestructionOccurrenceSpecification => "NewDestruction",
            Self::Continuation => "NewContinuation",

            Self::Component => "NewComponent",
            Self::Artifact => "NewArtifact",
            Self::Class => "NewClass",
            Self::Interface => "NewInterface",
            Self::Enumeration => "NewEnumeration",
            Self::Signal => "NewSignal",
            Self::DataType => "NewDataType",
            Self::PrimitiveType => "NewPrimitiveType",
            Self::Frame => "NewFrame",
            Self::NaryAssociation => "NewAssociation",
            Self::AssociationClass => "NewAssociationClass",
            Self::Template => "NewTemplate",
            Self::Package => "NewPackage",
            Self::Model => "NewModel",
            Self::Subsystem => "NewSubsystem",
            Self::Comment => "NewComment",
            Self::Node => "NewNode",
            Self::Device => "NewDevice",
            Self::ExecutionEnvironment => "NewExecutionEnvironment",
            Self::Collaboration => "NewCollaboration",
            Self::InstanceSpecification => "NewInstanceSpecification",
            Self::Profile => "NewProfile",
            Self::Stereotype => "NewStereotype",
            Self::Actor => "NewActor",
            Self::UseCase => "NewUseCase",
            Self::Activity => "NewActivity",
            Self::StateMachine => "NewStateMachine",
            Self::Interaction => "NewInteraction",
            Self::OpaqueAction => "NewOpaqueAction",
            Self::InitialNode => "NewInitialNode",
            Self::ActivityFinalNode => "NewActivityFinalNode",
            Self::FlowFinalNode => "NewFlowFinalNode",
            Self::DecisionNode => "NewDecisionNode",
            Self::MergeNode => "NewMergeNode",
            Self::ForkNode => "NewForkNode",
            Self::JoinNode => "NewJoinNode",
            Self::CentralBufferNode => "NewCentralBufferNode",
            Self::DataStoreNode => "NewDataStoreNode",
            Self::SendSignalAction => "NewSendSignalAction",
            Self::AcceptEventAction => "NewAcceptEventAction",
            Self::ActivityPartition => "NewActivityPartition",
            Self::Region => "NewRegion",
            Self::State => "NewState",
            Self::Pseudostate => "NewPseudostate",
            Self::FinalState => "NewFinalState",
            Self::Lifeline => "NewLifeline",
            Self::BehaviorExecutionSpecification => "NewBehaviorExecutionSpecification",
            Self::InteractionUse => "NewInteractionUse",
            Self::CombinedFragment => "NewCombinedFragment",
            Self::StateInvariant => "NewStateInvariant",
            Self::InputPin => "NewInputPin",
            Self::OutputPin => "NewOutputPin",
            Self::ValuePin => "NewValuePin",
            Self::InteractionOperand => "NewInteractionOperand",
            Self::Choice => "NewChoice",
            Self::Junction => "NewJunction",
            Self::StateFork => "NewStateFork",
            Self::StateJoin => "NewStateJoin",
            Self::ShallowHistory => "NewShallowHistory",
            Self::DeepHistory => "NewDeepHistory",
            Self::EntryPoint => "NewEntryPoint",
            Self::ExitPoint => "NewExitPoint",
            Self::Terminate => "NewTerminate",
        }
    }
    pub fn initial_size_in_diagram(self, diagram: &crate::Diagram) -> (f64, f64) {
        if self == Self::Lifeline {
            match crate::StructuralDiagramKind::from_diagram(diagram) {
                Some(crate::StructuralDiagramKind::Timing) => return (480.0, 180.0),
                Some(crate::StructuralDiagramKind::Communication) => return (140.0, 80.0),
                _ => {}
            }
        }
        self.initial_size()
    }

    pub fn initial_size(self) -> (f64, f64) {
        match self {
            Self::Port => (16.0, 16.0),
            Self::DestructionOccurrenceSpecification => (38.0, 38.0),
            Self::CollaborationUse => (185.0, 100.0),
            Self::InitialNode
            | Self::ActivityFinalNode
            | Self::FlowFinalNode
            | Self::Pseudostate
            | Self::FinalState => (38.0, 38.0),
            Self::DecisionNode | Self::MergeNode => (70.0, 70.0),
            Self::ForkNode | Self::JoinNode => (140.0, 20.0),
            Self::Lifeline => (140.0, 300.0),
            Self::BehaviorExecutionSpecification => (24.0, 110.0),
            Self::ActivityPartition | Self::Region | Self::CombinedFragment => (300.0, 220.0),
            Self::InputPin | Self::OutputPin | Self::ValuePin => (16.0, 16.0),
            Self::InteractionOperand => (250.0, 90.0),
            Self::StateFork | Self::StateJoin => (140.0, 20.0),
            Self::Choice => (60.0, 60.0),
            Self::Junction => (24.0, 24.0),
            Self::ShallowHistory
            | Self::DeepHistory
            | Self::EntryPoint
            | Self::ExitPoint
            | Self::Terminate => (38.0, 38.0),
            Self::Actor => (85.0, 135.0),
            Self::UseCase | Self::Collaboration => (185.0, 100.0),
            Self::Activity | Self::StateMachine => (220.0, 140.0),
            Self::Interaction => (300.0, 220.0),
            Self::Profile => (190.0, 125.0),
            Self::Interface => (120.0, 76.0),
            Self::Frame => (280.0, 205.0),
            Self::Package | Self::Model | Self::Subsystem => (190.0, 125.0),
            Self::Comment => (180.0, 92.0),
            Self::NaryAssociation => (80.0, 80.0),
            _ => (185.0, 112.0),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModelingTool {
    ProvidedInterface,
    RequiredInterface,
    Element(ModelingElementKind),
    Relation(ModelingRelationKind),
}
impl ModelingTool {
    pub const INTERFACE_FORMS: &'static [Self] =
        &[Self::ProvidedInterface, Self::RequiredInterface];
    pub fn resolve_interface_presentation(self) -> Option<&'static str> {
        match self {
            Self::ProvidedInterface => Some("provided"),
            Self::RequiredInterface => Some("required"),
            _ => None,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::ProvidedInterface => "Provided interface · visual",
            Self::RequiredInterface => "Required interface · visual",
            Self::Element(kind) => kind.label(),
            Self::Relation(kind) => kind.label(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_class_tool_has_a_unique_semantic_kind() {
        let mut kinds = std::collections::HashSet::new();
        for kind in ModelingElementKind::BASIC
            .iter()
            .chain(ModelingElementKind::ADVANCED)
            .chain(ModelingElementKind::STRUCTURAL)
            .chain(ModelingElementKind::PACKAGES)
            .chain(ModelingElementKind::ANNOTATIONS)
            .chain(ModelingElementKind::PROFILES)
            .chain(ModelingElementKind::BEHAVIOR)
            .chain(ModelingElementKind::ACTIVITY_NODES)
            .chain(ModelingElementKind::STATE_NODES)
            .chain(ModelingElementKind::INTERACTION_NODES)
            .chain(ModelingElementKind::DETAILS)
        {
            assert!(kinds.insert(kind.model_kind()));
        }
        for kind in ModelingRelationKind::BASIC
            .iter()
            .chain(ModelingRelationKind::ADVANCED)
            .chain(ModelingRelationKind::PACKAGES)
            .chain(ModelingRelationKind::STRUCTURAL)
            .chain(ModelingRelationKind::BEHAVIOR)
        {
            assert!(kinds.insert(kind.model_kind()));
        }
        assert_eq!(kinds.len(), 88);
    }

}
