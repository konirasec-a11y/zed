use super::{ShapeKind, Symbol, SymbolGroup};

pub(super) static C4: &[SymbolGroup] = &[
    SymbolGroup {
        id: "c4-context",
        title: "System context",
        items: &[
            Symbol {
                id: "c4-context-0",
                label: "Person",
                kind: Some(ShapeKind::Person),
            },
            Symbol {
                id: "c4-context-1",
                label: "External person",
                kind: Some(ShapeKind::Person),
            },
            Symbol {
                id: "c4-context-2",
                label: "Software system",
                kind: Some(ShapeKind::System),
            },
            Symbol {
                id: "c4-context-3",
                label: "External system",
                kind: Some(ShapeKind::External),
            },
            Symbol {
                id: "c4-context-4",
                label: "Enterprise boundary",
                kind: Some(ShapeKind::Boundary),
            },
            Symbol {
                id: "c4-context-5",
                label: "System boundary",
                kind: Some(ShapeKind::Boundary),
            },
            Symbol {
                id: "c4-context-6",
                label: "Relationship",
                kind: Some(ShapeKind::Relation),
            },
            Symbol {
                id: "c4-context-7",
                label: "Bidirectional relationship",
                kind: Some(ShapeKind::Relation),
            },
        ],
    },
    SymbolGroup {
        id: "c4-containers",
        title: "Container view",
        items: &[
            Symbol {
                id: "c4-containers-0",
                label: "Container / application",
                kind: Some(ShapeKind::Container),
            },
            Symbol {
                id: "c4-containers-1",
                label: "Web application",
                kind: Some(ShapeKind::Container),
            },
            Symbol {
                id: "c4-containers-2",
                label: "Mobile application",
                kind: Some(ShapeKind::Container),
            },
            Symbol {
                id: "c4-containers-3",
                label: "Microservice",
                kind: Some(ShapeKind::Container),
            },
            Symbol {
                id: "c4-containers-4",
                label: "API",
                kind: Some(ShapeKind::Component),
            },
            Symbol {
                id: "c4-containers-5",
                label: "Message broker",
                kind: Some(ShapeKind::Container),
            },
            Symbol {
                id: "c4-containers-6",
                label: "Database",
                kind: Some(ShapeKind::Database),
            },
            Symbol {
                id: "c4-containers-7",
                label: "Data store",
                kind: Some(ShapeKind::Database),
            },
            Symbol {
                id: "c4-containers-8",
                label: "Container boundary",
                kind: Some(ShapeKind::Boundary),
            },
        ],
    },
    SymbolGroup {
        id: "c4-components",
        title: "Component view",
        items: &[
            Symbol {
                id: "c4-components-0",
                label: "Component",
                kind: Some(ShapeKind::Component),
            },
            Symbol {
                id: "c4-components-1",
                label: "External component",
                kind: Some(ShapeKind::External),
            },
            Symbol {
                id: "c4-components-2",
                label: "Component boundary",
                kind: Some(ShapeKind::Boundary),
            },
            Symbol {
                id: "c4-components-3",
                label: "Dependency",
                kind: Some(ShapeKind::Relation),
            },
            Symbol {
                id: "c4-components-4",
                label: "Asynchronous relationship",
                kind: Some(ShapeKind::Relation),
            },
        ],
    },
    SymbolGroup {
        id: "c4-support",
        title: "Supporting views",
        items: &[
            Symbol {
                id: "c4-support-0",
                label: "Deployment node",
                kind: Some(ShapeKind::Deployment),
            },
            Symbol {
                id: "c4-support-1",
                label: "Infrastructure node",
                kind: Some(ShapeKind::Deployment),
            },
            Symbol {
                id: "c4-support-2",
                label: "Deployment environment",
                kind: Some(ShapeKind::Boundary),
            },
            Symbol {
                id: "c4-support-3",
                label: "Container instance",
                kind: Some(ShapeKind::Container),
            },
            Symbol {
                id: "c4-support-4",
                label: "Dynamic step",
                kind: Some(ShapeKind::Message),
            },
            Symbol {
                id: "c4-support-5",
                label: "System landscape",
                kind: Some(ShapeKind::System),
            },
            Symbol {
                id: "c4-support-6",
                label: "Code element",
                kind: Some(ShapeKind::Class),
            },
            Symbol {
                id: "c4-support-7",
                label: "Technology label",
                kind: Some(ShapeKind::Note),
            },
            Symbol {
                id: "c4-support-8",
                label: "Description",
                kind: Some(ShapeKind::Note),
            },
        ],
    },
];
