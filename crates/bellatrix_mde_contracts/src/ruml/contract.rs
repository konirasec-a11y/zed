use serde::{Deserialize, Serialize};

pub trait ContractType {
    const TYPE_NAME: &'static str;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Contract {
    CentralBufferNode(crate::ruml::central_buffer_node::CentralBufferNode),
    ChangeEvent(crate::ruml::change_event::ChangeEvent),
    Class(crate::ruml::class::Class),
    Classifier(crate::ruml::classifier::Classifier),
    ClassifierTemplateParameter(
        crate::ruml::classifier_template_parameter::ClassifierTemplateParameter,
    ),
    Clause(crate::ruml::clause::Clause),
    ClearAssociationAction(crate::ruml::clear_association_action::ClearAssociationAction),
    ClearStructuralFeatureAction(
        crate::ruml::clear_structural_feature_action::ClearStructuralFeatureAction,
    ),
    ClearVariableAction(crate::ruml::clear_variable_action::ClearVariableAction),
    Collaboration(crate::ruml::collaboration::Collaboration),
    CollaborationUse(crate::ruml::collaboration_use::CollaborationUse),
    CombinedFragment(crate::ruml::combined_fragment::CombinedFragment),
    Comment(crate::ruml::comment::Comment),
    CommunicationPath(crate::ruml::communication_path::CommunicationPath),
    Component(crate::ruml::component::Component),
    ComponentRealization(crate::ruml::component_realization::ComponentRealization),
    ConditionalNode(crate::ruml::conditional_node::ConditionalNode),
    ConnectableElement(crate::ruml::connectable_element::ConnectableElement),
    ConnectableElementTemplateParameter(
        crate::ruml::connectable_element_template_parameter::ConnectableElementTemplateParameter,
    ),
    ConnectionPointReference(crate::ruml::connection_point_reference::ConnectionPointReference),
    Connector(crate::ruml::connector::Connector),
    ConnectorEnd(crate::ruml::connector_end::ConnectorEnd),
    ConsiderIgnoreFragment(crate::ruml::consider_ignore_fragment::ConsiderIgnoreFragment),
    Constraint(crate::ruml::constraint::Constraint),
    Continuation(crate::ruml::continuation::Continuation),
    ControlFlow(crate::ruml::control_flow::ControlFlow),
    ControlNode(crate::ruml::control_node::ControlNode),
    CreateLinkAction(crate::ruml::create_link_action::CreateLinkAction),
    CreateLinkObjectAction(crate::ruml::create_link_object_action::CreateLinkObjectAction),
    CreateObjectAction(crate::ruml::create_object_action::CreateObjectAction),
    CreationEvent(crate::ruml::creation_event::CreationEvent),
    DataStoreNode(crate::ruml::data_store_node::DataStoreNode),
    DataType(crate::ruml::data_type::DataType),
    DecisionNode(crate::ruml::decision_node::DecisionNode),
    Dependency(crate::ruml::dependency::Dependency),
    DeployedArtifact(crate::ruml::deployed_artifact::DeployedArtifact),
    Deployment(crate::ruml::deployment::Deployment),
    DeploymentSpecification(crate::ruml::deployment_specification::DeploymentSpecification),
    DeploymentTarget(crate::ruml::deployment_target::DeploymentTarget),
    DestroyLinkAction(crate::ruml::destroy_link_action::DestroyLinkAction),
    DestroyObjectAction(crate::ruml::destroy_object_action::DestroyObjectAction),
    DestructionEvent(crate::ruml::destruction_event::DestructionEvent),
    Device(crate::ruml::device::Device),
    DirectedRelationship(crate::ruml::directed_relationship::DirectedRelationship),
    Duration(crate::ruml::duration::Duration),
    DurationConstraint(crate::ruml::duration_constraint::DurationConstraint),
    DurationInterval(crate::ruml::duration_interval::DurationInterval),
    DurationObservation(crate::ruml::duration_observation::DurationObservation),
    EAnnotation(crate::ruml::eannotation::EAnnotation),
    EAttribute(crate::ruml::eattribute::EAttribute),
    EClass(crate::ruml::eclass::EClass),
    EClassifier(crate::ruml::eclassifier::EClassifier),
    EDataType(crate::ruml::edata_type::EDataType),
    EEnum(crate::ruml::eenum::EEnum),
    EEnumLiteral(crate::ruml::eenum_literal::EEnumLiteral),
    EGenericType(crate::ruml::egeneric_type::EGenericType),
    Element(crate::ruml::element::Element),
    ElementImport(crate::ruml::element_import::ElementImport),
    EModelElement(crate::ruml::emodel_element::EModelElement),
    ENamedElement(crate::ruml::enamed_element::ENamedElement),
    EncapsulatedClassifier(crate::ruml::encapsulated_classifier::EncapsulatedClassifier),
    Enumeration(crate::ruml::enumeration::Enumeration),
    EnumerationLiteral(crate::ruml::enumeration_literal::EnumerationLiteral),
    EObject(crate::ruml::eobject::EObject),
    EOperation(crate::ruml::eoperation::EOperation),
    EPackage(crate::ruml::epackage::EPackage),
    EParameter(crate::ruml::eparameter::EParameter),
    EReference(crate::ruml::ereference::EReference),
    EStructuralFeature(crate::ruml::estructural_feature::EStructuralFeature),
    ETypeParameter(crate::ruml::etype_parameter::ETypeParameter),
    ETypedElement(crate::ruml::etyped_element::ETypedElement),
    Event(crate::ruml::event::Event),
    ExceptionHandler(crate::ruml::exception_handler::ExceptionHandler),
    ExecutableNode(crate::ruml::executable_node::ExecutableNode),
    ExecutionEnvironment(crate::ruml::execution_environment::ExecutionEnvironment),
    ExecutionEvent(crate::ruml::execution_event::ExecutionEvent),
    ExecutionOccurrenceSpecification(
        crate::ruml::execution_occurrence_specification::ExecutionOccurrenceSpecification,
    ),
    ExecutionSpecification(crate::ruml::execution_specification::ExecutionSpecification),
    ExpansionNode(crate::ruml::expansion_node::ExpansionNode),
    ExpansionRegion(crate::ruml::expansion_region::ExpansionRegion),
    Expression(crate::ruml::expression::Expression),
    Extend(crate::ruml::extend::Extend),
    Extension(crate::ruml::extension::Extension),
    ExtensionEnd(crate::ruml::extension_end::ExtensionEnd),
    ExtensionPoint(crate::ruml::extension_point::ExtensionPoint),
    Feature(crate::ruml::feature::Feature),
    FinalNode(crate::ruml::final_node::FinalNode),
    FinalState(crate::ruml::final_state::FinalState),
    FlowFinalNode(crate::ruml::flow_final_node::FlowFinalNode),
    ForkNode(crate::ruml::fork_node::ForkNode),
    FunctionBehavior(crate::ruml::function_behavior::FunctionBehavior),
    Gate(crate::ruml::gate::Gate),
    GeneralOrdering(crate::ruml::general_ordering::GeneralOrdering),
    Generalization(crate::ruml::generalization::Generalization),
    GeneralizationSet(crate::ruml::generalization_set::GeneralizationSet),
    Image(crate::ruml::image::Image),
    Include(crate::ruml::include::Include),
    InformationFlow(crate::ruml::information_flow::InformationFlow),
    InformationItem(crate::ruml::information_item::InformationItem),
    InitialNode(crate::ruml::initial_node::InitialNode),
    InputPin(crate::ruml::input_pin::InputPin),
    InstanceSpecification(crate::ruml::instance_specification::InstanceSpecification),
    InstanceValue(crate::ruml::instance_value::InstanceValue),
    Interaction(crate::ruml::interaction::Interaction),
    InteractionConstraint(crate::ruml::interaction_constraint::InteractionConstraint),
    InteractionFragment(crate::ruml::interaction_fragment::InteractionFragment),
    InteractionOperand(crate::ruml::interaction_operand::InteractionOperand),
    InteractionUse(crate::ruml::interaction_use::InteractionUse),
    Interface(crate::ruml::interface::Interface),
    InterfaceRealization(crate::ruml::interface_realization::InterfaceRealization),
    InterruptibleActivityRegion(
        crate::ruml::interruptible_activity_region::InterruptibleActivityRegion,
    ),
    Interval(crate::ruml::interval::Interval),
    IntervalConstraint(crate::ruml::interval_constraint::IntervalConstraint),
    InvocationAction(crate::ruml::invocation_action::InvocationAction),
    JoinNode(crate::ruml::join_node::JoinNode),
    Lifeline(crate::ruml::lifeline::Lifeline),
    LinkAction(crate::ruml::link_action::LinkAction),
    LinkEndCreationData(crate::ruml::link_end_creation_data::LinkEndCreationData),
    LinkEndData(crate::ruml::link_end_data::LinkEndData),
    LinkEndDestructionData(crate::ruml::link_end_destruction_data::LinkEndDestructionData),
    LiteralBoolean(crate::ruml::literal_boolean::LiteralBoolean),
    LiteralInteger(crate::ruml::literal_integer::LiteralInteger),
    LiteralNull(crate::ruml::literal_null::LiteralNull),
    LiteralSpecification(crate::ruml::literal_specification::LiteralSpecification),
    LiteralString(crate::ruml::literal_string::LiteralString),
    LiteralUnlimitedNatural(crate::ruml::literal_unlimited_natural::LiteralUnlimitedNatural),
    LoopNode(crate::ruml::loop_node::LoopNode),
    Manifestation(crate::ruml::manifestation::Manifestation),
    MergeNode(crate::ruml::merge_node::MergeNode),
    Message(crate::ruml::message::Message),
    MessageEnd(crate::ruml::message_end::MessageEnd),
    MessageEvent(crate::ruml::message_event::MessageEvent),
    MessageOccurrenceSpecification(
        crate::ruml::message_occurrence_specification::MessageOccurrenceSpecification,
    ),
    Model(crate::ruml::model::Model),
    MultiplicityElement(crate::ruml::multiplicity_element::MultiplicityElement),
    NamedElement(crate::ruml::named_element::NamedElement),
    Namespace(crate::ruml::namespace::Namespace),
    Node(crate::ruml::node::Node),
    ObjectFlow(crate::ruml::object_flow::ObjectFlow),
    ObjectNode(crate::ruml::object_node::ObjectNode),
    Observation(crate::ruml::observation::Observation),
    OccurrenceSpecification(crate::ruml::occurrence_specification::OccurrenceSpecification),
    OpaqueAction(crate::ruml::opaque_action::OpaqueAction),
    OpaqueBehavior(crate::ruml::opaque_behavior::OpaqueBehavior),
    OpaqueExpression(crate::ruml::opaque_expression::OpaqueExpression),
    Operation(crate::ruml::operation::Operation),
    OperationTemplateParameter(
        crate::ruml::operation_template_parameter::OperationTemplateParameter,
    ),
    OutputPin(crate::ruml::output_pin::OutputPin),
    Package(crate::ruml::package::Package),
    PackageImport(crate::ruml::package_import::PackageImport),
    PackageMerge(crate::ruml::package_merge::PackageMerge),
    PackageableElement(crate::ruml::packageable_element::PackageableElement),
    Parameter(crate::ruml::parameter::Parameter),
    ParameterSet(crate::ruml::parameter_set::ParameterSet),
    ParameterableElement(crate::ruml::parameterable_element::ParameterableElement),
    PartDecomposition(crate::ruml::part_decomposition::PartDecomposition),
    Pin(crate::ruml::pin::Pin),
    Port(crate::ruml::port::Port),
    PrimitiveType(crate::ruml::primitive_type::PrimitiveType),
    Profile(crate::ruml::profile::Profile),
    ProfileApplication(crate::ruml::profile_application::ProfileApplication),
    Property(crate::ruml::property::Property),
    ProtocolConformance(crate::ruml::protocol_conformance::ProtocolConformance),
    ProtocolStateMachine(crate::ruml::protocol_state_machine::ProtocolStateMachine),
    ProtocolTransition(crate::ruml::protocol_transition::ProtocolTransition),
    Pseudostate(crate::ruml::pseudostate::Pseudostate),
    QualifierValue(crate::ruml::qualifier_value::QualifierValue),
    RaiseExceptionAction(crate::ruml::raise_exception_action::RaiseExceptionAction),
    ReadExtentAction(crate::ruml::read_extent_action::ReadExtentAction),
    ReadIsClassifiedObjectAction(
        crate::ruml::read_is_classified_object_action::ReadIsClassifiedObjectAction,
    ),
    ReadLinkAction(crate::ruml::read_link_action::ReadLinkAction),
    ReadLinkObjectEndAction(crate::ruml::read_link_object_end_action::ReadLinkObjectEndAction),
    ReadLinkObjectEndQualifierAction(
        crate::ruml::read_link_object_end_qualifier_action::ReadLinkObjectEndQualifierAction,
    ),
    ReadSelfAction(crate::ruml::read_self_action::ReadSelfAction),
    ReadStructuralFeatureAction(
        crate::ruml::read_structural_feature_action::ReadStructuralFeatureAction,
    ),
    ReadVariableAction(crate::ruml::read_variable_action::ReadVariableAction),
    Realization(crate::ruml::realization::Realization),
    ReceiveOperationEvent(crate::ruml::receive_operation_event::ReceiveOperationEvent),
    ReceiveSignalEvent(crate::ruml::receive_signal_event::ReceiveSignalEvent),
    Reception(crate::ruml::reception::Reception),
    ReclassifyObjectAction(crate::ruml::reclassify_object_action::ReclassifyObjectAction),
    RedefinableElement(crate::ruml::redefinable_element::RedefinableElement),
    RedefinableTemplateSignature(
        crate::ruml::redefinable_template_signature::RedefinableTemplateSignature,
    ),
    ReduceAction(crate::ruml::reduce_action::ReduceAction),
    Region(crate::ruml::region::Region),
    Relationship(crate::ruml::relationship::Relationship),
    RemoveStructuralFeatureValueAction(
        crate::ruml::remove_structural_feature_value_action::RemoveStructuralFeatureValueAction,
    ),
    RemoveVariableValueAction(crate::ruml::remove_variable_value_action::RemoveVariableValueAction),
    ReplyAction(crate::ruml::reply_action::ReplyAction),
    SendObjectAction(crate::ruml::send_object_action::SendObjectAction),
    SendOperationEvent(crate::ruml::send_operation_event::SendOperationEvent),
    SendSignalAction(crate::ruml::send_signal_action::SendSignalAction),
    SendSignalEvent(crate::ruml::send_signal_event::SendSignalEvent),
    SequenceNode(crate::ruml::sequence_node::SequenceNode),
    Signal(crate::ruml::signal::Signal),
    SignalEvent(crate::ruml::signal_event::SignalEvent),
    Slot(crate::ruml::slot::Slot),
    StartClassifierBehaviorAction(
        crate::ruml::start_classifier_behavior_action::StartClassifierBehaviorAction,
    ),
    State(crate::ruml::state::State),
    StateInvariant(crate::ruml::state_invariant::StateInvariant),
    StateMachine(crate::ruml::state_machine::StateMachine),
    Stereotype(crate::ruml::stereotype::Stereotype),
    StringExpression(crate::ruml::string_expression::StringExpression),
    StructuralFeature(crate::ruml::structural_feature::StructuralFeature),
    StructuralFeatureAction(crate::ruml::structural_feature_action::StructuralFeatureAction),
    StructuredActivityNode(crate::ruml::structured_activity_node::StructuredActivityNode),
    StructuredClassifier(crate::ruml::structured_classifier::StructuredClassifier),
    Substitution(crate::ruml::substitution::Substitution),
    TemplateBinding(crate::ruml::template_binding::TemplateBinding),
    TemplateParameter(crate::ruml::template_parameter::TemplateParameter),
    TemplateParameterSubstitution(
        crate::ruml::template_parameter_substitution::TemplateParameterSubstitution,
    ),
    TemplateSignature(crate::ruml::template_signature::TemplateSignature),
    TemplateableElement(crate::ruml::templateable_element::TemplateableElement),
    TestIdentityAction(crate::ruml::test_identity_action::TestIdentityAction),
    TimeConstraint(crate::ruml::time_constraint::TimeConstraint),
    TimeEvent(crate::ruml::time_event::TimeEvent),
    TimeExpression(crate::ruml::time_expression::TimeExpression),
    TimeInterval(crate::ruml::time_interval::TimeInterval),
    TimeObservation(crate::ruml::time_observation::TimeObservation),
    Transition(crate::ruml::transition::Transition),
    Trigger(crate::ruml::trigger::Trigger),
    Type(crate::ruml::type_::Type),
    TypedElement(crate::ruml::typed_element::TypedElement),
    UnmarshallAction(crate::ruml::unmarshall_action::UnmarshallAction),
    Usage(crate::ruml::usage::Usage),
    UseCase(crate::ruml::use_case::UseCase),
    ValuePin(crate::ruml::value_pin::ValuePin),
    ValueSpecification(crate::ruml::value_specification::ValueSpecification),
    ValueSpecificationAction(crate::ruml::value_specification_action::ValueSpecificationAction),
    Variable(crate::ruml::variable::Variable),
    VariableAction(crate::ruml::variable_action::VariableAction),
    Vertex(crate::ruml::vertex::Vertex),
    WriteLinkAction(crate::ruml::write_link_action::WriteLinkAction),
    WriteStructuralFeatureAction(
        crate::ruml::write_structural_feature_action::WriteStructuralFeatureAction,
    ),
    WriteVariableAction(crate::ruml::write_variable_action::WriteVariableAction),
    Abstraction(crate::ruml::abstraction::Abstraction),
    AcceptCallAction(crate::ruml::accept_call_action::AcceptCallAction),
    AcceptEventAction(crate::ruml::accept_event_action::AcceptEventAction),
    Action(crate::ruml::action::Action),
    ActionExecutionSpecification(
        crate::ruml::action_execution_specification::ActionExecutionSpecification,
    ),
    ActionInputPin(crate::ruml::action_input_pin::ActionInputPin),
    Activity(crate::ruml::activity::Activity),
    ActivityEdge(crate::ruml::activity_edge::ActivityEdge),
    ActivityFinalNode(crate::ruml::activity_final_node::ActivityFinalNode),
    ActivityGroup(crate::ruml::activity_group::ActivityGroup),
    ActivityNode(crate::ruml::activity_node::ActivityNode),
    ActivityParameterNode(crate::ruml::activity_parameter_node::ActivityParameterNode),
    ActivityPartition(crate::ruml::activity_partition::ActivityPartition),
    Actor(crate::ruml::actor::Actor),
    AddStructuralFeatureValueAction(
        crate::ruml::add_structural_feature_value_action::AddStructuralFeatureValueAction,
    ),
    AddVariableValueAction(crate::ruml::add_variable_value_action::AddVariableValueAction),
    AnyReceiveEvent(crate::ruml::any_receive_event::AnyReceiveEvent),
    Artifact(crate::ruml::artifact::Artifact),
    Association(crate::ruml::association::Association),
    AssociationClass(crate::ruml::association_class::AssociationClass),
    Behavior(crate::ruml::behavior::Behavior),
    BehaviorExecutionSpecification(
        crate::ruml::behavior_execution_specification::BehaviorExecutionSpecification,
    ),
    BehavioralFeature(crate::ruml::behavioral_feature::BehavioralFeature),
    BehavioredClassifier(crate::ruml::behaviored_classifier::BehavioredClassifier),
    BroadcastSignalAction(crate::ruml::broadcast_signal_action::BroadcastSignalAction),
    CallAction(crate::ruml::call_action::CallAction),
    CallBehaviorAction(crate::ruml::call_behavior_action::CallBehaviorAction),
    CallEvent(crate::ruml::call_event::CallEvent),
    CallOperationAction(crate::ruml::call_operation_action::CallOperationAction),
}

