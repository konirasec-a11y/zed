use super::{ShapeKind, Symbol, SymbolGroup};

pub(super) static FLOWCHART: &[SymbolGroup] = &[
    SymbolGroup {
        id: "flowchart-basic",
        title: "Common symbols",
        items: &[
            Symbol {
                id: "flowchart-basic-0",
                label: "Start / end",
                kind: ShapeKind::Terminator,
            },
            Symbol {
                id: "flowchart-basic-1",
                label: "Process",
                kind: ShapeKind::Process,
            },
            Symbol {
                id: "flowchart-basic-2",
                label: "Decision",
                kind: ShapeKind::Decision,
            },
            Symbol {
                id: "flowchart-basic-3",
                label: "Input / output",
                kind: ShapeKind::Input,
            },
            Symbol {
                id: "flowchart-basic-4",
                label: "Document",
                kind: ShapeKind::Document,
            },
            Symbol {
                id: "flowchart-basic-5",
                label: "Predefined process",
                kind: ShapeKind::Predefined,
            },
            Symbol {
                id: "flowchart-basic-6",
                label: "Database / storage",
                kind: ShapeKind::Database,
            },
            Symbol {
                id: "flowchart-basic-7",
                label: "Flow line",
                kind: ShapeKind::Relation,
            },
            Symbol {
                id: "flowchart-basic-8",
                label: "Connector",
                kind: ShapeKind::Connector,
            },
        ],
    },
    SymbolGroup {
        id: "flowchart-advanced",
        title: "Extended symbols",
        items: &[
            Symbol {
                id: "flowchart-advanced-0",
                label: "Manual input",
                kind: ShapeKind::Input,
            },
            Symbol {
                id: "flowchart-advanced-1",
                label: "Manual operation",
                kind: ShapeKind::Process,
            },
            Symbol {
                id: "flowchart-advanced-2",
                label: "Preparation",
                kind: ShapeKind::Process,
            },
            Symbol {
                id: "flowchart-advanced-3",
                label: "Delay",
                kind: ShapeKind::Terminator,
            },
            Symbol {
                id: "flowchart-advanced-4",
                label: "Display",
                kind: ShapeKind::Document,
            },
            Symbol {
                id: "flowchart-advanced-5",
                label: "Multiple documents",
                kind: ShapeKind::Document,
            },
            Symbol {
                id: "flowchart-advanced-6",
                label: "Stored data",
                kind: ShapeKind::Database,
            },
            Symbol {
                id: "flowchart-advanced-7",
                label: "Internal storage",
                kind: ShapeKind::Database,
            },
            Symbol {
                id: "flowchart-advanced-8",
                label: "Direct access storage",
                kind: ShapeKind::Database,
            },
            Symbol {
                id: "flowchart-advanced-9",
                label: "Sequential access storage",
                kind: ShapeKind::Database,
            },
            Symbol {
                id: "flowchart-advanced-10",
                label: "Off-page connector",
                kind: ShapeKind::Connector,
            },
            Symbol {
                id: "flowchart-advanced-11",
                label: "Annotation",
                kind: ShapeKind::Note,
            },
            Symbol {
                id: "flowchart-advanced-12",
                label: "Subprocess",
                kind: ShapeKind::Predefined,
            },
        ],
    },
    SymbolGroup {
        id: "flowchart-lanes",
        title: "Layout & connectors",
        items: &[
            Symbol {
                id: "flowchart-lanes-0",
                label: "Swimlane",
                kind: ShapeKind::Boundary,
            },
            Symbol {
                id: "flowchart-lanes-1",
                label: "Pool",
                kind: ShapeKind::Boundary,
            },
            Symbol {
                id: "flowchart-lanes-2",
                label: "Group",
                kind: ShapeKind::Boundary,
            },
            Symbol {
                id: "flowchart-lanes-3",
                label: "Yes / no branch",
                kind: ShapeKind::Relation,
            },
            Symbol {
                id: "flowchart-lanes-4",
                label: "Arrow connector",
                kind: ShapeKind::Relation,
            },
            Symbol {
                id: "flowchart-lanes-5",
                label: "Dotted connector",
                kind: ShapeKind::Relation,
            },
        ],
    },
];
