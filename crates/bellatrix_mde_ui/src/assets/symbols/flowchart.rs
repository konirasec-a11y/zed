use super::{ShapeKind, Symbol, SymbolGroup};

pub(super) static FLOWCHART: &[SymbolGroup] = &[
    SymbolGroup {
        id: "flowchart-basic",
        title: "Common symbols",
        items: &[
            Symbol {
                id: "flowchart-basic-0",
                label: "Start / end",
                kind: Some(ShapeKind::Terminator),
            },
            Symbol {
                id: "flowchart-basic-1",
                label: "Process",
                kind: Some(ShapeKind::Process),
            },
            Symbol {
                id: "flowchart-basic-2",
                label: "Decision",
                kind: Some(ShapeKind::Decision),
            },
            Symbol {
                id: "flowchart-basic-3",
                label: "Input / output",
                kind: Some(ShapeKind::Input),
            },
            Symbol {
                id: "flowchart-basic-4",
                label: "Document",
                kind: Some(ShapeKind::Document),
            },
            Symbol {
                id: "flowchart-basic-5",
                label: "Predefined process",
                kind: Some(ShapeKind::Predefined),
            },
            Symbol {
                id: "flowchart-basic-6",
                label: "Database / storage",
                kind: Some(ShapeKind::Database),
            },
            Symbol {
                id: "flowchart-basic-7",
                label: "Flow line",
                kind: Some(ShapeKind::Relation),
            },
            Symbol {
                id: "flowchart-basic-8",
                label: "Connector",
                kind: Some(ShapeKind::Connector),
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
                kind: Some(ShapeKind::Input),
            },
            Symbol {
                id: "flowchart-advanced-1",
                label: "Manual operation",
                kind: Some(ShapeKind::Process),
            },
            Symbol {
                id: "flowchart-advanced-2",
                label: "Preparation",
                kind: Some(ShapeKind::Process),
            },
            Symbol {
                id: "flowchart-advanced-3",
                label: "Delay",
                kind: Some(ShapeKind::Terminator),
            },
            Symbol {
                id: "flowchart-advanced-4",
                label: "Display",
                kind: Some(ShapeKind::Document),
            },
            Symbol {
                id: "flowchart-advanced-5",
                label: "Multiple documents",
                kind: Some(ShapeKind::Document),
            },
            Symbol {
                id: "flowchart-advanced-6",
                label: "Stored data",
                kind: Some(ShapeKind::Database),
            },
            Symbol {
                id: "flowchart-advanced-7",
                label: "Internal storage",
                kind: Some(ShapeKind::Database),
            },
            Symbol {
                id: "flowchart-advanced-8",
                label: "Direct access storage",
                kind: Some(ShapeKind::Database),
            },
            Symbol {
                id: "flowchart-advanced-9",
                label: "Sequential access storage",
                kind: Some(ShapeKind::Database),
            },
            Symbol {
                id: "flowchart-advanced-10",
                label: "Off-page connector",
                kind: Some(ShapeKind::Connector),
            },
            Symbol {
                id: "flowchart-advanced-11",
                label: "Annotation",
                kind: Some(ShapeKind::Note),
            },
            Symbol {
                id: "flowchart-advanced-12",
                label: "Subprocess",
                kind: Some(ShapeKind::Predefined),
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
                kind: Some(ShapeKind::Boundary),
            },
            Symbol {
                id: "flowchart-lanes-1",
                label: "Pool",
                kind: Some(ShapeKind::Boundary),
            },
            Symbol {
                id: "flowchart-lanes-2",
                label: "Group",
                kind: Some(ShapeKind::Boundary),
            },
            Symbol {
                id: "flowchart-lanes-3",
                label: "Yes / no branch",
                kind: Some(ShapeKind::Relation),
            },
            Symbol {
                id: "flowchart-lanes-4",
                label: "Arrow connector",
                kind: Some(ShapeKind::Relation),
            },
            Symbol {
                id: "flowchart-lanes-5",
                label: "Dotted connector",
                kind: Some(ShapeKind::Relation),
            },
        ],
    },
];