impl Contract {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::CentralBufferNode(..) => "CentralBufferNode",
            Self::ChangeEvent(..) => "ChangeEvent",
            Self::Class(..) => "Class",
            Self::Classifier(..) => "Classifier",
            Self::ClassifierTemplateParameter(..) => "ClassifierTemplateParameter",
            Self::Clause(..) => "Clause",
            Self::ClearAssociationAction(..) => "ClearAssociationAction",
            Self::ClearStructuralFeatureAction(..) => "ClearStructuralFeatureAction",
            Self::ClearVariableAction(..) => "ClearVariableAction",
            Self::Collaboration(..) => "Collaboration",
            Self::CollaborationUse(..) => "CollaborationUse",
            Self::CombinedFragment(..) => "CombinedFragment",
            Self::Comment(..) => "Comment",
            Self::CommunicationPath(..) => "CommunicationPath",
            Self::Component(..) => "Component",
            Self::ComponentRealization(..) => "ComponentRealization",
            Self::ConditionalNode(..) => "ConditionalNode",
            Self::ConnectableElement(..) => "ConnectableElement",
            Self::ConnectableElementTemplateParameter(..) => "ConnectableElementTemplateParameter",
            Self::ConnectionPointReference(..) => "ConnectionPointReference",
            Self::Connector(..) => "Connector",
            Self::ConnectorEnd(..) => "ConnectorEnd",
            Self::ConsiderIgnoreFragment(..) => "ConsiderIgnoreFragment",
            Self::Constraint(..) => "Constraint",
            Self::Continuation(..) => "Continuation",
            Self::ControlFlow(..) => "ControlFlow",
            Self::ControlNode(..) => "ControlNode",
            Self::CreateLinkAction(..) => "CreateLinkAction",
            Self::CreateLinkObjectAction(..) => "CreateLinkObjectAction",
            Self::CreateObjectAction(..) => "CreateObjectAction",
            Self::CreationEvent(..) => "CreationEvent",
            Self::DataStoreNode(..) => "DataStoreNode",
            Self::DataType(..) => "DataType",
            Self::DecisionNode(..) => "DecisionNode",
            Self::Dependency(..) => "Dependency",
            Self::DeployedArtifact(..) => "DeployedArtifact",
            Self::Deployment(..) => "Deployment",
            Self::DeploymentSpecification(..) => "DeploymentSpecification",
            Self::DeploymentTarget(..) => "DeploymentTarget",
            Self::DestroyLinkAction(..) => "DestroyLinkAction",
            Self::DestroyObjectAction(..) => "DestroyObjectAction",
            Self::DestructionEvent(..) => "DestructionEvent",
            Self::Device(..) => "Device",
            Self::DirectedRelationship(..) => "DirectedRelationship",
            Self::Duration(..) => "Duration",
            Self::DurationConstraint(..) => "DurationConstraint",
            Self::DurationInterval(..) => "DurationInterval",
            Self::DurationObservation(..) => "DurationObservation",
            Self::EAnnotation(..) => "EAnnotation",
            Self::EAttribute(..) => "EAttribute",
            Self::EClass(..) => "EClass",
            Self::EClassifier(..) => "EClassifier",
            Self::EDataType(..) => "EDataType",
            Self::EEnum(..) => "EEnum",
            Self::EEnumLiteral(..) => "EEnumLiteral",
            Self::EGenericType(..) => "EGenericType",
            Self::Element(..) => "Element",
            Self::ElementImport(..) => "ElementImport",
            Self::EModelElement(..) => "EModelElement",
            Self::ENamedElement(..) => "ENamedElement",
            Self::EncapsulatedClassifier(..) => "EncapsulatedClassifier",
            Self::Enumeration(..) => "Enumeration",
            Self::EnumerationLiteral(..) => "EnumerationLiteral",
            Self::EObject(..) => "EObject",
            Self::EOperation(..) => "EOperation",
            Self::EPackage(..) => "EPackage",
            Self::EParameter(..) => "EParameter",
            Self::EReference(..) => "EReference",
            Self::EStructuralFeature(..) => "EStructuralFeature",
            Self::ETypeParameter(..) => "ETypeParameter",
            Self::ETypedElement(..) => "ETypedElement",
            Self::Event(..) => "Event",
            Self::ExceptionHandler(..) => "ExceptionHandler",
            Self::ExecutableNode(..) => "ExecutableNode",
            Self::ExecutionEnvironment(..) => "ExecutionEnvironment",
            Self::ExecutionEvent(..) => "ExecutionEvent",
            Self::ExecutionOccurrenceSpecification(..) => "ExecutionOccurrenceSpecification",
            Self::ExecutionSpecification(..) => "ExecutionSpecification",
            Self::ExpansionNode(..) => "ExpansionNode",
            Self::ExpansionRegion(..) => "ExpansionRegion",
            Self::Expression(..) => "Expression",
            Self::Extend(..) => "Extend",
            Self::Extension(..) => "Extension",
            Self::ExtensionEnd(..) => "ExtensionEnd",
            Self::ExtensionPoint(..) => "ExtensionPoint",
            Self::Feature(..) => "Feature",
            Self::FinalNode(..) => "FinalNode",
            Self::FinalState(..) => "FinalState",
            Self::FlowFinalNode(..) => "FlowFinalNode",
            Self::ForkNode(..) => "ForkNode",
            Self::FunctionBehavior(..) => "FunctionBehavior",
            Self::Gate(..) => "Gate",
            Self::GeneralOrdering(..) => "GeneralOrdering",
            Self::Generalization(..) => "Generalization",
            Self::GeneralizationSet(..) => "GeneralizationSet",
            Self::Image(..) => "Image",
            Self::Include(..) => "Include",
            Self::InformationFlow(..) => "InformationFlow",
            Self::InformationItem(..) => "InformationItem",
            Self::InitialNode(..) => "InitialNode",
            Self::InputPin(..) => "InputPin",
            Self::InstanceSpecification(..) => "InstanceSpecification",
            Self::InstanceValue(..) => "InstanceValue",
            Self::Interaction(..) => "Interaction",
            Self::InteractionConstraint(..) => "InteractionConstraint",
            Self::InteractionFragment(..) => "InteractionFragment",
            Self::InteractionOperand(..) => "InteractionOperand",
            Self::InteractionUse(..) => "InteractionUse",
            Self::Interface(..) => "Interface",
            Self::InterfaceRealization(..) => "InterfaceRealization",
            Self::InterruptibleActivityRegion(..) => "InterruptibleActivityRegion",
            Self::Interval(..) => "Interval",
            Self::IntervalConstraint(..) => "IntervalConstraint",
            Self::InvocationAction(..) => "InvocationAction",
            Self::JoinNode(..) => "JoinNode",
            Self::Lifeline(..) => "Lifeline",
            Self::LinkAction(..) => "LinkAction",
            Self::LinkEndCreationData(..) => "LinkEndCreationData",
            Self::LinkEndData(..) => "LinkEndData",
            Self::LinkEndDestructionData(..) => "LinkEndDestructionData",
            Self::LiteralBoolean(..) => "LiteralBoolean",
            Self::LiteralInteger(..) => "LiteralInteger",
            Self::LiteralNull(..) => "LiteralNull",
            Self::LiteralSpecification(..) => "LiteralSpecification",
            Self::LiteralString(..) => "LiteralString",
            Self::LiteralUnlimitedNatural(..) => "LiteralUnlimitedNatural",
            Self::LoopNode(..) => "LoopNode",
            Self::Manifestation(..) => "Manifestation",
            Self::MergeNode(..) => "MergeNode",
            Self::Message(..) => "Message",
            Self::MessageEnd(..) => "MessageEnd",
            Self::MessageEvent(..) => "MessageEvent",
            Self::MessageOccurrenceSpecification(..) => "MessageOccurrenceSpecification",
            Self::Model(..) => "Model",
            Self::MultiplicityElement(..) => "MultiplicityElement",
            Self::NamedElement(..) => "NamedElement",
            Self::Namespace(..) => "Namespace",
            Self::Node(..) => "Node",
            Self::ObjectFlow(..) => "ObjectFlow",
            Self::ObjectNode(..) => "ObjectNode",
            Self::Observation(..) => "Observation",
            Self::OccurrenceSpecification(..) => "OccurrenceSpecification",
            Self::OpaqueAction(..) => "OpaqueAction",
            Self::OpaqueBehavior(..) => "OpaqueBehavior",
            Self::OpaqueExpression(..) => "OpaqueExpression",
            Self::Operation(..) => "Operation",
            Self::OperationTemplateParameter(..) => "OperationTemplateParameter",
            Self::OutputPin(..) => "OutputPin",
            Self::Package(..) => "Package",
            Self::PackageImport(..) => "PackageImport",
            Self::PackageMerge(..) => "PackageMerge",
            Self::PackageableElement(..) => "PackageableElement",
            Self::Parameter(..) => "Parameter",
            Self::ParameterSet(..) => "ParameterSet",
            Self::ParameterableElement(..) => "ParameterableElement",
            Self::PartDecomposition(..) => "PartDecomposition",
            Self::Pin(..) => "Pin",
            Self::Port(..) => "Port",
            Self::PrimitiveType(..) => "PrimitiveType",
            Self::Profile(..) => "Profile",
            Self::ProfileApplication(..) => "ProfileApplication",
            Self::Property(..) => "Property",
            Self::ProtocolConformance(..) => "ProtocolConformance",
            Self::ProtocolStateMachine(..) => "ProtocolStateMachine",
            Self::ProtocolTransition(..) => "ProtocolTransition",
            Self::Pseudostate(..) => "Pseudostate",
            Self::QualifierValue(..) => "QualifierValue",
            Self::RaiseExceptionAction(..) => "RaiseExceptionAction",
            Self::ReadExtentAction(..) => "ReadExtentAction",
            Self::ReadIsClassifiedObjectAction(..) => "ReadIsClassifiedObjectAction",
            Self::ReadLinkAction(..) => "ReadLinkAction",
            Self::ReadLinkObjectEndAction(..) => "ReadLinkObjectEndAction",
            Self::ReadLinkObjectEndQualifierAction(..) => "ReadLinkObjectEndQualifierAction",
            Self::ReadSelfAction(..) => "ReadSelfAction",
            Self::ReadStructuralFeatureAction(..) => "ReadStructuralFeatureAction",
            Self::ReadVariableAction(..) => "ReadVariableAction",
            Self::Realization(..) => "Realization",
            Self::ReceiveOperationEvent(..) => "ReceiveOperationEvent",
            Self::ReceiveSignalEvent(..) => "ReceiveSignalEvent",
            Self::Reception(..) => "Reception",
            Self::ReclassifyObjectAction(..) => "ReclassifyObjectAction",
            Self::RedefinableElement(..) => "RedefinableElement",
            Self::RedefinableTemplateSignature(..) => "RedefinableTemplateSignature",
            Self::ReduceAction(..) => "ReduceAction",
            Self::Region(..) => "Region",
            Self::Relationship(..) => "Relationship",
            Self::RemoveStructuralFeatureValueAction(..) => "RemoveStructuralFeatureValueAction",
            Self::RemoveVariableValueAction(..) => "RemoveVariableValueAction",
            Self::ReplyAction(..) => "ReplyAction",
            Self::SendObjectAction(..) => "SendObjectAction",
            Self::SendOperationEvent(..) => "SendOperationEvent",
            Self::SendSignalAction(..) => "SendSignalAction",
            Self::SendSignalEvent(..) => "SendSignalEvent",
            Self::SequenceNode(..) => "SequenceNode",
            Self::Signal(..) => "Signal",
            Self::SignalEvent(..) => "SignalEvent",
            Self::Slot(..) => "Slot",
            Self::StartClassifierBehaviorAction(..) => "StartClassifierBehaviorAction",
            Self::State(..) => "State",
            Self::StateInvariant(..) => "StateInvariant",
            Self::StateMachine(..) => "StateMachine",
            Self::Stereotype(..) => "Stereotype",
            Self::StringExpression(..) => "StringExpression",
            Self::StructuralFeature(..) => "StructuralFeature",
            Self::StructuralFeatureAction(..) => "StructuralFeatureAction",
            Self::StructuredActivityNode(..) => "StructuredActivityNode",
            Self::StructuredClassifier(..) => "StructuredClassifier",
            Self::Substitution(..) => "Substitution",
            Self::TemplateBinding(..) => "TemplateBinding",
            Self::TemplateParameter(..) => "TemplateParameter",
            Self::TemplateParameterSubstitution(..) => "TemplateParameterSubstitution",
            Self::TemplateSignature(..) => "TemplateSignature",
            Self::TemplateableElement(..) => "TemplateableElement",
            Self::TestIdentityAction(..) => "TestIdentityAction",
            Self::TimeConstraint(..) => "TimeConstraint",
            Self::TimeEvent(..) => "TimeEvent",
            Self::TimeExpression(..) => "TimeExpression",
            Self::TimeInterval(..) => "TimeInterval",
            Self::TimeObservation(..) => "TimeObservation",
            Self::Transition(..) => "Transition",
            Self::Trigger(..) => "Trigger",
            Self::Type(..) => "Type",
            Self::TypedElement(..) => "TypedElement",
            Self::UnmarshallAction(..) => "UnmarshallAction",
            Self::Usage(..) => "Usage",
            Self::UseCase(..) => "UseCase",
            Self::ValuePin(..) => "ValuePin",
            Self::ValueSpecification(..) => "ValueSpecification",
            Self::ValueSpecificationAction(..) => "ValueSpecificationAction",
            Self::Variable(..) => "Variable",
            Self::VariableAction(..) => "VariableAction",
            Self::Vertex(..) => "Vertex",
            Self::WriteLinkAction(..) => "WriteLinkAction",
            Self::WriteStructuralFeatureAction(..) => "WriteStructuralFeatureAction",
            Self::WriteVariableAction(..) => "WriteVariableAction",
            Self::Abstraction(..) => "Abstraction",
            Self::AcceptCallAction(..) => "AcceptCallAction",
            Self::AcceptEventAction(..) => "AcceptEventAction",
            Self::Action(..) => "Action",
            Self::ActionExecutionSpecification(..) => "ActionExecutionSpecification",
            Self::ActionInputPin(..) => "ActionInputPin",
            Self::Activity(..) => "Activity",
            Self::ActivityEdge(..) => "ActivityEdge",
            Self::ActivityFinalNode(..) => "ActivityFinalNode",
            Self::ActivityGroup(..) => "ActivityGroup",
            Self::ActivityNode(..) => "ActivityNode",
            Self::ActivityParameterNode(..) => "ActivityParameterNode",
            Self::ActivityPartition(..) => "ActivityPartition",
            Self::Actor(..) => "Actor",
            Self::AddStructuralFeatureValueAction(..) => "AddStructuralFeatureValueAction",
            Self::AddVariableValueAction(..) => "AddVariableValueAction",
            Self::AnyReceiveEvent(..) => "AnyReceiveEvent",
            Self::Artifact(..) => "Artifact",
            Self::Association(..) => "Association",
            Self::AssociationClass(..) => "AssociationClass",
            Self::Behavior(..) => "Behavior",
            Self::BehaviorExecutionSpecification(..) => "BehaviorExecutionSpecification",
            Self::BehavioralFeature(..) => "BehavioralFeature",
            Self::BehavioredClassifier(..) => "BehavioredClassifier",
            Self::BroadcastSignalAction(..) => "BroadcastSignalAction",
            Self::CallAction(..) => "CallAction",
            Self::CallBehaviorAction(..) => "CallBehaviorAction",
            Self::CallEvent(..) => "CallEvent",
            Self::CallOperationAction(..) => "CallOperationAction",
        }
    }

    pub fn id(&self) -> &str {
        match self {
            Self::CentralBufferNode(instance) => &instance.id,
            Self::ChangeEvent(instance) => &instance.id,
            Self::Class(instance) => &instance.id,
            Self::Classifier(instance) => &instance.id,
            Self::ClassifierTemplateParameter(instance) => &instance.id,
            Self::Clause(instance) => &instance.id,
            Self::ClearAssociationAction(instance) => &instance.id,
            Self::ClearStructuralFeatureAction(instance) => &instance.id,
            Self::ClearVariableAction(instance) => &instance.id,
            Self::Collaboration(instance) => &instance.id,
            Self::CollaborationUse(instance) => &instance.id,
            Self::CombinedFragment(instance) => &instance.id,
            Self::Comment(instance) => &instance.id,
            Self::CommunicationPath(instance) => &instance.id,
            Self::Component(instance) => &instance.id,
            Self::ComponentRealization(instance) => &instance.id,
            Self::ConditionalNode(instance) => &instance.id,
            Self::ConnectableElement(instance) => &instance.id,
            Self::ConnectableElementTemplateParameter(instance) => &instance.id,
            Self::ConnectionPointReference(instance) => &instance.id,
            Self::Connector(instance) => &instance.id,
            Self::ConnectorEnd(instance) => &instance.id,
            Self::ConsiderIgnoreFragment(instance) => &instance.id,
            Self::Constraint(instance) => &instance.id,
            Self::Continuation(instance) => &instance.id,
            Self::ControlFlow(instance) => &instance.id,
            Self::ControlNode(instance) => &instance.id,
            Self::CreateLinkAction(instance) => &instance.id,
            Self::CreateLinkObjectAction(instance) => &instance.id,
            Self::CreateObjectAction(instance) => &instance.id,
            Self::CreationEvent(instance) => &instance.id,
            Self::DataStoreNode(instance) => &instance.id,
            Self::DataType(instance) => &instance.id,
            Self::DecisionNode(instance) => &instance.id,
            Self::Dependency(instance) => &instance.id,
            Self::DeployedArtifact(instance) => &instance.id,
            Self::Deployment(instance) => &instance.id,
            Self::DeploymentSpecification(instance) => &instance.id,
            Self::DeploymentTarget(instance) => &instance.id,
            Self::DestroyLinkAction(instance) => &instance.id,
            Self::DestroyObjectAction(instance) => &instance.id,
            Self::DestructionEvent(instance) => &instance.id,
            Self::Device(instance) => &instance.id,
            Self::DirectedRelationship(instance) => &instance.id,
            Self::Duration(instance) => &instance.id,
            Self::DurationConstraint(instance) => &instance.id,
            Self::DurationInterval(instance) => &instance.id,
            Self::DurationObservation(instance) => &instance.id,
            Self::EAnnotation(instance) => &instance.id,
            Self::EAttribute(instance) => &instance.id,
            Self::EClass(instance) => &instance.id,
            Self::EClassifier(instance) => &instance.id,
            Self::EDataType(instance) => &instance.id,
            Self::EEnum(instance) => &instance.id,
            Self::EEnumLiteral(instance) => &instance.id,
            Self::EGenericType(instance) => &instance.id,
            Self::Element(instance) => &instance.id,
            Self::ElementImport(instance) => &instance.id,
            Self::EModelElement(instance) => &instance.id,
            Self::ENamedElement(instance) => &instance.id,
            Self::EncapsulatedClassifier(instance) => &instance.id,
            Self::Enumeration(instance) => &instance.id,
            Self::EnumerationLiteral(instance) => &instance.id,
            Self::EObject(instance) => &instance.id,
            Self::EOperation(instance) => &instance.id,
            Self::EPackage(instance) => &instance.id,
            Self::EParameter(instance) => &instance.id,
            Self::EReference(instance) => &instance.id,
            Self::EStructuralFeature(instance) => &instance.id,
            Self::ETypeParameter(instance) => &instance.id,
            Self::ETypedElement(instance) => &instance.id,
            Self::Event(instance) => &instance.id,
            Self::ExceptionHandler(instance) => &instance.id,
            Self::ExecutableNode(instance) => &instance.id,
            Self::ExecutionEnvironment(instance) => &instance.id,
            Self::ExecutionEvent(instance) => &instance.id,
            Self::ExecutionOccurrenceSpecification(instance) => &instance.id,
            Self::ExecutionSpecification(instance) => &instance.id,
            Self::ExpansionNode(instance) => &instance.id,
            Self::ExpansionRegion(instance) => &instance.id,
            Self::Expression(instance) => &instance.id,
            Self::Extend(instance) => &instance.id,
            Self::Extension(instance) => &instance.id,
            Self::ExtensionEnd(instance) => &instance.id,
            Self::ExtensionPoint(instance) => &instance.id,
            Self::Feature(instance) => &instance.id,
            Self::FinalNode(instance) => &instance.id,
            Self::FinalState(instance) => &instance.id,
            Self::FlowFinalNode(instance) => &instance.id,
            Self::ForkNode(instance) => &instance.id,
            Self::FunctionBehavior(instance) => &instance.id,
            Self::Gate(instance) => &instance.id,
            Self::GeneralOrdering(instance) => &instance.id,
            Self::Generalization(instance) => &instance.id,
            Self::GeneralizationSet(instance) => &instance.id,
            Self::Image(instance) => &instance.id,
            Self::Include(instance) => &instance.id,
            Self::InformationFlow(instance) => &instance.id,
            Self::InformationItem(instance) => &instance.id,
            Self::InitialNode(instance) => &instance.id,
            Self::InputPin(instance) => &instance.id,
            Self::InstanceSpecification(instance) => &instance.id,
            Self::InstanceValue(instance) => &instance.id,
            Self::Interaction(instance) => &instance.id,
            Self::InteractionConstraint(instance) => &instance.id,
            Self::InteractionFragment(instance) => &instance.id,
            Self::InteractionOperand(instance) => &instance.id,
            Self::InteractionUse(instance) => &instance.id,
            Self::Interface(instance) => &instance.id,
            Self::InterfaceRealization(instance) => &instance.id,
            Self::InterruptibleActivityRegion(instance) => &instance.id,
            Self::Interval(instance) => &instance.id,
            Self::IntervalConstraint(instance) => &instance.id,
            Self::InvocationAction(instance) => &instance.id,
            Self::JoinNode(instance) => &instance.id,
            Self::Lifeline(instance) => &instance.id,
            Self::LinkAction(instance) => &instance.id,
            Self::LinkEndCreationData(instance) => &instance.id,
            Self::LinkEndData(instance) => &instance.id,
            Self::LinkEndDestructionData(instance) => &instance.id,
            Self::LiteralBoolean(instance) => &instance.id,
            Self::LiteralInteger(instance) => &instance.id,
            Self::LiteralNull(instance) => &instance.id,
            Self::LiteralSpecification(instance) => &instance.id,
            Self::LiteralString(instance) => &instance.id,
            Self::LiteralUnlimitedNatural(instance) => &instance.id,
            Self::LoopNode(instance) => &instance.id,
            Self::Manifestation(instance) => &instance.id,
            Self::MergeNode(instance) => &instance.id,
            Self::Message(instance) => &instance.id,
            Self::MessageEnd(instance) => &instance.id,
            Self::MessageEvent(instance) => &instance.id,
            Self::MessageOccurrenceSpecification(instance) => &instance.id,
            Self::Model(instance) => &instance.id,
            Self::MultiplicityElement(instance) => &instance.id,
            Self::NamedElement(instance) => &instance.id,
            Self::Namespace(instance) => &instance.id,
            Self::Node(instance) => &instance.id,
            Self::ObjectFlow(instance) => &instance.id,
            Self::ObjectNode(instance) => &instance.id,
            Self::Observation(instance) => &instance.id,
            Self::OccurrenceSpecification(instance) => &instance.id,
            Self::OpaqueAction(instance) => &instance.id,
            Self::OpaqueBehavior(instance) => &instance.id,
            Self::OpaqueExpression(instance) => &instance.id,
            Self::Operation(instance) => &instance.id,
            Self::OperationTemplateParameter(instance) => &instance.id,
            Self::OutputPin(instance) => &instance.id,
            Self::Package(instance) => &instance.id,
            Self::PackageImport(instance) => &instance.id,
            Self::PackageMerge(instance) => &instance.id,
            Self::PackageableElement(instance) => &instance.id,
            Self::Parameter(instance) => &instance.id,
            Self::ParameterSet(instance) => &instance.id,
            Self::ParameterableElement(instance) => &instance.id,
            Self::PartDecomposition(instance) => &instance.id,
            Self::Pin(instance) => &instance.id,
            Self::Port(instance) => &instance.id,
            Self::PrimitiveType(instance) => &instance.id,
            Self::Profile(instance) => &instance.id,
            Self::ProfileApplication(instance) => &instance.id,
            Self::Property(instance) => &instance.id,
            Self::ProtocolConformance(instance) => &instance.id,
            Self::ProtocolStateMachine(instance) => &instance.id,
            Self::ProtocolTransition(instance) => &instance.id,
            Self::Pseudostate(instance) => &instance.id,
            Self::QualifierValue(instance) => &instance.id,
            Self::RaiseExceptionAction(instance) => &instance.id,
            Self::ReadExtentAction(instance) => &instance.id,
            Self::ReadIsClassifiedObjectAction(instance) => &instance.id,
            Self::ReadLinkAction(instance) => &instance.id,
            Self::ReadLinkObjectEndAction(instance) => &instance.id,
            Self::ReadLinkObjectEndQualifierAction(instance) => &instance.id,
            Self::ReadSelfAction(instance) => &instance.id,
            Self::ReadStructuralFeatureAction(instance) => &instance.id,
            Self::ReadVariableAction(instance) => &instance.id,
            Self::Realization(instance) => &instance.id,
            Self::ReceiveOperationEvent(instance) => &instance.id,
            Self::ReceiveSignalEvent(instance) => &instance.id,
            Self::Reception(instance) => &instance.id,
            Self::ReclassifyObjectAction(instance) => &instance.id,
            Self::RedefinableElement(instance) => &instance.id,
            Self::RedefinableTemplateSignature(instance) => &instance.id,
            Self::ReduceAction(instance) => &instance.id,
            Self::Region(instance) => &instance.id,
            Self::Relationship(instance) => &instance.id,
            Self::RemoveStructuralFeatureValueAction(instance) => &instance.id,
            Self::RemoveVariableValueAction(instance) => &instance.id,
            Self::ReplyAction(instance) => &instance.id,
            Self::SendObjectAction(instance) => &instance.id,
            Self::SendOperationEvent(instance) => &instance.id,
            Self::SendSignalAction(instance) => &instance.id,
            Self::SendSignalEvent(instance) => &instance.id,
            Self::SequenceNode(instance) => &instance.id,
            Self::Signal(instance) => &instance.id,
            Self::SignalEvent(instance) => &instance.id,
            Self::Slot(instance) => &instance.id,
            Self::StartClassifierBehaviorAction(instance) => &instance.id,
            Self::State(instance) => &instance.id,
            Self::StateInvariant(instance) => &instance.id,
            Self::StateMachine(instance) => &instance.id,
            Self::Stereotype(instance) => &instance.id,
            Self::StringExpression(instance) => &instance.id,
            Self::StructuralFeature(instance) => &instance.id,
            Self::StructuralFeatureAction(instance) => &instance.id,
            Self::StructuredActivityNode(instance) => &instance.id,
            Self::StructuredClassifier(instance) => &instance.id,
            Self::Substitution(instance) => &instance.id,
            Self::TemplateBinding(instance) => &instance.id,
            Self::TemplateParameter(instance) => &instance.id,
            Self::TemplateParameterSubstitution(instance) => &instance.id,
            Self::TemplateSignature(instance) => &instance.id,
            Self::TemplateableElement(instance) => &instance.id,
            Self::TestIdentityAction(instance) => &instance.id,
            Self::TimeConstraint(instance) => &instance.id,
            Self::TimeEvent(instance) => &instance.id,
            Self::TimeExpression(instance) => &instance.id,
            Self::TimeInterval(instance) => &instance.id,
            Self::TimeObservation(instance) => &instance.id,
            Self::Transition(instance) => &instance.id,
            Self::Trigger(instance) => &instance.id,
            Self::Type(instance) => &instance.id,
            Self::TypedElement(instance) => &instance.id,
            Self::UnmarshallAction(instance) => &instance.id,
            Self::Usage(instance) => &instance.id,
            Self::UseCase(instance) => &instance.id,
            Self::ValuePin(instance) => &instance.id,
            Self::ValueSpecification(instance) => &instance.id,
            Self::ValueSpecificationAction(instance) => &instance.id,
            Self::Variable(instance) => &instance.id,
            Self::VariableAction(instance) => &instance.id,
            Self::Vertex(instance) => &instance.id,
            Self::WriteLinkAction(instance) => &instance.id,
            Self::WriteStructuralFeatureAction(instance) => &instance.id,
            Self::WriteVariableAction(instance) => &instance.id,
            Self::Abstraction(instance) => &instance.id,
            Self::AcceptCallAction(instance) => &instance.id,
            Self::AcceptEventAction(instance) => &instance.id,
            Self::Action(instance) => &instance.id,
            Self::ActionExecutionSpecification(instance) => &instance.id,
            Self::ActionInputPin(instance) => &instance.id,
            Self::Activity(instance) => &instance.id,
            Self::ActivityEdge(instance) => &instance.id,
            Self::ActivityFinalNode(instance) => &instance.id,
            Self::ActivityGroup(instance) => &instance.id,
            Self::ActivityNode(instance) => &instance.id,
            Self::ActivityParameterNode(instance) => &instance.id,
            Self::ActivityPartition(instance) => &instance.id,
            Self::Actor(instance) => &instance.id,
            Self::AddStructuralFeatureValueAction(instance) => &instance.id,
            Self::AddVariableValueAction(instance) => &instance.id,
            Self::AnyReceiveEvent(instance) => &instance.id,
            Self::Artifact(instance) => &instance.id,
            Self::Association(instance) => &instance.id,
            Self::AssociationClass(instance) => &instance.id,
            Self::Behavior(instance) => &instance.id,
            Self::BehaviorExecutionSpecification(instance) => &instance.id,
            Self::BehavioralFeature(instance) => &instance.id,
            Self::BehavioredClassifier(instance) => &instance.id,
            Self::BroadcastSignalAction(instance) => &instance.id,
            Self::CallAction(instance) => &instance.id,
            Self::CallBehaviorAction(instance) => &instance.id,
            Self::CallEvent(instance) => &instance.id,
            Self::CallOperationAction(instance) => &instance.id,
        }
    }

    pub fn from_data(type_name: &str, data: serde_json::Value) -> Result<Self, String> {
        serde_json::from_value(serde_json::json!({"type": type_name, "data": data}))
            .map_err(|error| error.to_string())
    }
}

