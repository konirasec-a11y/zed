//! Visual vocabulary from the reviewed v3 reference (not a bellatrix-mde-modeling schema).
//! Names, groups and SVG geometry are reusable in any GPUI host.

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Family {
    Uml,
    C4,
    MindMap,
    Flowchart,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ShapeKind {
    FoundMessage,
    LostMessage,
    GeneralOrdering,
    SynchronousMessage,
    ReplyMessage,
    Extension,
    DashedLink,
    AssociationClassAttachment,
    Port,
    DeepHistory,
    Junction,
    InputPin,
    OutputPin,
    InteractionOperand,

    InitialNode,
    FinalNode,
    FlowFinal,
    History,
    Terminate,
    EntryPoint,
    Collaboration,
    Execution,
    ObjectNode,
    SendSignal,
    AcceptEvent,
    Partition,
    CombinedFragment,

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
    ProvidedInterface,
    RequiredInterface,
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

mod uml;
use uml::UML;
mod c4;
use c4::C4;
mod mindmap;
use mindmap::MINDMAP;
mod flowchart;
use flowchart::FLOWCHART;
