use super::command::{
    ModelingElementKind, ModelingRelationKind, ModelingTool, StructuralDiagramKind,
};

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
    pub fn initial_properties(self) -> std::collections::BTreeMap<String, serde_json::Value> {
        let mut properties = std::collections::BTreeMap::new();
        if matches!(self, Self::Part | Self::Port) {
            properties.insert("aggregation".into(), serde_json::json!("composite"));
        }
        if let Some(kind) = self.resolve_pseudostate_kind() {
            properties.insert("kind".into(), serde_json::json!(kind));
        }
        properties
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
            match StructuralDiagramKind::from_diagram(diagram) {
                Some(StructuralDiagramKind::Timing) => return (480.0, 180.0),
                Some(StructuralDiagramKind::Communication) => return (140.0, 80.0),
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

impl ModelingRelationKind {
    pub const MESSAGES: &'static [Self] = &[
        Self::MessageSynchronous,
        Self::MessageAsynchronous,
        Self::MessageReply,
        Self::MessageFound,
        Self::MessageLost,
        Self::GeneralOrdering,
    ];
    pub fn resolve_message_sort(self) -> Option<&'static str> {
        match self {
            Self::MessageSynchronous | Self::MessageFound | Self::MessageLost => Some("synchCall"),
            Self::MessageAsynchronous => Some("asynchCall"),
            Self::MessageReply => Some("reply"),
            _ => None,
        }
    }

    pub fn resolve_message_kind(self) -> Option<&'static str> {
        match self {
            Self::MessageFound => Some("found"),
            Self::MessageLost => Some("lost"),
            _ => None,
        }
    }
    pub fn uses_occurrence_anchors(self) -> bool {
        self.resolve_message_sort().is_some() || self == Self::GeneralOrdering
    }

    pub const BASIC: &'static [Self] = &[
        Self::Association,
        Self::DirectedAssociation,
        Self::Aggregation,
        Self::Composition,
        Self::Dependency,
        Self::Generalization,
        Self::InterfaceRealization,
    ];
    pub const ADVANCED: &'static [Self] = &[
        Self::TemplateBinding,
        Self::Realization,
        Self::AssociationClassAttachment,
    ];
    pub const BEHAVIOR: &'static [Self] = &[
        Self::Include,
        Self::Extend,
        Self::ControlFlow,
        Self::ObjectFlow,
        Self::Transition,
    ];
    pub const STRUCTURAL: &'static [Self] = &[
        Self::ElementImport,
        Self::ProfileApplication,
        Self::Extension,
        Self::Connector,
        Self::NoteLink,
        Self::ConstraintLink,
        Self::Usage,
        Self::Abstraction,
        Self::PackageImport,
        Self::PackageMerge,
        Self::Deployment,
        Self::Manifestation,
    ];
    pub const PACKAGES: &'static [Self] = &[Self::Containment];
    pub fn label(self) -> &'static str {
        match self {
            Self::ElementImport => "Element import",
            Self::ProfileApplication => "Profile application · visual",
            Self::Extension => "Extension · visual",
            Self::Connector => "Connector · visual",
            Self::NoteLink => "Comment link · visual",
            Self::ConstraintLink => "Constraint link · visual",
            Self::MessageSynchronous => "Synchronous message",
            Self::MessageAsynchronous => "Asynchronous message",
            Self::MessageReply => "Reply message",
            Self::MessageFound => "Found message · visual",
            Self::MessageLost => "Lost message · visual",
            Self::GeneralOrdering => "General ordering · visual",
            Self::AssociationClassAttachment => "Association-class attachment",
            Self::Association => "Association",
            Self::DirectedAssociation => "Directed Association",
            Self::Aggregation => "Aggregation",
            Self::Composition => "Composition",
            Self::Dependency => "Dependency",
            Self::Generalization => "Generalization",
            Self::InterfaceRealization => "Interface Realization",
            Self::TemplateBinding => "Template Binding",
            Self::Realization => "Realization",
            Self::Containment => "Containment",
            Self::Usage => "Usage",
            Self::Abstraction => "Abstraction",
            Self::Include => "Include",
            Self::Extend => "Extend",
            Self::PackageImport => "Package Import",
            Self::PackageMerge => "Package Merge",
            Self::Deployment => "Deployment",
            Self::Manifestation => "Manifestation",
            Self::ControlFlow => "Control Flow",
            Self::ObjectFlow => "Object Flow",
            Self::Transition => "Transition",
        }
    }
    pub fn model_kind(self) -> &'static str {
        match self {
            Self::ElementImport => "uml.element_import",
            Self::ProfileApplication => "uml.profile_application",
            Self::Extension => "uml.extension",
            Self::Connector => "uml.connector",
            Self::NoteLink => "uml.note_link",
            Self::ConstraintLink => "uml.constraint_link",
            Self::MessageSynchronous
            | Self::MessageAsynchronous
            | Self::MessageReply
            | Self::MessageFound
            | Self::MessageLost => "uml.message",
            Self::GeneralOrdering => "uml.general_ordering",
            Self::AssociationClassAttachment => "view.association_class_attachment",
            Self::Association => "uml.association",
            Self::DirectedAssociation => "uml.directed_association",
            Self::Aggregation => "uml.aggregation",
            Self::Composition => "uml.composition",
            Self::Dependency => "uml.dependency",
            Self::Generalization => "uml.generalization",
            Self::InterfaceRealization => "uml.interface_realization",
            Self::TemplateBinding => "uml.template_binding",
            Self::Realization => "uml.realization",
            Self::Containment => "uml.containment",
            Self::Usage => "uml.usage",
            Self::Abstraction => "uml.abstraction",
            Self::Include => "uml.include",
            Self::Extend => "uml.extend",
            Self::PackageImport => "uml.package_import",
            Self::PackageMerge => "uml.package_merge",
            Self::Deployment => "uml.deployment",
            Self::Manifestation => "uml.manifestation",
            Self::ControlFlow => "uml.control_flow",
            Self::ObjectFlow => "uml.object_flow",
            Self::Transition => "uml.transition",
        }
    }
    /// The first click is Relation.from, not necessarily the arrowhead end.
    pub fn endpoint_instruction(self) -> &'static str {
        match self {
            Self::ElementImport => "click importing namespace, then imported element",
            Self::ProfileApplication => "click applying package, then profile (visual only)",
            Self::Extension => "click stereotype, then metaclass (filled triangle at metaclass)",
            Self::Connector => "click a role or port, then another role or port (visual only)",
            Self::NoteLink => "click comment, then annotated view (visual only)",
            Self::ConstraintLink => "click constraint, then constrained view (visual only)",
            Self::MessageSynchronous | Self::MessageAsynchronous | Self::MessageReply => {
                "click the source lifeline at send position, then the target at receive position"
            }
            Self::MessageFound => "click an empty send position, then the receiving Lifeline",
            Self::MessageLost => "click the sending Lifeline, then an empty receive position",
            Self::GeneralOrdering => {
                "click the before occurrence position, then the after position (visual only)"
            }
            Self::AssociationClassAttachment => {
                "click the association path, then its classifier box (visual attachment)"
            }
            Self::Association => "click either first class, then the second (no arrow)",
            Self::DirectedAssociation => "click source, then navigable target (arrow at target)",
            Self::Aggregation => "click the whole, then the part (hollow diamond at first)",
            Self::Composition => "click the whole, then the part (filled diamond at first)",
            Self::Dependency => "click dependent client, then supplier (arrow at target)",
            Self::Generalization => "click subtype, then supertype (triangle at target)",
            Self::InterfaceRealization => {
                "click implementing class, then interface (triangle at target)"
            }
            Self::TemplateBinding => "click bound element, then template (arrow at target)",
            Self::Realization => {
                "click implementing classifier, then specification (triangle at target)"
            }
            Self::Containment => "click container, then contained element (circle at first)",
            Self::Usage | Self::Abstraction => "click client, then supplier",
            Self::Include => "click including use case, then included use case",
            Self::Extend => "click extending use case, then extended use case",
            Self::PackageImport => "click importing namespace, then imported package",
            Self::PackageMerge => "click receiving package, then merged package",
            Self::Deployment => "click deployed artifact, then deployment target",
            Self::Manifestation => "click artifact, then manifested model element",
            Self::ControlFlow | Self::ObjectFlow => {
                "click source activity node, then target in the same activity"
            }
            Self::Transition => "click source vertex, then target in the same region",
        }
    }

    pub fn is_visual_only(self) -> bool {
        matches!(
            self,
            Self::ProfileApplication
                | Self::Extension
                | Self::Connector
                | Self::NoteLink
                | Self::ConstraintLink
                | Self::AssociationClassAttachment
                | Self::MessageSynchronous
                | Self::MessageAsynchronous
                | Self::MessageReply
                | Self::MessageFound
                | Self::MessageLost
                | Self::GeneralOrdering
        )
    }
}