impl ContractType for crate::ruml::central_buffer_node::CentralBufferNode {
    const TYPE_NAME: &'static str = "CentralBufferNode";
}

impl ContractType for crate::ruml::change_event::ChangeEvent {
    const TYPE_NAME: &'static str = "ChangeEvent";
}

impl ContractType for crate::ruml::class::Class {
    const TYPE_NAME: &'static str = "Class";
}

impl ContractType for crate::ruml::classifier::Classifier {
    const TYPE_NAME: &'static str = "Classifier";
}

impl ContractType for crate::ruml::classifier_template_parameter::ClassifierTemplateParameter {
    const TYPE_NAME: &'static str = "ClassifierTemplateParameter";
}

impl ContractType for crate::ruml::clause::Clause {
    const TYPE_NAME: &'static str = "Clause";
}

impl ContractType for crate::ruml::clear_association_action::ClearAssociationAction {
    const TYPE_NAME: &'static str = "ClearAssociationAction";
}

impl ContractType for crate::ruml::clear_structural_feature_action::ClearStructuralFeatureAction {
    const TYPE_NAME: &'static str = "ClearStructuralFeatureAction";
}

impl ContractType for crate::ruml::clear_variable_action::ClearVariableAction {
    const TYPE_NAME: &'static str = "ClearVariableAction";
}

