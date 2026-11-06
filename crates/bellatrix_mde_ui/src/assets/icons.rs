use crate::assets::symbols::ShapeKind;

pub fn glyph(kind: ShapeKind) -> &'static [u8] {
    match kind {
        ShapeKind::FoundMessage => include_bytes!("icons/svg/found-message.svg"),
        ShapeKind::LostMessage => include_bytes!("icons/svg/lost-message.svg"),
        ShapeKind::GeneralOrdering => include_bytes!("icons/svg/general-ordering.svg"),
        ShapeKind::InputPin => include_bytes!("icons/svg/input-pin.svg"),
        ShapeKind::OutputPin => include_bytes!("icons/svg/output-pin.svg"),
        ShapeKind::InteractionOperand => include_bytes!("icons/svg/interaction-operand.svg"),

        ShapeKind::InitialNode => include_bytes!("icons/svg/initial-node.svg"),
        ShapeKind::FinalNode => include_bytes!("icons/svg/final-node.svg"),
        ShapeKind::FlowFinal => include_bytes!("icons/svg/flow-final.svg"),
        ShapeKind::DashedLink => include_bytes!("icons/svg/dashed-link.svg"),
        ShapeKind::Extension => include_bytes!("icons/svg/extension.svg"),
        ShapeKind::AssociationClassAttachment => {
            include_bytes!("icons/svg/association-class-attachment.svg")
        }
        ShapeKind::Port => include_bytes!("icons/svg/port.svg"),
        ShapeKind::DeepHistory => include_bytes!("icons/svg/deep-history.svg"),
        ShapeKind::Junction => include_bytes!("icons/svg/junction.svg"),
        ShapeKind::History => include_bytes!("icons/svg/history.svg"),
        ShapeKind::Terminate => include_bytes!("icons/svg/terminate.svg"),
        ShapeKind::EntryPoint => include_bytes!("icons/svg/entry-point.svg"),
        ShapeKind::Collaboration => include_bytes!("icons/svg/collaboration.svg"),
        ShapeKind::Execution => include_bytes!("icons/svg/execution.svg"),
        ShapeKind::ObjectNode => include_bytes!("icons/svg/object-node.svg"),
        ShapeKind::SendSignal => include_bytes!("icons/svg/send-signal.svg"),
        ShapeKind::AcceptEvent => include_bytes!("icons/svg/accept-event.svg"),
        ShapeKind::Partition => include_bytes!("icons/svg/partition.svg"),
        ShapeKind::CombinedFragment => include_bytes!("icons/svg/combined-fragment.svg"),

        ShapeKind::Activity => include_bytes!("icons/svg/activity.svg"),
        ShapeKind::Actor => include_bytes!("icons/svg/actor.svg"),
        ShapeKind::Aggregate => include_bytes!("icons/svg/aggregate.svg"),
        ShapeKind::Association => include_bytes!("icons/svg/association.svg"),
        ShapeKind::Boundary => include_bytes!("icons/svg/boundary.svg"),
        ShapeKind::Branch => include_bytes!("icons/svg/branch.svg"),
        ShapeKind::Callout => include_bytes!("icons/svg/callout.svg"),
        ShapeKind::Class => include_bytes!("icons/svg/class.svg"),
        ShapeKind::Component => include_bytes!("icons/svg/component.svg"),
        ShapeKind::Composition => include_bytes!("icons/svg/composition.svg"),
        ShapeKind::Connector => include_bytes!("icons/svg/connector.svg"),
        ShapeKind::Container => include_bytes!("icons/svg/container.svg"),
        ShapeKind::Database => include_bytes!("icons/svg/database.svg"),
        ShapeKind::Decision => include_bytes!("icons/svg/decision.svg"),
        ShapeKind::Dependency => include_bytes!("icons/svg/dependency.svg"),
        ShapeKind::Deployment => include_bytes!("icons/svg/deployment.svg"),
        ShapeKind::Document => include_bytes!("icons/svg/document.svg"),
        ShapeKind::Enum => include_bytes!("icons/svg/enum.svg"),
        ShapeKind::External => include_bytes!("icons/svg/external.svg"),
        ShapeKind::Fork => include_bytes!("icons/svg/fork.svg"),
        ShapeKind::Inheritance => include_bytes!("icons/svg/inheritance.svg"),
        ShapeKind::Input => include_bytes!("icons/svg/input.svg"),
        ShapeKind::Interface => include_bytes!("icons/svg/interface.svg"),
        ShapeKind::ProvidedInterface => include_bytes!("icons/svg/provided-interface.svg"),
        ShapeKind::RequiredInterface => include_bytes!("icons/svg/required-interface.svg"),
        ShapeKind::Lifeline => include_bytes!("icons/svg/lifeline.svg"),
        ShapeKind::SynchronousMessage => include_bytes!("icons/svg/synchronous-message.svg"),
        ShapeKind::ReplyMessage => include_bytes!("icons/svg/reply-message.svg"),
        ShapeKind::Message => include_bytes!("icons/svg/message.svg"),
        ShapeKind::Note => include_bytes!("icons/svg/note.svg"),
        ShapeKind::Package => include_bytes!("icons/svg/package.svg"),
        ShapeKind::Person => include_bytes!("icons/svg/person.svg"),
        ShapeKind::Predefined => include_bytes!("icons/svg/predefined.svg"),
        ShapeKind::Process => include_bytes!("icons/svg/process.svg"),
        ShapeKind::Relation => include_bytes!("icons/svg/relation.svg"),
        ShapeKind::State => include_bytes!("icons/svg/state.svg"),
        ShapeKind::System => include_bytes!("icons/svg/system.svg"),
        ShapeKind::Terminator => include_bytes!("icons/svg/terminator.svg"),
        ShapeKind::Topic => include_bytes!("icons/svg/topic.svg"),
        ShapeKind::Usecase => include_bytes!("icons/svg/usecase.svg"),
    }
}

