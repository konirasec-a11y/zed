use crate::{ShapeKind, Symbol, SymbolGroup};

pub(super) static UML: &[SymbolGroup] = &[
    SymbolGroup {
        id: "uml-classes",
        title: "Classes & structure",
        items: &[
            Symbol {
                id: "uml-classes-0",
                label: "Class",
                kind: ShapeKind::Class,
            },
            Symbol {
                id: "uml-classes-1",
                label: "Interface",
                kind: ShapeKind::Interface,
            },
            Symbol {
                id: "uml-classes-2",
                label: "Enumeration",
                kind: ShapeKind::Enum,
            },
            Symbol {
                id: "uml-classes-3",
                label: "Primitive type",
                kind: ShapeKind::Class,
            },
            Symbol {
                id: "uml-classes-4",
                label: "Data type",
                kind: ShapeKind::Class,
            },
            Symbol {
                id: "uml-classes-5",
                label: "Signal",
                kind: ShapeKind::Class,
            },
            Symbol {
                id: "uml-classes-6",
                label: "Association class",
                kind: ShapeKind::Class,
            },
            Symbol {
                id: "uml-classes-7",
                label: "Template",
                kind: ShapeKind::Class,
            },
            Symbol {
                id: "uml-classes-8",
                label: "Package",
                kind: ShapeKind::Package,
            },
            Symbol {
                id: "uml-classes-9",
                label: "Model",
                kind: ShapeKind::Package,
            },
            Symbol {
                id: "uml-classes-10",
                label: "Subsystem",
                kind: ShapeKind::Package,
            },
            Symbol {
                id: "uml-classes-11",
                label: "Comment",
                kind: ShapeKind::Note,
            },
        ],
    },
    SymbolGroup {
        id: "uml-relationships",
        title: "Relationships",
        items: &[
            Symbol {
                id: "uml-relationships-0",
                label: "Association",
                kind: ShapeKind::Association,
            },
            Symbol {
                id: "uml-relationships-1",
                label: "Directed association",
                kind: ShapeKind::Association,
            },
            Symbol {
                id: "uml-relationships-2",
                label: "Aggregation",
                kind: ShapeKind::Aggregate,
            },
            Symbol {
                id: "uml-relationships-3",
                label: "Composition",
                kind: ShapeKind::Composition,
            },
            Symbol {
                id: "uml-relationships-4",
                label: "Dependency",
                kind: ShapeKind::Dependency,
            },
            Symbol {
                id: "uml-relationships-5",
                label: "Generalization",
                kind: ShapeKind::Inheritance,
            },
            Symbol {
                id: "uml-relationships-6",
                label: "Interface realization",
                kind: ShapeKind::Inheritance,
            },
            Symbol {
                id: "uml-relationships-7",
                label: "Abstraction",
                kind: ShapeKind::Dependency,
            },
            Symbol {
                id: "uml-relationships-8",
                label: "Usage",
                kind: ShapeKind::Dependency,
            },
            Symbol {
                id: "uml-relationships-9",
                label: "Containment",
                kind: ShapeKind::Association,
            },
            Symbol {
                id: "uml-relationships-10",
                label: "Information flow",
                kind: ShapeKind::Association,
            },
        ],
    },
    SymbolGroup {
        id: "uml-usecase",
        title: "Use cases",
        items: &[
            Symbol {
                id: "uml-usecase-0",
                label: "Actor",
                kind: ShapeKind::Actor,
            },
            Symbol {
                id: "uml-usecase-1",
                label: "Use case",
                kind: ShapeKind::Usecase,
            },
            Symbol {
                id: "uml-usecase-2",
                label: "Subject boundary",
                kind: ShapeKind::Boundary,
            },
            Symbol {
                id: "uml-usecase-3",
                label: "Include",
                kind: ShapeKind::Dependency,
            },
            Symbol {
                id: "uml-usecase-4",
                label: "Extend",
                kind: ShapeKind::Dependency,
            },
            Symbol {
                id: "uml-usecase-5",
                label: "Extension point",
                kind: ShapeKind::Note,
            },
            Symbol {
                id: "uml-usecase-6",
                label: "Generalization",
                kind: ShapeKind::Inheritance,
            },
        ],
    },
    SymbolGroup {
        id: "uml-sequence",
        title: "Sequence & interaction",
        items: &[
            Symbol {
                id: "uml-sequence-0",
                label: "Lifeline",
                kind: ShapeKind::Lifeline,
            },
            Symbol {
                id: "uml-sequence-1",
                label: "Actor lifeline",
                kind: ShapeKind::Actor,
            },
            Symbol {
                id: "uml-sequence-2",
                label: "Message",
                kind: ShapeKind::SynchronousMessage,
            },
            Symbol {
                id: "uml-sequence-3",
                label: "Async message",
                kind: ShapeKind::Message,
            },
            Symbol {
                id: "uml-sequence-4",
                label: "Reply message",
                kind: ShapeKind::ReplyMessage,
            },
            Symbol {
                id: "uml-sequence-5",
                label: "Create message",
                kind: ShapeKind::Message,
            },
            Symbol {
                id: "uml-sequence-6",
                label: "Delete message",
                kind: ShapeKind::Message,
            },
            Symbol {
                id: "uml-sequence-7",
                label: "Found message",
                kind: ShapeKind::Message,
            },
            Symbol {
                id: "uml-sequence-8",
                label: "Lost message",
                kind: ShapeKind::Message,
            },
            Symbol {
                id: "uml-sequence-9",
                label: "Self message",
                kind: ShapeKind::Message,
            },
            Symbol {
                id: "uml-sequence-10",
                label: "Execution",
                kind: ShapeKind::Execution,
            },
            Symbol {
                id: "uml-sequence-11",
                label: "Combined fragment",
                kind: ShapeKind::CombinedFragment,
            },
            Symbol {
                id: "uml-sequence-12",
                label: "Interaction use",
                kind: ShapeKind::Boundary,
            },
            Symbol {
                id: "uml-sequence-13",
                label: "State invariant",
                kind: ShapeKind::State,
            },
            Symbol {
                id: "uml-sequence-14",
                label: "Gate",
                kind: ShapeKind::Connector,
            },
            Symbol {
                id: "uml-sequence-15",
                label: "Duration constraint",
                kind: ShapeKind::Note,
            },
        ],
    },
    SymbolGroup {
        id: "uml-activity",
        title: "Activities",
        items: &[
            Symbol {
                id: "uml-activity-0",
                label: "Initial node",
                kind: ShapeKind::InitialNode,
            },
            Symbol {
                id: "uml-activity-1",
                label: "Action",
                kind: ShapeKind::Activity,
            },
            Symbol {
                id: "uml-activity-2",
                label: "Activity",
                kind: ShapeKind::Activity,
            },
            Symbol {
                id: "uml-activity-3",
                label: "Decision",
                kind: ShapeKind::Decision,
            },
            Symbol {
                id: "uml-activity-4",
                label: "Merge",
                kind: ShapeKind::Decision,
            },
            Symbol {
                id: "uml-activity-5",
                label: "Fork",
                kind: ShapeKind::Fork,
            },
            Symbol {
                id: "uml-activity-6",
                label: "Join",
                kind: ShapeKind::Fork,
            },
            Symbol {
                id: "uml-activity-7",
                label: "Activity final",
                kind: ShapeKind::FinalNode,
            },
            Symbol {
                id: "uml-activity-8",
                label: "Flow final",
                kind: ShapeKind::FlowFinal,
            },
            Symbol {
                id: "uml-activity-9",
                label: "Object node",
                kind: ShapeKind::ObjectNode,
            },
            Symbol {
                id: "uml-activity-10",
                label: "Input pin",
                kind: ShapeKind::InputPin,
            },
            Symbol {
                id: "uml-activity-11",
                label: "Output pin",
                kind: ShapeKind::OutputPin,
            },
            Symbol {
                id: "uml-activity-12",
                label: "Control flow",
                kind: ShapeKind::Relation,
            },
            Symbol {
                id: "uml-activity-13",
                label: "Object flow",
                kind: ShapeKind::Relation,
            },
            Symbol {
                id: "uml-activity-14",
                label: "Partition",
                kind: ShapeKind::Partition,
            },
            Symbol {
                id: "uml-activity-15",
                label: "Expansion region",
                kind: ShapeKind::Boundary,
            },
            Symbol {
                id: "uml-activity-16",
                label: "Accept event",
                kind: ShapeKind::AcceptEvent,
            },
        ],
    },
    SymbolGroup {
        id: "uml-state",
        title: "State machines",
        items: &[
            Symbol {
                id: "uml-state-0",
                label: "State",
                kind: ShapeKind::State,
            },
            Symbol {
                id: "uml-state-1",
                label: "Initial pseudostate",
                kind: ShapeKind::InitialNode,
            },
            Symbol {
                id: "uml-state-2",
                label: "Final state",
                kind: ShapeKind::FinalNode,
            },
            Symbol {
                id: "uml-state-3",
                label: "Choice",
                kind: ShapeKind::Decision,
            },
            Symbol {
                id: "uml-state-4",
                label: "Junction",
                kind: ShapeKind::Junction,
            },
            Symbol {
                id: "uml-state-5",
                label: "Fork / join",
                kind: ShapeKind::Fork,
            },
            Symbol {
                id: "uml-state-6",
                label: "History",
                kind: ShapeKind::History,
            },
            Symbol {
                id: "uml-state-7",
                label: "Entry point",
                kind: ShapeKind::EntryPoint,
            },
            Symbol {
                id: "uml-state-8",
                label: "Exit point",
                kind: ShapeKind::FlowFinal,
            },
            Symbol {
                id: "uml-state-9",
                label: "Transition",
                kind: ShapeKind::Relation,
            },
            Symbol {
                id: "uml-state-10",
                label: "Region",
                kind: ShapeKind::Partition,
            },
            Symbol {
                id: "uml-state-11",
                label: "Terminate",
                kind: ShapeKind::Terminate,
            },
        ],
    },
    SymbolGroup {
        id: "uml-components",
        title: "Components & deployment",
        items: &[
            Symbol {
                id: "uml-components-0",
                label: "Component",
                kind: ShapeKind::Component,
            },
            Symbol {
                id: "uml-components-1",
                label: "Artifact",
                kind: ShapeKind::Document,
            },
            Symbol {
                id: "uml-components-2",
                label: "Port",
                kind: ShapeKind::Port,
            },
            Symbol {
                id: "uml-components-3",
                label: "Part",
                kind: ShapeKind::Class,
            },
            Symbol {
                id: "uml-components-4",
                label: "Provided interface",
                kind: ShapeKind::Interface,
            },
            Symbol {
                id: "uml-components-5",
                label: "Required interface",
                kind: ShapeKind::Interface,
            },
            Symbol {
                id: "uml-components-6",
                label: "Connector",
                kind: ShapeKind::Relation,
            },
            Symbol {
                id: "uml-components-7",
                label: "Node",
                kind: ShapeKind::Deployment,
            },
            Symbol {
                id: "uml-components-8",
                label: "Device",
                kind: ShapeKind::Deployment,
            },
            Symbol {
                id: "uml-components-9",
                label: "Execution environment",
                kind: ShapeKind::Deployment,
            },
            Symbol {
                id: "uml-components-10",
                label: "Deployment specification",
                kind: ShapeKind::Document,
            },
            Symbol {
                id: "uml-components-11",
                label: "Communication path",
                kind: ShapeKind::Association,
            },
            Symbol {
                id: "uml-components-12",
                label: "Deployment",
                kind: ShapeKind::Dependency,
            },
            Symbol {
                id: "uml-components-13",
                label: "Manifestation",
                kind: ShapeKind::Dependency,
            },
        ],
    },
    SymbolGroup {
        id: "uml-others",
        title: "Other UML diagrams",
        items: &[
            Symbol {
                id: "uml-others-0",
                label: "Instance specification",
                kind: ShapeKind::Class,
            },
            Symbol {
                id: "uml-others-1",
                label: "Slot",
                kind: ShapeKind::Class,
            },
            Symbol {
                id: "uml-others-2",
                label: "Object",
                kind: ShapeKind::Class,
            },
            Symbol {
                id: "uml-others-3",
                label: "Collaboration",
                kind: ShapeKind::Collaboration,
            },
            Symbol {
                id: "uml-others-4",
                label: "Interaction overview",
                kind: ShapeKind::Boundary,
            },
            Symbol {
                id: "uml-others-5",
                label: "Timing frame",
                kind: ShapeKind::Boundary,
            },
            Symbol {
                id: "uml-others-6",
                label: "Profile",
                kind: ShapeKind::Package,
            },
            Symbol {
                id: "uml-others-7",
                label: "Stereotype",
                kind: ShapeKind::Class,
            },
            Symbol {
                id: "uml-others-8",
                label: "Metaclass",
                kind: ShapeKind::Class,
            },
            Symbol {
                id: "uml-others-9",
                label: "Extension",
                kind: ShapeKind::Extension,
            },
            Symbol {
                id: "uml-others-10",
                label: "Constraint",
                kind: ShapeKind::Note,
            },
            Symbol {
                id: "uml-others-11",
                label: "Time observation",
                kind: ShapeKind::Note,
            },
            Symbol {
                id: "uml-others-12",
                label: "Duration observation",
                kind: ShapeKind::Note,
            },
        ],
    },
];