impl ContractType for crate::ruml::collaboration::Collaboration {
    const TYPE_NAME: &'static str = "Collaboration";
}

impl ContractType for crate::ruml::collaboration_use::CollaborationUse {
    const TYPE_NAME: &'static str = "CollaborationUse";
}

impl ContractType for crate::ruml::combined_fragment::CombinedFragment {
    const TYPE_NAME: &'static str = "CombinedFragment";
}

impl ContractType for crate::ruml::comment::Comment {
    const TYPE_NAME: &'static str = "Comment";
}

impl ContractType for crate::ruml::communication_path::CommunicationPath {
    const TYPE_NAME: &'static str = "CommunicationPath";
}

impl ContractType for crate::ruml::component::Component {
    const TYPE_NAME: &'static str = "Component";
}

impl ContractType for crate::ruml::component_realization::ComponentRealization {
    const TYPE_NAME: &'static str = "ComponentRealization";
}

impl ContractType for crate::ruml::conditional_node::ConditionalNode {
    const TYPE_NAME: &'static str = "ConditionalNode";
}

impl ContractType for crate::ruml::connectable_element::ConnectableElement {
    const TYPE_NAME: &'static str = "ConnectableElement";
}

impl ContractType
    for crate::ruml::connectable_element_template_parameter::ConnectableElementTemplateParameter
{
    const TYPE_NAME: &'static str = "ConnectableElementTemplateParameter";
}