#[derive(Clone, Copy)]
pub enum IconCategory {
    Project,
    Element,
    Relation,
    Diagram,
}

pub fn uml_icon(name: &str, relationship: bool) -> (&'static [u8], IconCategory) {
    let name = canonical_name(name);
    let category = match name.as_str() {
        "Project" => IconCategory::Project,
        "Abstraction"
        | "ActivityInterrupt"
        | "Aggregation"
        | "Association"
        | "AssociationClassAttachment"
        | "AssociationClassLink"
        | "AssociationEnd"
        | "CommunicationPath"
        | "ComponentRealization"
        | "Composition"
        | "Connector"
        | "ConnectorEnd"
        | "ConstraintLink"
        | "Containment"
        | "ControlFlow"
        | "Dependency"
        | "Deployment"
        | "DirectedAssociation"
        | "DirectedLink"
        | "ElementImport"
        | "ExceptionHandler"
        | "Extend"
        | "Extension"
        | "GeneralOrdering"
        | "Generalization"
        | "Include"
        | "InformationFlow"
        | "InterfaceRealization"
        | "Link"
        | "LinkEnd"
        | "LinkObjectLink"
        | "Manifestation"
        | "Message"
        | "NoteLink"
        | "ObjectFlow"
        | "PackageImport"
        | "PackageMerge"
        | "ProfileApplication"
        | "Realization"
        | "RoleBinding"
        | "SelfConnector"
        | "SelfLink"
        | "SelfTransition"
        | "TemplateBinding"
        | "Transition"
        | "Usage" => IconCategory::Relation,
        name if name.ends_with("Diagram") => IconCategory::Diagram,
        _ if relationship => IconCategory::Relation,
        _ => IconCategory::Element,
    };
    let icon: &'static [u8] = match name.as_str() {
        "AcceptEventAction" | "AcceptSignal" | "Reception" | "Event" | "AcceptTimeEvent" => {
            include_bytes!("icons/svg/accept-event.svg")
        }
        "Action"
        | "Activity"
        | "OpaqueBehavior"
        | "StructuredActivityNode"
        | "DoActivity"
        | "Effect"
        | "EntryActivity"
        | "ExitActivity" => include_bytes!("icons/svg/activity.svg"),
        "Actor" => include_bytes!("icons/svg/actor.svg"),
        "Aggregation" => include_bytes!("icons/svg/aggregate.svg"),
        "Association" | "AssociationEnd" | "CommunicationPath" | "Link" | "LinkEnd"
        | "LinkObjectLink" | "SelfLink" => include_bytes!("icons/svg/association.svg"),
        "AssociationClass" => include_bytes!("icons/svg/association-class.svg"),
        "AssociationClassAttachment" | "AssociationClassLink" => {
            include_bytes!("icons/svg/association-class-attachment.svg")
        }
        "Attribute" | "EnumerationLiteral" | "Slot" => include_bytes!("icons/svg/attribute.svg"),
        "Boundary"
        | "Frame"
        | "Interaction"
        | "InteractionUse"
        | "UseCaseSubject"
        | "ExpansionRegion"
        | "InterruptibleActivityRegion" => include_bytes!("icons/svg/boundary.svg"),
        "Class" | "Template" | "MetaClass" | "Entity" | "Part" => {
            include_bytes!("icons/svg/class.svg")
        }
        "Collaboration" | "CollaborationUse" => include_bytes!("icons/svg/collaboration.svg"),
        "CombinedFragment" => include_bytes!("icons/svg/combined-fragment.svg"),
        "Component" => include_bytes!("icons/svg/component.svg"),
        "Composition" => include_bytes!("icons/svg/composition.svg"),
        "Gate" | "Endpoint" | "ActivityEdgeConnector" | "Control" | "TimeTick" => {
            include_bytes!("icons/svg/connector.svg")
        }
        "Constraint" | "DurationConstraint" | "TimeConstraint" => {
            include_bytes!("icons/svg/constraint.svg")
        }
        "Containment" => include_bytes!("icons/svg/containment.svg"),
        "NoteLink" | "ConstraintLink" => include_bytes!("icons/svg/dashed-link.svg"),
        "DataType" => include_bytes!("icons/svg/data-type.svg"),
        "DecisionNode" | "MergeNode" | "Choice" => include_bytes!("icons/svg/decision.svg"),
        "DeepHistory" => include_bytes!("icons/svg/deep-history.svg"),
        "Dependency" | "Abstraction" | "Usage" | "Include" | "Extend" | "ElementImport"
        | "PackageImport" | "PackageMerge" | "Deployment" | "Manifestation" | "RoleBinding"
        | "TemplateBinding" | "ProfileApplication" => include_bytes!("icons/svg/dependency.svg"),
        "Node" | "Device" | "ExecutionEnvironment" => include_bytes!("icons/svg/deployment.svg"),
        "ActivityDiagram"
        | "ClassDiagram"
        | "CommunicationDiagram"
        | "ComponentDiagram"
        | "CompositeStructureDiagram"
        | "DeploymentDiagram"
        | "InformationFlowDiagram"
        | "InteractionOverviewDiagram"
        | "ObjectDiagram"
        | "PackageDiagram"
        | "ProfileDiagram"
        | "SequenceDiagram"
        | "StatechartDiagram"
        | "TimingDiagram"
        | "UseCaseDiagram" => include_bytes!("icons/svg/diagram.svg"),
        "DirectedAssociation" | "DirectedLink" => {
            include_bytes!("icons/svg/directed-association.svg")
        }
        "Artifact" | "DeploymentSpecification" | "Image" => {
            include_bytes!("icons/svg/document.svg")
        }
        "Ellipse" => include_bytes!("icons/svg/ellipse.svg"),
        "EntryPoint" | "ConnectionPointReference" => include_bytes!("icons/svg/entry-point.svg"),
        "Enumeration" => include_bytes!("icons/svg/enum.svg"),
        "BehaviorExecutionSpecification" | "ExecutionSpecification" => {
            include_bytes!("icons/svg/execution.svg")
        }
        "Extension" => include_bytes!("icons/svg/extension.svg"),
        "ActivityFinalNode" | "FinalState" => include_bytes!("icons/svg/final-node.svg"),
        "FlowFinalNode" | "ExitPoint" => include_bytes!("icons/svg/flow-final.svg"),
        "ForkNode" | "JoinNode" | "Fork" | "Join" | "StateFork" | "StateJoin" => {
            include_bytes!("icons/svg/fork.svg")
        }
        "GeneralOrdering" => include_bytes!("icons/svg/general-ordering.svg"),
        "ShallowHistory" => include_bytes!("icons/svg/history.svg"),
        "Generalization" => include_bytes!("icons/svg/inheritance.svg"),
        "InitialNode" | "Pseudostate" | "InitialState" => {
            include_bytes!("icons/svg/initial-node.svg")
        }
        "InputPin" | "ValuePin" | "ExpansionNode" | "InputExpansionNode" => {
            include_bytes!("icons/svg/input-pin.svg")
        }
        "InteractionOperand" => include_bytes!("icons/svg/interaction-operand.svg"),
        "Interface" => include_bytes!("icons/svg/interface.svg"),
        "Junction" => include_bytes!("icons/svg/junction.svg"),
        "Lifeline" | "TimingLifeline" => include_bytes!("icons/svg/lifeline.svg"),
        "NaryAssociationNode" => include_bytes!("icons/svg/nary-association.svg"),
        "Note" | "ExtensionPoint" | "DurationObservation" | "TimeObservation" => {
            include_bytes!("icons/svg/note.svg")
        }
        "Object" | "ArtifactInstance" | "ComponentInstance" | "NodeInstance" => {
            include_bytes!("icons/svg/object.svg")
        }
        "ObjectNode" | "CentralBufferNode" | "DataStoreNode" | "ActivityParameterNode" => {
            include_bytes!("icons/svg/object-node.svg")
        }
        "Operation" => include_bytes!("icons/svg/operation.svg"),
        "OutputPin" | "OutputExpansionNode" => include_bytes!("icons/svg/output-pin.svg"),
        "Package" | "Model" | "Subsystem" | "Profile" => include_bytes!("icons/svg/package.svg"),
        "Parameter" | "TemplateParameter" | "TemplateParameterSubstitution" => {
            include_bytes!("icons/svg/parameter.svg")
        }
        "ActivityPartition" | "Region" | "SwimlaneHorz" | "SwimlaneVert" => {
            include_bytes!("icons/svg/partition.svg")
        }
        "Port" => include_bytes!("icons/svg/port.svg"),
        "PrimitiveType" => include_bytes!("icons/svg/primitive-type.svg"),
        "Project" => include_bytes!("icons/svg/project.svg"),
        "Realization" | "InterfaceRealization" | "ComponentRealization" => {
            include_bytes!("icons/svg/realization.svg")
        }
        "Rectangle" => include_bytes!("icons/svg/rectangle.svg"),
        "ControlFlow" | "ObjectFlow" | "Transition" | "SelfTransition" | "Connector"
        | "SelfConnector" | "ConnectorEnd" | "InformationFlow" | "ActivityInterrupt"
        | "ExceptionHandler" => include_bytes!("icons/svg/relation.svg"),
        "SendSignalAction" | "SendSignal" => include_bytes!("icons/svg/send-signal.svg"),
        "Signal" | "InformationItem" => include_bytes!("icons/svg/signal.svg"),
        "State" | "CompositeState" | "OrthogonalState" | "SubmachineState" | "StateMachine"
        | "StateInvariant" | "StateCondition" | "Continuation" | "TimeSegment" => {
            include_bytes!("icons/svg/state.svg")
        }
        "Stereotype" => include_bytes!("icons/svg/stereotype.svg"),
        "Message" => include_bytes!("icons/svg/synchronous-message.svg"),
        "Terminate" | "DestructionOccurrenceSpecification" => {
            include_bytes!("icons/svg/terminate.svg")
        }
        "Text" | "TextBox" => include_bytes!("icons/svg/text.svg"),
        "RoundedRectangle" => include_bytes!("icons/svg/topic.svg"),
        "UseCase" => include_bytes!("icons/svg/usecase.svg"),
        _ => include_bytes!("icons/svg/unknown.svg"),
    };
    (icon, category)
}

