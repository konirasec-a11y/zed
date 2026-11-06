use super::{Symbol, SymbolGroup};

pub(super) static UML: &[SymbolGroup] = &[
    SymbolGroup {
        id: "uml-classes",
        title: "Classes & structure",
        items: &[
            Symbol {
                id: "uml-classes-0",
                label: "Class",
                kind: None,
            },
            Symbol {
                id: "uml-classes-1",
                label: "Interface",
                kind: None,
            },
            Symbol {
                id: "uml-classes-2",
                label: "Enumeration",
                kind: None,
            },
            Symbol {
                id: "uml-classes-3",
                label: "Primitive type",
                kind: None,
            },
            Symbol {
                id: "uml-classes-4",
                label: "Data type",
                kind: None,
            },
            Symbol {
                id: "uml-classes-5",
                label: "Signal",
                kind: None,
            },
            Symbol {
                id: "uml-classes-6",
                label: "Association class",
                kind: None,
            },
            Symbol {
                id: "uml-classes-7",
                label: "Template",
                kind: None,
            },
            Symbol {
                id: "uml-classes-8",
                label: "Package",
                kind: None,
            },
            Symbol {
                id: "uml-classes-9",
                label: "Model",
                kind: None,
            },
            Symbol {
                id: "uml-classes-10",
                label: "Subsystem",
                kind: None,
            },
            Symbol {
                id: "uml-classes-11",
                label: "Comment",
                kind: None,
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
                kind: None,
            },
            Symbol {
                id: "uml-relationships-1",
                label: "Directed association",
                kind: None,
            },
            Symbol {
                id: "uml-relationships-2",
                label: "Aggregation",
                kind: None,
            },
            Symbol {
                id: "uml-relationships-3",
                label: "Composition",
                kind: None,
            },
            Symbol {
                id: "uml-relationships-4",
                label: "Dependency",
                kind: None,
            },
            Symbol {
                id: "uml-relationships-5",
                label: "Generalization",
                kind: None,
            },
            Symbol {
                id: "uml-relationships-6",
                label: "Interface realization",
                kind: None,
            },
            Symbol {
                id: "uml-relationships-7",
                label: "Abstraction",
                kind: None,
            },
            Symbol {
                id: "uml-relationships-8",
                label: "Usage",
                kind: None,
            },
            Symbol {
                id: "uml-relationships-9",
                label: "Containment",
                kind: None,
            },
            Symbol {
                id: "uml-relationships-10",
                label: "Information flow",
                kind: None,
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
                kind: None,
            },
            Symbol {
                id: "uml-usecase-1",
                label: "Use case",
                kind: None,
            },
            Symbol {
                id: "uml-usecase-2",
                label: "Subject boundary",
                kind: None,
            },
            Symbol {
                id: "uml-usecase-3",
                label: "Include",
                kind: None,
            },
            Symbol {
                id: "uml-usecase-4",
                label: "Extend",
                kind: None,
            },
            Symbol {
                id: "uml-usecase-5",
                label: "Extension point",
                kind: None,
            },
            Symbol {
                id: "uml-usecase-6",
                label: "Generalization",
                kind: None,
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
                kind: None,
            },
            Symbol {
                id: "uml-sequence-1",
                label: "Actor lifeline",
                kind: None,
            },
            Symbol {
                id: "uml-sequence-2",
                label: "Message",
                kind: None,
            },
            Symbol {
                id: "uml-sequence-3",
                label: "Async message",
                kind: None,
            },
            Symbol {
                id: "uml-sequence-4",
                label: "Reply message",
                kind: None,
            },
            Symbol {
                id: "uml-sequence-5",
                label: "Create message",
                kind: None,
            },
            Symbol {
                id: "uml-sequence-6",
                label: "Delete message",
                kind: None,
            },
            Symbol {
                id: "uml-sequence-7",
                label: "Found message",
                kind: None,
            },
            Symbol {
                id: "uml-sequence-8",
                label: "Lost message",
                kind: None,
            },
            Symbol {
                id: "uml-sequence-9",
                label: "Self message",
                kind: None,
            },
            Symbol {
                id: "uml-sequence-10",
                label: "Execution",
                kind: None,
            },
            Symbol {
                id: "uml-sequence-11",
                label: "Combined fragment",
                kind: None,
            },
            Symbol {
                id: "uml-sequence-12",
                label: "Interaction use",
                kind: None,
            },
            Symbol {
                id: "uml-sequence-13",
                label: "State invariant",
                kind: None,
            },
            Symbol {
                id: "uml-sequence-14",
                label: "Gate",
                kind: None,
            },
            Symbol {
                id: "uml-sequence-15",
                label: "Duration constraint",
                kind: None,
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
                kind: None,
            },
            Symbol {
                id: "uml-activity-1",
                label: "Action",
                kind: None,
            },
            Symbol {
                id: "uml-activity-2",
                label: "Activity",
                kind: None,
            },
            Symbol {
                id: "uml-activity-3",
                label: "Decision",
                kind: None,
            },
            Symbol {
                id: "uml-activity-4",
                label: "Merge",
                kind: None,
            },
            Symbol {
                id: "uml-activity-5",
                label: "Fork",
                kind: None,
            },
            Symbol {
                id: "uml-activity-6",
                label: "Join",
                kind: None,
            },
            Symbol {
                id: "uml-activity-7",
                label: "Activity final",
                kind: None,
            },
            Symbol {
                id: "uml-activity-8",
                label: "Flow final",
                kind: None,
            },
            Symbol {
                id: "uml-activity-9",
                label: "Object node",
                kind: None,
            },
            Symbol {
                id: "uml-activity-10",
                label: "Input pin",
                kind: None,
            },
            Symbol {
                id: "uml-activity-11",
                label: "Output pin",
                kind: None,
            },
            Symbol {
                id: "uml-activity-12",
                label: "Control flow",
                kind: None,
            },
            Symbol {
                id: "uml-activity-13",
                label: "Object flow",
                kind: None,
            },
            Symbol {
                id: "uml-activity-14",
                label: "Partition",
                kind: None,
            },
            Symbol {
                id: "uml-activity-15",
                label: "Expansion region",
                kind: None,
            },
            Symbol {
                id: "uml-activity-16",
                label: "Accept event",
                kind: None,
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
                kind: None,
            },
            Symbol {
                id: "uml-state-1",
                label: "Initial pseudostate",
                kind: None,
            },
            Symbol {
                id: "uml-state-2",
                label: "Final state",
                kind: None,
            },
            Symbol {
                id: "uml-state-3",
                label: "Choice",
                kind: None,
            },
            Symbol {
                id: "uml-state-4",
                label: "Junction",
                kind: None,
            },
            Symbol {
                id: "uml-state-5",
                label: "Fork / join",
                kind: None,
            },
            Symbol {
                id: "uml-state-6",
                label: "History",
                kind: None,
            },
            Symbol {
                id: "uml-state-7",
                label: "Entry point",
                kind: None,
            },
            Symbol {
                id: "uml-state-8",
                label: "Exit point",
                kind: None,
            },
            Symbol {
                id: "uml-state-9",
                label: "Transition",
                kind: None,
            },
            Symbol {
                id: "uml-state-10",
                label: "Region",
                kind: None,
            },
            Symbol {
                id: "uml-state-11",
                label: "Terminate",
                kind: None,
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
                kind: None,
            },
            Symbol {
                id: "uml-components-1",
                label: "Artifact",
                kind: None,
            },
            Symbol {
                id: "uml-components-2",
                label: "Port",
                kind: None,
            },
            Symbol {
                id: "uml-components-3",
                label: "Part",
                kind: None,
            },
            Symbol {
                id: "uml-components-4",
                label: "Provided interface",
                kind: None,
            },
            Symbol {
                id: "uml-components-5",
                label: "Required interface",
                kind: None,
            },
            Symbol {
                id: "uml-components-6",
                label: "Connector",
                kind: None,
            },
            Symbol {
                id: "uml-components-7",
                label: "Node",
                kind: None,
            },
            Symbol {
                id: "uml-components-8",
                label: "Device",
                kind: None,
            },
            Symbol {
                id: "uml-components-9",
                label: "Execution environment",
                kind: None,
            },
            Symbol {
                id: "uml-components-10",
                label: "Deployment specification",
                kind: None,
            },
            Symbol {
                id: "uml-components-11",
                label: "Communication path",
                kind: None,
            },
            Symbol {
                id: "uml-components-12",
                label: "Deployment",
                kind: None,
            },
            Symbol {
                id: "uml-components-13",
                label: "Manifestation",
                kind: None,
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
                kind: None,
            },
            Symbol {
                id: "uml-others-1",
                label: "Slot",
                kind: None,
            },
            Symbol {
                id: "uml-others-2",
                label: "Object",
                kind: None,
            },
            Symbol {
                id: "uml-others-3",
                label: "Collaboration",
                kind: None,
            },
            Symbol {
                id: "uml-others-4",
                label: "Interaction overview",
                kind: None,
            },
            Symbol {
                id: "uml-others-5",
                label: "Timing frame",
                kind: None,
            },
            Symbol {
                id: "uml-others-6",
                label: "Profile",
                kind: None,
            },
            Symbol {
                id: "uml-others-7",
                label: "Stereotype",
                kind: None,
            },
            Symbol {
                id: "uml-others-8",
                label: "Metaclass",
                kind: None,
            },
            Symbol {
                id: "uml-others-9",
                label: "Extension",
                kind: None,
            },
            Symbol {
                id: "uml-others-10",
                label: "Constraint",
                kind: None,
            },
            Symbol {
                id: "uml-others-11",
                label: "Time observation",
                kind: None,
            },
            Symbol {
                id: "uml-others-12",
                label: "Duration observation",
                kind: None,
            },
        ],
    },
];