impl ContractType for crate::ruml::connection_point_reference::ConnectionPointReference {
    const TYPE_NAME: &'static str = "ConnectionPointReference";
}

impl ContractType for crate::ruml::connector::Connector {
    const TYPE_NAME: &'static str = "Connector";
}

impl ContractType for crate::ruml::connector_end::ConnectorEnd {
    const TYPE_NAME: &'static str = "ConnectorEnd";
}

impl ContractType for crate::ruml::consider_ignore_fragment::ConsiderIgnoreFragment {
    const TYPE_NAME: &'static str = "ConsiderIgnoreFragment";
}

impl ContractType for crate::ruml::constraint::Constraint {
    const TYPE_NAME: &'static str = "Constraint";
}

impl ContractType for crate::ruml::continuation::Continuation {
    const TYPE_NAME: &'static str = "Continuation";
}

impl ContractType for crate::ruml::control_flow::ControlFlow {
    const TYPE_NAME: &'static str = "ControlFlow";
}

impl ContractType for crate::ruml::control_node::ControlNode {
    const TYPE_NAME: &'static str = "ControlNode";
}

impl ContractType for crate::ruml::create_link_action::CreateLinkAction {
    const TYPE_NAME: &'static str = "CreateLinkAction";
}

impl ContractType for crate::ruml::create_link_object_action::CreateLinkObjectAction {
    const TYPE_NAME: &'static str = "CreateLinkObjectAction";
}