fn canonical_name(name: &str) -> String {
    let name = name.strip_prefix("UML").unwrap_or(name);
    let name = name.strip_suffix("View").unwrap_or(name);
    let mut upper = true;
    let name: String = name
        .chars()
        .filter_map(|character| {
            if character == '_' || character == ' ' {
                upper = true;
                None
            } else if upper {
                upper = false;
                Some(character.to_ascii_uppercase())
            } else {
                Some(character)
            }
        })
        .collect();
    match name.as_str() {
        "Metaclass" => "MetaClass".into(),
        "Property" => "Attribute".into(),
        "InstanceSpecification" => "Object".into(),
        "Comment" => "Note".into(),
        "OpaqueAction" => "Action".into(),
        "CallBehaviorAction" => "Action".into(),
        "CallOperationAction" => "Action".into(),
        "ProvidedInterface" => "Interface".into(),
        "RequiredInterface" => "Interface".into(),
        "AsyncMessage" => "Message".into(),
        "AsyncSignalMessage" => "Message".into(),
        "CreateMessage" => "Message".into(),
        "DeleteMessage" => "Message".into(),
        "ForwardMessage" => "Message".into(),
        "FoundMessage" => "Message".into(),
        "LostMessage" => "Message".into(),
        "ReplyMessage" => "Message".into(),
        "ReverseMessage" => "Message".into(),
        "SelfMessage" => "Message".into(),
        "AcceptEvent" => "AcceptEventAction".into(),
        "Decision" => "DecisionNode".into(),
        "Merge" => "MergeNode".into(),
        "ActivityFinal" => "ActivityFinalNode".into(),
        "FlowFinal" => "FlowFinalNode".into(),
        "Partition" => "ActivityPartition".into(),
        "InitialPseudostate" => "Pseudostate".into(),
        "History" => "ShallowHistory".into(),
        "Execution" => "BehaviorExecutionSpecification".into(),
        "SubjectBoundary" => "Frame".into(),
        "ActorLifeline" => "Actor".into(),
        "InteractionOverview" => "Interaction".into(),
        "TimingFrame" => "Frame".into(),
        "Fork/Join" => "ForkNode".into(),
        "StateMachineDiagram" => "StatechartDiagram".into(),
        "InteractionDiagram" => "SequenceDiagram".into(),
        "Label" => "Text".into(),
        "NodeLabel" => "Text".into(),
        "EdgeLabel" => "Text".into(),
        "RoundRect" => "RoundedRectangle".into(),
        _ => name,
    }
}
