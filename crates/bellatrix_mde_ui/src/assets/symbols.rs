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

pub fn glyph(kind: ShapeKind) -> &'static [u8] {
    match kind {
        ShapeKind::FoundMessage => include_bytes!("symbols/svg/found-message.svg"),
        ShapeKind::LostMessage => include_bytes!("symbols/svg/lost-message.svg"),
        ShapeKind::GeneralOrdering => include_bytes!("symbols/svg/general-ordering.svg"),
        ShapeKind::InputPin => include_bytes!("symbols/svg/input-pin.svg"),
        ShapeKind::OutputPin => include_bytes!("symbols/svg/output-pin.svg"),
        ShapeKind::InteractionOperand => include_bytes!("symbols/svg/interaction-operand.svg"),

        ShapeKind::InitialNode => include_bytes!("symbols/svg/initial-node.svg"),
        ShapeKind::FinalNode => include_bytes!("symbols/svg/final-node.svg"),
        ShapeKind::FlowFinal => include_bytes!("symbols/svg/flow-final.svg"),
        ShapeKind::DashedLink => include_bytes!("symbols/svg/dashed-link.svg"),
        ShapeKind::Extension => include_bytes!("symbols/svg/extension.svg"),
        ShapeKind::AssociationClassAttachment => {
            include_bytes!("symbols/svg/association-class-attachment.svg")
        }
        ShapeKind::Port => include_bytes!("symbols/svg/port.svg"),
        ShapeKind::DeepHistory => include_bytes!("symbols/svg/deep-history.svg"),
        ShapeKind::Junction => include_bytes!("symbols/svg/junction.svg"),
        ShapeKind::History => include_bytes!("symbols/svg/history.svg"),
        ShapeKind::Terminate => include_bytes!("symbols/svg/terminate.svg"),
        ShapeKind::EntryPoint => include_bytes!("symbols/svg/entry-point.svg"),
        ShapeKind::Collaboration => include_bytes!("symbols/svg/collaboration.svg"),
        ShapeKind::Execution => include_bytes!("symbols/svg/execution.svg"),
        ShapeKind::ObjectNode => include_bytes!("symbols/svg/object-node.svg"),
        ShapeKind::SendSignal => include_bytes!("symbols/svg/send-signal.svg"),
        ShapeKind::AcceptEvent => include_bytes!("symbols/svg/accept-event.svg"),
        ShapeKind::Partition => include_bytes!("symbols/svg/partition.svg"),
        ShapeKind::CombinedFragment => include_bytes!("symbols/svg/combined-fragment.svg"),

        ShapeKind::Activity => include_bytes!("symbols/svg/activity.svg"),
        ShapeKind::Actor => include_bytes!("symbols/svg/actor.svg"),
        ShapeKind::Aggregate => include_bytes!("symbols/svg/aggregate.svg"),
        ShapeKind::Association => include_bytes!("symbols/svg/association.svg"),
        ShapeKind::Boundary => include_bytes!("symbols/svg/boundary.svg"),
        ShapeKind::Branch => include_bytes!("symbols/svg/branch.svg"),
        ShapeKind::Callout => include_bytes!("symbols/svg/callout.svg"),
        ShapeKind::Class => include_bytes!("symbols/svg/class.svg"),
        ShapeKind::Component => include_bytes!("symbols/svg/component.svg"),
        ShapeKind::Composition => include_bytes!("symbols/svg/composition.svg"),
        ShapeKind::Connector => include_bytes!("symbols/svg/connector.svg"),
        ShapeKind::Container => include_bytes!("symbols/svg/container.svg"),
        ShapeKind::Database => include_bytes!("symbols/svg/database.svg"),
        ShapeKind::Decision => include_bytes!("symbols/svg/decision.svg"),
        ShapeKind::Dependency => include_bytes!("symbols/svg/dependency.svg"),
        ShapeKind::Deployment => include_bytes!("symbols/svg/deployment.svg"),
        ShapeKind::Document => include_bytes!("symbols/svg/document.svg"),
        ShapeKind::Enum => include_bytes!("symbols/svg/enum.svg"),
        ShapeKind::External => include_bytes!("symbols/svg/external.svg"),
        ShapeKind::Fork => include_bytes!("symbols/svg/fork.svg"),
        ShapeKind::Inheritance => include_bytes!("symbols/svg/inheritance.svg"),
        ShapeKind::Input => include_bytes!("symbols/svg/input.svg"),
        ShapeKind::Interface => include_bytes!("symbols/svg/interface.svg"),
        ShapeKind::ProvidedInterface => include_bytes!("symbols/svg/provided-interface.svg"),
        ShapeKind::RequiredInterface => include_bytes!("symbols/svg/required-interface.svg"),
        ShapeKind::Lifeline => include_bytes!("symbols/svg/lifeline.svg"),
        ShapeKind::SynchronousMessage => include_bytes!("symbols/svg/synchronous-message.svg"),
        ShapeKind::ReplyMessage => include_bytes!("symbols/svg/reply-message.svg"),
        ShapeKind::Message => include_bytes!("symbols/svg/message.svg"),
        ShapeKind::Note => include_bytes!("symbols/svg/note.svg"),
        ShapeKind::Package => include_bytes!("symbols/svg/package.svg"),
        ShapeKind::Person => include_bytes!("symbols/svg/person.svg"),
        ShapeKind::Predefined => include_bytes!("symbols/svg/predefined.svg"),
        ShapeKind::Process => include_bytes!("symbols/svg/process.svg"),
        ShapeKind::Relation => include_bytes!("symbols/svg/relation.svg"),
        ShapeKind::State => include_bytes!("symbols/svg/state.svg"),
        ShapeKind::System => include_bytes!("symbols/svg/system.svg"),
        ShapeKind::Terminator => include_bytes!("symbols/svg/terminator.svg"),
        ShapeKind::Topic => include_bytes!("symbols/svg/topic.svg"),
        ShapeKind::Usecase => include_bytes!("symbols/svg/usecase.svg"),
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