impl ContractType for crate::ruml::create_object_action::CreateObjectAction {
    const TYPE_NAME: &'static str = "CreateObjectAction";
}

impl ContractType for crate::ruml::creation_event::CreationEvent {
    const TYPE_NAME: &'static str = "CreationEvent";
}

impl ContractType for crate::ruml::data_store_node::DataStoreNode {
    const TYPE_NAME: &'static str = "DataStoreNode";
}

impl ContractType for crate::ruml::data_type::DataType {
    const TYPE_NAME: &'static str = "DataType";
}

impl ContractType for crate::ruml::decision_node::DecisionNode {
    const TYPE_NAME: &'static str = "DecisionNode";
}

impl ContractType for crate::ruml::dependency::Dependency {
    const TYPE_NAME: &'static str = "Dependency";
}

impl ContractType for crate::ruml::deployed_artifact::DeployedArtifact {
    const TYPE_NAME: &'static str = "DeployedArtifact";
}

impl ContractType for crate::ruml::deployment::Deployment {
    const TYPE_NAME: &'static str = "Deployment";
}

impl ContractType for crate::ruml::deployment_specification::DeploymentSpecification {
    const TYPE_NAME: &'static str = "DeploymentSpecification";
}

impl ContractType for crate::ruml::deployment_target::DeploymentTarget {
    const TYPE_NAME: &'static str = "DeploymentTarget";
}

impl ContractType for crate::ruml::destroy_link_action::DestroyLinkAction {
    const TYPE_NAME: &'static str = "DestroyLinkAction";
}

impl ContractType for crate::ruml::destroy_object_action::DestroyObjectAction {
    const TYPE_NAME: &'static str = "DestroyObjectAction";
}

impl ContractType for crate::ruml::destruction_event::DestructionEvent {
    const TYPE_NAME: &'static str = "DestructionEvent";
}

impl ContractType for crate::ruml::device::Device {
    const TYPE_NAME: &'static str = "Device";
}

impl ContractType for crate::ruml::directed_relationship::DirectedRelationship {
    const TYPE_NAME: &'static str = "DirectedRelationship";
}

impl ContractType for crate::ruml::duration::Duration {
    const TYPE_NAME: &'static str = "Duration";
}

impl ContractType for crate::ruml::duration_constraint::DurationConstraint {
    const TYPE_NAME: &'static str = "DurationConstraint";
}

impl ContractType for crate::ruml::duration_interval::DurationInterval {
    const TYPE_NAME: &'static str = "DurationInterval";
}

impl ContractType for crate::ruml::duration_observation::DurationObservation {
    const TYPE_NAME: &'static str = "DurationObservation";
}

impl ContractType for crate::ruml::eannotation::EAnnotation {
    const TYPE_NAME: &'static str = "EAnnotation";
}

impl ContractType for crate::ruml::eattribute::EAttribute {
    const TYPE_NAME: &'static str = "EAttribute";
}

impl ContractType for crate::ruml::eclass::EClass {
    const TYPE_NAME: &'static str = "EClass";
}

impl ContractType for crate::ruml::eclassifier::EClassifier {
    const TYPE_NAME: &'static str = "EClassifier";
}

impl ContractType for crate::ruml::edata_type::EDataType {
    const TYPE_NAME: &'static str = "EDataType";
}

impl ContractType for crate::ruml::eenum::EEnum {
    const TYPE_NAME: &'static str = "EEnum";
}

impl ContractType for crate::ruml::eenum_literal::EEnumLiteral {
    const TYPE_NAME: &'static str = "EEnumLiteral";
}

impl ContractType for crate::ruml::egeneric_type::EGenericType {
    const TYPE_NAME: &'static str = "EGenericType";
}

impl ContractType for crate::ruml::element::Element {
    const TYPE_NAME: &'static str = "Element";
}

impl ContractType for crate::ruml::element_import::ElementImport {
    const TYPE_NAME: &'static str = "ElementImport";
}

impl ContractType for crate::ruml::emodel_element::EModelElement {
    const TYPE_NAME: &'static str = "EModelElement";
}

impl ContractType for crate::ruml::enamed_element::ENamedElement {
    const TYPE_NAME: &'static str = "ENamedElement";
}

impl ContractType for crate::ruml::encapsulated_classifier::EncapsulatedClassifier {
    const TYPE_NAME: &'static str = "EncapsulatedClassifier";
}

impl ContractType for crate::ruml::enumeration::Enumeration {
    const TYPE_NAME: &'static str = "Enumeration";
}

impl ContractType for crate::ruml::enumeration_literal::EnumerationLiteral {
    const TYPE_NAME: &'static str = "EnumerationLiteral";
}

impl ContractType for crate::ruml::eobject::EObject {
    const TYPE_NAME: &'static str = "EObject";
}

impl ContractType for crate::ruml::eoperation::EOperation {
    const TYPE_NAME: &'static str = "EOperation";
}

impl ContractType for crate::ruml::epackage::EPackage {
    const TYPE_NAME: &'static str = "EPackage";
}

impl ContractType for crate::ruml::eparameter::EParameter {
    const TYPE_NAME: &'static str = "EParameter";
}

impl ContractType for crate::ruml::ereference::EReference {
    const TYPE_NAME: &'static str = "EReference";
}

impl ContractType for crate::ruml::estructural_feature::EStructuralFeature {
    const TYPE_NAME: &'static str = "EStructuralFeature";
}

impl ContractType for crate::ruml::etype_parameter::ETypeParameter {
    const TYPE_NAME: &'static str = "ETypeParameter";
}

impl ContractType for crate::ruml::etyped_element::ETypedElement {
    const TYPE_NAME: &'static str = "ETypedElement";
}

impl ContractType for crate::ruml::event::Event {
    const TYPE_NAME: &'static str = "Event";
}

impl ContractType for crate::ruml::exception_handler::ExceptionHandler {
    const TYPE_NAME: &'static str = "ExceptionHandler";
}

impl ContractType for crate::ruml::executable_node::ExecutableNode {
    const TYPE_NAME: &'static str = "ExecutableNode";
}

impl ContractType for crate::ruml::execution_environment::ExecutionEnvironment {
    const TYPE_NAME: &'static str = "ExecutionEnvironment";
}

impl ContractType for crate::ruml::execution_event::ExecutionEvent {
    const TYPE_NAME: &'static str = "ExecutionEvent";
}

impl ContractType
    for crate::ruml::execution_occurrence_specification::ExecutionOccurrenceSpecification
{
    const TYPE_NAME: &'static str = "ExecutionOccurrenceSpecification";
}

impl ContractType for crate::ruml::execution_specification::ExecutionSpecification {
    const TYPE_NAME: &'static str = "ExecutionSpecification";
}

impl ContractType for crate::ruml::expansion_node::ExpansionNode {
    const TYPE_NAME: &'static str = "ExpansionNode";
}

impl ContractType for crate::ruml::expansion_region::ExpansionRegion {
    const TYPE_NAME: &'static str = "ExpansionRegion";
}

impl ContractType for crate::ruml::expression::Expression {
    const TYPE_NAME: &'static str = "Expression";
}

impl ContractType for crate::ruml::extend::Extend {
    const TYPE_NAME: &'static str = "Extend";
}

impl ContractType for crate::ruml::extension::Extension {
    const TYPE_NAME: &'static str = "Extension";
}

impl ContractType for crate::ruml::extension_end::ExtensionEnd {
    const TYPE_NAME: &'static str = "ExtensionEnd";
}

impl ContractType for crate::ruml::extension_point::ExtensionPoint {
    const TYPE_NAME: &'static str = "ExtensionPoint";
}

impl ContractType for crate::ruml::feature::Feature {
    const TYPE_NAME: &'static str = "Feature";
}

impl ContractType for crate::ruml::final_node::FinalNode {
    const TYPE_NAME: &'static str = "FinalNode";
}

impl ContractType for crate::ruml::final_state::FinalState {
    const TYPE_NAME: &'static str = "FinalState";
}

impl ContractType for crate::ruml::flow_final_node::FlowFinalNode {
    const TYPE_NAME: &'static str = "FlowFinalNode";
}

impl ContractType for crate::ruml::fork_node::ForkNode {
    const TYPE_NAME: &'static str = "ForkNode";
}

impl ContractType for crate::ruml::function_behavior::FunctionBehavior {
    const TYPE_NAME: &'static str = "FunctionBehavior";
}

impl ContractType for crate::ruml::gate::Gate {
    const TYPE_NAME: &'static str = "Gate";
}

impl ContractType for crate::ruml::general_ordering::GeneralOrdering {
    const TYPE_NAME: &'static str = "GeneralOrdering";
}

impl ContractType for crate::ruml::generalization::Generalization {
    const TYPE_NAME: &'static str = "Generalization";
}

impl ContractType for crate::ruml::generalization_set::GeneralizationSet {
    const TYPE_NAME: &'static str = "GeneralizationSet";
}

impl ContractType for crate::ruml::image::Image {
    const TYPE_NAME: &'static str = "Image";
}

impl ContractType for crate::ruml::include::Include {
    const TYPE_NAME: &'static str = "Include";
}

impl ContractType for crate::ruml::information_flow::InformationFlow {
    const TYPE_NAME: &'static str = "InformationFlow";
}

impl ContractType for crate::ruml::information_item::InformationItem {
    const TYPE_NAME: &'static str = "InformationItem";
}

impl ContractType for crate::ruml::initial_node::InitialNode {
    const TYPE_NAME: &'static str = "InitialNode";
}

impl ContractType for crate::ruml::input_pin::InputPin {
    const TYPE_NAME: &'static str = "InputPin";
}

impl ContractType for crate::ruml::instance_specification::InstanceSpecification {
    const TYPE_NAME: &'static str = "InstanceSpecification";
}

impl ContractType for crate::ruml::instance_value::InstanceValue {
    const TYPE_NAME: &'static str = "InstanceValue";
}

impl ContractType for crate::ruml::interaction::Interaction {
    const TYPE_NAME: &'static str = "Interaction";
}

impl ContractType for crate::ruml::interaction_constraint::InteractionConstraint {
    const TYPE_NAME: &'static str = "InteractionConstraint";
}

impl ContractType for crate::ruml::interaction_fragment::InteractionFragment {
    const TYPE_NAME: &'static str = "InteractionFragment";
}

impl ContractType for crate::ruml::interaction_operand::InteractionOperand {
    const TYPE_NAME: &'static str = "InteractionOperand";
}

impl ContractType for crate::ruml::interaction_use::InteractionUse {
    const TYPE_NAME: &'static str = "InteractionUse";
}