impl StructuralDiagramKind {
    pub const ALL_DIAGRAMS: [Self; 14] = [
        Self::Class,
        Self::Object,
        Self::Package,
        Self::Component,
        Self::CompositeStructure,
        Self::Deployment,
        Self::Profile,
        Self::UseCase,
        Self::Activity,
        Self::StateMachine,
        Self::Sequence,
        Self::Communication,
        Self::InteractionOverview,
        Self::Timing,
    ];
    pub const ALL: [Self; 7] = [
        Self::Class,
        Self::Object,
        Self::Package,
        Self::Component,
        Self::CompositeStructure,
        Self::Deployment,
        Self::Profile,
    ];
    pub const fn uml_di_type(self) -> &'static str {
        match self {
            Self::Class => "UMLClassDiagram",
            Self::Object => "UMLObjectDiagram",
            Self::Package => "UMLPackageDiagram",
            Self::Component => "UMLComponentDiagram",
            Self::CompositeStructure => "UMLCompositeStructureDiagram",
            Self::Deployment => "UMLDeploymentDiagram",
            Self::Profile => "UMLProfileDiagram",
            Self::UseCase => "UMLUseCaseDiagram",
            Self::Activity => "UMLActivityDiagram",
            Self::StateMachine => "UMLStateMachineDiagram",
            Self::Sequence | Self::Communication | Self::InteractionOverview | Self::Timing => {
                "UMLInteractionDiagram"
            }
        }
    }
    pub const fn interaction_kind(self) -> Option<&'static str> {
        match self {
            Self::Sequence => Some("sequence"),
            Self::Communication => Some("communication"),
            Self::InteractionOverview => Some("overview"),
            Self::Timing => Some("timing"),
            _ => None,
        }
    }
    pub const fn context_metaclass(self) -> Option<&'static str> {
        match self {
            Self::Activity => Some("Activity"),
            Self::StateMachine => Some("StateMachine"),
            Self::Sequence | Self::Communication | Self::InteractionOverview | Self::Timing => {
                Some("Interaction")
            }
            _ => None,
        }
    }
    pub fn from_diagram(diagram: &crate::Diagram) -> Option<Self> {
        if diagram.uml_di_type == "UMLInteractionDiagram" {
            return match diagram.interaction_kind.as_deref().unwrap_or("sequence") {
                "sequence" => Some(Self::Sequence),
                "communication" => Some(Self::Communication),
                "overview" => Some(Self::InteractionOverview),
                "timing" => Some(Self::Timing),
                _ => None,
            };
        }
        Self::from_uml_di_type(&diagram.uml_di_type)
    }
    pub fn from_uml_di_type(value: &str) -> Option<Self> {
        Self::ALL_DIAGRAMS
            .into_iter()
            .find(|kind| kind.uml_di_type() == value)
    }
}
