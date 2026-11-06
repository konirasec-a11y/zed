use super::{ShapeKind, Symbol, SymbolGroup};

pub(super) static MINDMAP: &[SymbolGroup] = &[
    SymbolGroup {
        id: "mindmap-topics",
        title: "Topics",
        items: &[
            Symbol {
                id: "mindmap-topics-0",
                label: "Central topic",
                kind: Some(ShapeKind::Topic),
            },
            Symbol {
                id: "mindmap-topics-1",
                label: "Main topic",
                kind: Some(ShapeKind::Branch),
            },
            Symbol {
                id: "mindmap-topics-2",
                label: "Subtopic",
                kind: Some(ShapeKind::Branch),
            },
            Symbol {
                id: "mindmap-topics-3",
                label: "Sibling topic",
                kind: Some(ShapeKind::Branch),
            },
            Symbol {
                id: "mindmap-topics-4",
                label: "Floating topic",
                kind: Some(ShapeKind::Topic),
            },
            Symbol {
                id: "mindmap-topics-5",
                label: "Callout",
                kind: Some(ShapeKind::Callout),
            },
        ],
    },
    SymbolGroup {
        id: "mindmap-structure",
        title: "Structure",
        items: &[
            Symbol {
                id: "mindmap-structure-0",
                label: "Branch connector",
                kind: Some(ShapeKind::Relation),
            },
            Symbol {
                id: "mindmap-structure-1",
                label: "Relationship",
                kind: Some(ShapeKind::Relation),
            },
            Symbol {
                id: "mindmap-structure-2",
                label: "Boundary",
                kind: Some(ShapeKind::Boundary),
            },
            Symbol {
                id: "mindmap-structure-3",
                label: "Summary",
                kind: Some(ShapeKind::Note),
            },
            Symbol {
                id: "mindmap-structure-4",
                label: "Grouping",
                kind: Some(ShapeKind::Boundary),
            },
            Symbol {
                id: "mindmap-structure-5",
                label: "Cross-link",
                kind: Some(ShapeKind::Relation),
            },
        ],
    },
    SymbolGroup {
        id: "mindmap-markers",
        title: "Annotations & markers",
        items: &[
            Symbol {
                id: "mindmap-markers-0",
                label: "Priority marker",
                kind: Some(ShapeKind::State),
            },
            Symbol {
                id: "mindmap-markers-1",
                label: "Progress marker",
                kind: Some(ShapeKind::State),
            },
            Symbol {
                id: "mindmap-markers-2",
                label: "Flag",
                kind: Some(ShapeKind::Note),
            },
            Symbol {
                id: "mindmap-markers-3",
                label: "Note",
                kind: Some(ShapeKind::Note),
            },
            Symbol {
                id: "mindmap-markers-4",
                label: "Link",
                kind: Some(ShapeKind::Relation),
            },
            Symbol {
                id: "mindmap-markers-5",
                label: "Tag",
                kind: Some(ShapeKind::Callout),
            },
            Symbol {
                id: "mindmap-markers-6",
                label: "Image",
                kind: Some(ShapeKind::Document),
            },
        ],
    },
];