impl ContractType for crate::ruml::interface::Interface {
    const TYPE_NAME: &'static str = "Interface";
}

impl ContractType for crate::ruml::interface_realization::InterfaceRealization {
    const TYPE_NAME: &'static str = "InterfaceRealization";
}

impl ContractType for crate::ruml::interruptible_activity_region::InterruptibleActivityRegion {
    const TYPE_NAME: &'static str = "InterruptibleActivityRegion";
}

impl ContractType for crate::ruml::interval::Interval {
    const TYPE_NAME: &'static str = "Interval";
}

impl ContractType for crate::ruml::interval_constraint::IntervalConstraint {
    const TYPE_NAME: &'static str = "IntervalConstraint";
}

impl ContractType for crate::ruml::invocation_action::InvocationAction {
    const TYPE_NAME: &'static str = "InvocationAction";
}

impl ContractType for crate::ruml::join_node::JoinNode {
    const TYPE_NAME: &'static str = "JoinNode";
}

impl ContractType for crate::ruml::lifeline::Lifeline {
    const TYPE_NAME: &'static str = "Lifeline";
}

impl ContractType for crate::ruml::link_action::LinkAction {
    const TYPE_NAME: &'static str = "LinkAction";
}

impl ContractType for crate::ruml::link_end_creation_data::LinkEndCreationData {
    const TYPE_NAME: &'static str = "LinkEndCreationData";
}

impl ContractType for crate::ruml::link_end_data::LinkEndData {
    const TYPE_NAME: &'static str = "LinkEndData";
}

impl ContractType for crate::ruml::link_end_destruction_data::LinkEndDestructionData {
    const TYPE_NAME: &'static str = "LinkEndDestructionData";
}

impl ContractType for crate::ruml::literal_boolean::LiteralBoolean {
    const TYPE_NAME: &'static str = "LiteralBoolean";
}

impl ContractType for crate::ruml::literal_integer::LiteralInteger {
    const TYPE_NAME: &'static str = "LiteralInteger";
}

impl ContractType for crate::ruml::literal_null::LiteralNull {
    const TYPE_NAME: &'static str = "LiteralNull";
}

impl ContractType for crate::ruml::literal_specification::LiteralSpecification {
    const TYPE_NAME: &'static str = "LiteralSpecification";
}

impl ContractType for crate::ruml::literal_string::LiteralString {
    const TYPE_NAME: &'static str = "LiteralString";
}

impl ContractType for crate::ruml::literal_unlimited_natural::LiteralUnlimitedNatural {
    const TYPE_NAME: &'static str = "LiteralUnlimitedNatural";
}

impl ContractType for crate::ruml::loop_node::LoopNode {
    const TYPE_NAME: &'static str = "LoopNode";
}

impl ContractType for crate::ruml::manifestation::Manifestation {
    const TYPE_NAME: &'static str = "Manifestation";
}

impl ContractType for crate::ruml::merge_node::MergeNode {
    const TYPE_NAME: &'static str = "MergeNode";
}

impl ContractType for crate::ruml::message::Message {
    const TYPE_NAME: &'static str = "Message";
}

impl ContractType for crate::ruml::message_end::MessageEnd {
    const TYPE_NAME: &'static str = "MessageEnd";
}

impl ContractType for crate::ruml::message_event::MessageEvent {
    const TYPE_NAME: &'static str = "MessageEvent";
}

impl ContractType
    for crate::ruml::message_occurrence_specification::MessageOccurrenceSpecification
{
    const TYPE_NAME: &'static str = "MessageOccurrenceSpecification";
}

impl ContractType for crate::ruml::model::Model {
    const TYPE_NAME: &'static str = "Model";
}

impl ContractType for crate::ruml::multiplicity_element::MultiplicityElement {
    const TYPE_NAME: &'static str = "MultiplicityElement";
}

impl ContractType for crate::ruml::named_element::NamedElement {
    const TYPE_NAME: &'static str = "NamedElement";
}

impl ContractType for crate::ruml::namespace::Namespace {
    const TYPE_NAME: &'static str = "Namespace";
}

impl ContractType for crate::ruml::node::Node {
    const TYPE_NAME: &'static str = "Node";
}

impl ContractType for crate::ruml::object_flow::ObjectFlow {
    const TYPE_NAME: &'static str = "ObjectFlow";
}

impl ContractType for crate::ruml::object_node::ObjectNode {
    const TYPE_NAME: &'static str = "ObjectNode";
}

impl ContractType for crate::ruml::observation::Observation {
    const TYPE_NAME: &'static str = "Observation";
}

impl ContractType for crate::ruml::occurrence_specification::OccurrenceSpecification {
    const TYPE_NAME: &'static str = "OccurrenceSpecification";
}

impl ContractType for crate::ruml::opaque_action::OpaqueAction {
    const TYPE_NAME: &'static str = "OpaqueAction";
}

impl ContractType for crate::ruml::opaque_behavior::OpaqueBehavior {
    const TYPE_NAME: &'static str = "OpaqueBehavior";
}

impl ContractType for crate::ruml::opaque_expression::OpaqueExpression {
    const TYPE_NAME: &'static str = "OpaqueExpression";
}

impl ContractType for crate::ruml::operation::Operation {
    const TYPE_NAME: &'static str = "Operation";
}

impl ContractType for crate::ruml::operation_template_parameter::OperationTemplateParameter {
    const TYPE_NAME: &'static str = "OperationTemplateParameter";
}

impl ContractType for crate::ruml::output_pin::OutputPin {
    const TYPE_NAME: &'static str = "OutputPin";
}

impl ContractType for crate::ruml::package::Package {
    const TYPE_NAME: &'static str = "Package";
}

impl ContractType for crate::ruml::package_import::PackageImport {
    const TYPE_NAME: &'static str = "PackageImport";
}

impl ContractType for crate::ruml::package_merge::PackageMerge {
    const TYPE_NAME: &'static str = "PackageMerge";
}

impl ContractType for crate::ruml::packageable_element::PackageableElement {
    const TYPE_NAME: &'static str = "PackageableElement";
}

impl ContractType for crate::ruml::parameter::Parameter {
    const TYPE_NAME: &'static str = "Parameter";
}

impl ContractType for crate::ruml::parameter_set::ParameterSet {
    const TYPE_NAME: &'static str = "ParameterSet";
}

impl ContractType for crate::ruml::parameterable_element::ParameterableElement {
    const TYPE_NAME: &'static str = "ParameterableElement";
}

impl ContractType for crate::ruml::part_decomposition::PartDecomposition {
    const TYPE_NAME: &'static str = "PartDecomposition";
}

impl ContractType for crate::ruml::pin::Pin {
    const TYPE_NAME: &'static str = "Pin";
}

impl ContractType for crate::ruml::port::Port {
    const TYPE_NAME: &'static str = "Port";
}

impl ContractType for crate::ruml::primitive_type::PrimitiveType {
    const TYPE_NAME: &'static str = "PrimitiveType";
}

impl ContractType for crate::ruml::profile::Profile {
    const TYPE_NAME: &'static str = "Profile";
}

impl ContractType for crate::ruml::profile_application::ProfileApplication {
    const TYPE_NAME: &'static str = "ProfileApplication";
}

impl ContractType for crate::ruml::property::Property {
    const TYPE_NAME: &'static str = "Property";
}

impl ContractType for crate::ruml::protocol_conformance::ProtocolConformance {
    const TYPE_NAME: &'static str = "ProtocolConformance";
}

impl ContractType for crate::ruml::protocol_state_machine::ProtocolStateMachine {
    const TYPE_NAME: &'static str = "ProtocolStateMachine";
}

impl ContractType for crate::ruml::protocol_transition::ProtocolTransition {
    const TYPE_NAME: &'static str = "ProtocolTransition";
}

impl ContractType for crate::ruml::pseudostate::Pseudostate {
    const TYPE_NAME: &'static str = "Pseudostate";
}

impl ContractType for crate::ruml::qualifier_value::QualifierValue {
    const TYPE_NAME: &'static str = "QualifierValue";
}

impl ContractType for crate::ruml::raise_exception_action::RaiseExceptionAction {
    const TYPE_NAME: &'static str = "RaiseExceptionAction";
}

impl ContractType for crate::ruml::read_extent_action::ReadExtentAction {
    const TYPE_NAME: &'static str = "ReadExtentAction";
}

impl ContractType for crate::ruml::read_is_classified_object_action::ReadIsClassifiedObjectAction {
    const TYPE_NAME: &'static str = "ReadIsClassifiedObjectAction";
}

impl ContractType for crate::ruml::read_link_action::ReadLinkAction {
    const TYPE_NAME: &'static str = "ReadLinkAction";
}

impl ContractType for crate::ruml::read_link_object_end_action::ReadLinkObjectEndAction {
    const TYPE_NAME: &'static str = "ReadLinkObjectEndAction";
}

impl ContractType
    for crate::ruml::read_link_object_end_qualifier_action::ReadLinkObjectEndQualifierAction
{
    const TYPE_NAME: &'static str = "ReadLinkObjectEndQualifierAction";
}

impl ContractType for crate::ruml::read_self_action::ReadSelfAction {
    const TYPE_NAME: &'static str = "ReadSelfAction";
}

impl ContractType for crate::ruml::read_structural_feature_action::ReadStructuralFeatureAction {
    const TYPE_NAME: &'static str = "ReadStructuralFeatureAction";
}

impl ContractType for crate::ruml::read_variable_action::ReadVariableAction {
    const TYPE_NAME: &'static str = "ReadVariableAction";
}

impl ContractType for crate::ruml::realization::Realization {
    const TYPE_NAME: &'static str = "Realization";
}

impl ContractType for crate::ruml::receive_operation_event::ReceiveOperationEvent {
    const TYPE_NAME: &'static str = "ReceiveOperationEvent";
}

impl ContractType for crate::ruml::receive_signal_event::ReceiveSignalEvent {
    const TYPE_NAME: &'static str = "ReceiveSignalEvent";
}

impl ContractType for crate::ruml::reception::Reception {
    const TYPE_NAME: &'static str = "Reception";
}

impl ContractType for crate::ruml::reclassify_object_action::ReclassifyObjectAction {
    const TYPE_NAME: &'static str = "ReclassifyObjectAction";
}

impl ContractType for crate::ruml::redefinable_element::RedefinableElement {
    const TYPE_NAME: &'static str = "RedefinableElement";
}

impl ContractType for crate::ruml::redefinable_template_signature::RedefinableTemplateSignature {
    const TYPE_NAME: &'static str = "RedefinableTemplateSignature";
}

impl ContractType for crate::ruml::reduce_action::ReduceAction {
    const TYPE_NAME: &'static str = "ReduceAction";
}

impl ContractType for crate::ruml::region::Region {
    const TYPE_NAME: &'static str = "Region";
}

impl ContractType for crate::ruml::relationship::Relationship {
    const TYPE_NAME: &'static str = "Relationship";
}

