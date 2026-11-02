//! Visual vocabulary from the reviewed v3 reference (not a bellatrix-mde-modeling schema).
//! Names, groups and SVG geometry are reusable in any GPUI host.
pub mod gallery;
pub use gallery::SymbolGallery;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Family {
    Uml,
    C4,
    MindMap,
    Flowchart,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ShapeKind {
    Activity,
    Actor,
    Aggregate,
    Association,
    Boundary,
    Branch,
    Callout,
    Class,
    Component,
    Composition,
    Connector,
    Container,
    Database,
    Decision,
    Dependency,
    Deployment,
    Document,
    Enum,
    External,
    Fork,
    Inheritance,
    Input,
    Interface,
    Lifeline,
    Message,
    Note,
    Package,
    Person,
    Predefined,
    Process,
    Relation,
    State,
    System,
    Terminator,
    Topic,
    Usecase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Symbol {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: ShapeKind,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SymbolGroup {
    pub id: &'static str,
    pub title: &'static str,
    pub items: &'static [Symbol],
}

pub fn groups(family: Family) -> &'static [SymbolGroup] {
    match family {
        Family::Uml => UML,
        Family::C4 => C4,
        Family::MindMap => MINDMAP,
        Family::Flowchart => FLOWCHART,
    }
}

pub fn count(family: Family) -> usize {
    groups(family).iter().map(|group| group.items.len()).sum()
}

pub fn glyph(kind: ShapeKind) -> &'static [u8] {
    match kind {
        ShapeKind::Activity => include_bytes!("../assets/activity.svg"),
        ShapeKind::Actor => include_bytes!("../assets/actor.svg"),
        ShapeKind::Aggregate => include_bytes!("../assets/aggregate.svg"),
        ShapeKind::Association => include_bytes!("../assets/association.svg"),
        ShapeKind::Boundary => include_bytes!("../assets/boundary.svg"),
        ShapeKind::Branch => include_bytes!("../assets/branch.svg"),
        ShapeKind::Callout => include_bytes!("../assets/callout.svg"),
        ShapeKind::Class => include_bytes!("../assets/class.svg"),
        ShapeKind::Component => include_bytes!("../assets/component.svg"),
        ShapeKind::Composition => include_bytes!("../assets/composition.svg"),
        ShapeKind::Connector => include_bytes!("../assets/connector.svg"),
        ShapeKind::Container => include_bytes!("../assets/container.svg"),
        ShapeKind::Database => include_bytes!("../assets/database.svg"),
        ShapeKind::Decision => include_bytes!("../assets/decision.svg"),
        ShapeKind::Dependency => include_bytes!("../assets/dependency.svg"),
        ShapeKind::Deployment => include_bytes!("../assets/deployment.svg"),
        ShapeKind::Document => include_bytes!("../assets/document.svg"),
        ShapeKind::Enum => include_bytes!("../assets/enum.svg"),
        ShapeKind::External => include_bytes!("../assets/external.svg"),
        ShapeKind::Fork => include_bytes!("../assets/fork.svg"),
        ShapeKind::Inheritance => include_bytes!("../assets/inheritance.svg"),
        ShapeKind::Input => include_bytes!("../assets/input.svg"),
        ShapeKind::Interface => include_bytes!("../assets/interface.svg"),
        ShapeKind::Lifeline => include_bytes!("../assets/lifeline.svg"),
        ShapeKind::Message => include_bytes!("../assets/message.svg"),
        ShapeKind::Note => include_bytes!("../assets/note.svg"),
        ShapeKind::Package => include_bytes!("../assets/package.svg"),
        ShapeKind::Person => include_bytes!("../assets/person.svg"),
        ShapeKind::Predefined => include_bytes!("../assets/predefined.svg"),
        ShapeKind::Process => include_bytes!("../assets/process.svg"),
        ShapeKind::Relation => include_bytes!("../assets/relation.svg"),
        ShapeKind::State => include_bytes!("../assets/state.svg"),
        ShapeKind::System => include_bytes!("../assets/system.svg"),
        ShapeKind::Terminator => include_bytes!("../assets/terminator.svg"),
        ShapeKind::Topic => include_bytes!("../assets/topic.svg"),
        ShapeKind::Usecase => include_bytes!("../assets/usecase.svg"),
    }
}

mod uml;
use uml::UML;
mod c4;
use c4::C4;
mod mindmap;
use mindmap::MINDMAP;
mod flowchart;
use flowchart::FLOWCHART;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    #[test]
    fn all_catalog_entries_match_reviewed_design() {
        for (family, expected) in [
            (Family::Uml, 102),
            (Family::C4, 31),
            (Family::MindMap, 19),
            (Family::Flowchart, 28),
        ] {
            assert_eq!(count(family), expected);
            let mut ids = HashSet::new();
            for group in groups(family) {
                assert!(!group.title.is_empty());
                for symbol in group.items {
                    assert!(ids.insert(symbol.id));
                    assert!(!glyph(symbol.kind).is_empty());
                }
            }
        }
    }
}