impl ContractType
    for crate::ruml::remove_structural_feature_value_action::RemoveStructuralFeatureValueAction
{
    const TYPE_NAME: &'static str = "RemoveStructuralFeatureValueAction";
}

impl ContractType for crate::ruml::remove_variable_value_action::RemoveVariableValueAction {
    const TYPE_NAME: &'static str = "RemoveVariableValueAction";
}

impl ContractType for crate::ruml::reply_action::ReplyAction {
    const TYPE_NAME: &'static str = "ReplyAction";
}

impl ContractType for crate::ruml::send_object_action::SendObjectAction {
    const TYPE_NAME: &'static str = "SendObjectAction";
}

impl ContractType for crate::ruml::send_operation_event::SendOperationEvent {
    const TYPE_NAME: &'static str = "SendOperationEvent";
}

impl ContractType for crate::ruml::send_signal_action::SendSignalAction {
    const TYPE_NAME: &'static str = "SendSignalAction";
}

impl ContractType for crate::ruml::send_signal_event::SendSignalEvent {
    const TYPE_NAME: &'static str = "SendSignalEvent";
}

impl ContractType for crate::ruml::sequence_node::SequenceNode {
    const TYPE_NAME: &'static str = "SequenceNode";
}

impl ContractType for crate::ruml::signal::Signal {
    const TYPE_NAME: &'static str = "Signal";
}

impl ContractType for crate::ruml::signal_event::SignalEvent {
    const TYPE_NAME: &'static str = "SignalEvent";
}

impl ContractType for crate::ruml::slot::Slot {
    const TYPE_NAME: &'static str = "Slot";
}

impl ContractType for crate::ruml::start_classifier_behavior_action::StartClassifierBehaviorAction {
    const TYPE_NAME: &'static str = "StartClassifierBehaviorAction";
}

impl ContractType for crate::ruml::state::State {
    const TYPE_NAME: &'static str = "State";
}

impl ContractType for crate::ruml::state_invariant::StateInvariant {
    const TYPE_NAME: &'static str = "StateInvariant";
}

impl ContractType for crate::ruml::state_machine::StateMachine {
    const TYPE_NAME: &'static str = "StateMachine";
}

impl ContractType for crate::ruml::stereotype::Stereotype {
    const TYPE_NAME: &'static str = "Stereotype";
}

impl ContractType for crate::ruml::string_expression::StringExpression {
    const TYPE_NAME: &'static str = "StringExpression";
}

impl ContractType for crate::ruml::structural_feature::StructuralFeature {
    const TYPE_NAME: &'static str = "StructuralFeature";
}

impl ContractType for crate::ruml::structural_feature_action::StructuralFeatureAction {
    const TYPE_NAME: &'static str = "StructuralFeatureAction";
}

impl ContractType for crate::ruml::structured_activity_node::StructuredActivityNode {
    const TYPE_NAME: &'static str = "StructuredActivityNode";
}

impl ContractType for crate::ruml::structured_classifier::StructuredClassifier {
    const TYPE_NAME: &'static str = "StructuredClassifier";
}

impl ContractType for crate::ruml::substitution::Substitution {
    const TYPE_NAME: &'static str = "Substitution";
}

impl ContractType for crate::ruml::template_binding::TemplateBinding {
    const TYPE_NAME: &'static str = "TemplateBinding";
}

impl ContractType for crate::ruml::template_parameter::TemplateParameter {
    const TYPE_NAME: &'static str = "TemplateParameter";
}

impl ContractType for crate::ruml::template_parameter_substitution::TemplateParameterSubstitution {
    const TYPE_NAME: &'static str = "TemplateParameterSubstitution";
}

impl ContractType for crate::ruml::template_signature::TemplateSignature {
    const TYPE_NAME: &'static str = "TemplateSignature";
}

impl ContractType for crate::ruml::templateable_element::TemplateableElement {
    const TYPE_NAME: &'static str = "TemplateableElement";
}

impl ContractType for crate::ruml::test_identity_action::TestIdentityAction {
    const TYPE_NAME: &'static str = "TestIdentityAction";
}

impl ContractType for crate::ruml::time_constraint::TimeConstraint {
    const TYPE_NAME: &'static str = "TimeConstraint";
}

impl ContractType for crate::ruml::time_event::TimeEvent {
    const TYPE_NAME: &'static str = "TimeEvent";
}

impl ContractType for crate::ruml::time_expression::TimeExpression {
    const TYPE_NAME: &'static str = "TimeExpression";
}

impl ContractType for crate::ruml::time_interval::TimeInterval {
    const TYPE_NAME: &'static str = "TimeInterval";
}

impl ContractType for crate::ruml::time_observation::TimeObservation {
    const TYPE_NAME: &'static str = "TimeObservation";
}

impl ContractType for crate::ruml::transition::Transition {
    const TYPE_NAME: &'static str = "Transition";
}

impl ContractType for crate::ruml::trigger::Trigger {
    const TYPE_NAME: &'static str = "Trigger";
}

impl ContractType for crate::ruml::type_::Type {
    const TYPE_NAME: &'static str = "Type";
}

impl ContractType for crate::ruml::typed_element::TypedElement {
    const TYPE_NAME: &'static str = "TypedElement";
}

impl ContractType for crate::ruml::unmarshall_action::UnmarshallAction {
    const TYPE_NAME: &'static str = "UnmarshallAction";
}

impl ContractType for crate::ruml::usage::Usage {
    const TYPE_NAME: &'static str = "Usage";
}

impl ContractType for crate::ruml::use_case::UseCase {
    const TYPE_NAME: &'static str = "UseCase";
}

impl ContractType for crate::ruml::value_pin::ValuePin {
    const TYPE_NAME: &'static str = "ValuePin";
}

impl ContractType for crate::ruml::value_specification::ValueSpecification {
    const TYPE_NAME: &'static str = "ValueSpecification";
}

impl ContractType for crate::ruml::value_specification_action::ValueSpecificationAction {
    const TYPE_NAME: &'static str = "ValueSpecificationAction";
}

impl ContractType for crate::ruml::variable::Variable {
    const TYPE_NAME: &'static str = "Variable";
}

impl ContractType for crate::ruml::variable_action::VariableAction {
    const TYPE_NAME: &'static str = "VariableAction";
}

impl ContractType for crate::ruml::vertex::Vertex {
    const TYPE_NAME: &'static str = "Vertex";
}

impl ContractType for crate::ruml::write_link_action::WriteLinkAction {
    const TYPE_NAME: &'static str = "WriteLinkAction";
}

impl ContractType for crate::ruml::write_structural_feature_action::WriteStructuralFeatureAction {
    const TYPE_NAME: &'static str = "WriteStructuralFeatureAction";
}

impl ContractType for crate::ruml::write_variable_action::WriteVariableAction {
    const TYPE_NAME: &'static str = "WriteVariableAction";
}

impl ContractType for crate::ruml::abstraction::Abstraction {
    const TYPE_NAME: &'static str = "Abstraction";
}

impl ContractType for crate::ruml::accept_call_action::AcceptCallAction {
    const TYPE_NAME: &'static str = "AcceptCallAction";
}

impl ContractType for crate::ruml::accept_event_action::AcceptEventAction {
    const TYPE_NAME: &'static str = "AcceptEventAction";
}

impl ContractType for crate::ruml::action::Action {
    const TYPE_NAME: &'static str = "Action";
}

impl ContractType for crate::ruml::action_execution_specification::ActionExecutionSpecification {
    const TYPE_NAME: &'static str = "ActionExecutionSpecification";
}

impl ContractType for crate::ruml::action_input_pin::ActionInputPin {
    const TYPE_NAME: &'static str = "ActionInputPin";
}

impl ContractType for crate::ruml::activity::Activity {
    const TYPE_NAME: &'static str = "Activity";
}

impl ContractType for crate::ruml::activity_edge::ActivityEdge {
    const TYPE_NAME: &'static str = "ActivityEdge";
}

impl ContractType for crate::ruml::activity_final_node::ActivityFinalNode {
    const TYPE_NAME: &'static str = "ActivityFinalNode";
}

impl ContractType for crate::ruml::activity_group::ActivityGroup {
    const TYPE_NAME: &'static str = "ActivityGroup";
}

impl ContractType for crate::ruml::activity_node::ActivityNode {
    const TYPE_NAME: &'static str = "ActivityNode";
}

impl ContractType for crate::ruml::activity_parameter_node::ActivityParameterNode {
    const TYPE_NAME: &'static str = "ActivityParameterNode";
}

impl ContractType for crate::ruml::activity_partition::ActivityPartition {
    const TYPE_NAME: &'static str = "ActivityPartition";
}

impl ContractType for crate::ruml::actor::Actor {
    const TYPE_NAME: &'static str = "Actor";
}

impl ContractType
    for crate::ruml::add_structural_feature_value_action::AddStructuralFeatureValueAction
{
    const TYPE_NAME: &'static str = "AddStructuralFeatureValueAction";
}

impl ContractType for crate::ruml::add_variable_value_action::AddVariableValueAction {
    const TYPE_NAME: &'static str = "AddVariableValueAction";
}

impl ContractType for crate::ruml::any_receive_event::AnyReceiveEvent {
    const TYPE_NAME: &'static str = "AnyReceiveEvent";
}

impl ContractType for crate::ruml::artifact::Artifact {
    const TYPE_NAME: &'static str = "Artifact";
}

impl ContractType for crate::ruml::association::Association {
    const TYPE_NAME: &'static str = "Association";
}

impl ContractType for crate::ruml::association_class::AssociationClass {
    const TYPE_NAME: &'static str = "AssociationClass";
}

impl ContractType for crate::ruml::behavior::Behavior {
    const TYPE_NAME: &'static str = "Behavior";
}

impl ContractType
    for crate::ruml::behavior_execution_specification::BehaviorExecutionSpecification
{
    const TYPE_NAME: &'static str = "BehaviorExecutionSpecification";
}

impl ContractType for crate::ruml::behavioral_feature::BehavioralFeature {
    const TYPE_NAME: &'static str = "BehavioralFeature";
}

impl ContractType for crate::ruml::behaviored_classifier::BehavioredClassifier {
    const TYPE_NAME: &'static str = "BehavioredClassifier";
}

impl ContractType for crate::ruml::broadcast_signal_action::BroadcastSignalAction {
    const TYPE_NAME: &'static str = "BroadcastSignalAction";
}

impl ContractType for crate::ruml::call_action::CallAction {
    const TYPE_NAME: &'static str = "CallAction";
}

impl ContractType for crate::ruml::call_behavior_action::CallBehaviorAction {
    const TYPE_NAME: &'static str = "CallBehaviorAction";
}

impl ContractType for crate::ruml::call_event::CallEvent {
    const TYPE_NAME: &'static str = "CallEvent";
}

impl ContractType for crate::ruml::call_operation_action::CallOperationAction {
    const TYPE_NAME: &'static str = "CallOperationAction";
}
