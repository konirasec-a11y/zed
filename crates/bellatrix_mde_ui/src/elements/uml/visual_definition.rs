use crate::assets::symbols::{ShapeKind, glyph};
use crate::assets::uml::staruml::{ExplorerIconCategory, explorer_glyph};
use crate::elements::uml::{UmlClassifier, UmlFigure, UmlRelationGlyph};
use bellatrix_mde_cordis::presentation_contract::{FigurePresentation, RelationshipStyle};
use bellatrix_mde_cordis::projection::SemanticIdentity;
use bellatrix_mde_cordis::{ModelingElementKind, ModelingRelationKind, ModelingTool};
use gpui::SharedString;

#[derive(Clone, Copy)]
pub struct UmlVisualDefinition {
    icon: &'static [u8],
    category: ExplorerIconCategory,
}

impl UmlVisualDefinition {
    pub fn for_element(kind: &str, identity: &SemanticIdentity) -> Self {
        Self::for_kind(
            kind,
            identity
                .metaclass
                .as_deref()
                .unwrap_or(&identity.type_label),
            identity.relationship,
        )
    }

    pub fn for_kind(kind: &str, type_label: &str, relationship: bool) -> Self {
        let imported_name = kind.strip_prefix("mdj.").map(|kind| {
            let kind = kind.strip_prefix("UML").unwrap_or(kind);
            kind.strip_suffix("View").unwrap_or(kind)
        });
        let name = match kind {
            "mdj.ERDDataModel" => "Package",
            "mdj.ERDEntity" => "Class",
            "mdj.ERDColumn" => "Attribute",
            "mdj.ERDRelationship" | "mdj.ERDRelationshipEnd" => "Association",
            "uml.attribute" | "uml.property" | "rust.field" => "Attribute",
            "uml.part" => "Part",
            "depends_on" => "Dependency",
            "uml.composition" => "Composition",
            "uml.aggregation" => "Aggregation",
            "uml.nary_association" => "NaryAssociationNode",
            "uml.instance_specification" => "Object",
            "uml.comment" => "Note",
            "uml.opaque_action" | "uml.call_behavior_action" | "uml.call_operation_action" => {
                "Action"
            }
            "uml.final_node" => "ActivityFinalNode",
            "uml.initial_node" => "InitialNode",
            "uml.decision_node" => "DecisionNode",
            "uml.merge_node" => "MergeNode",
            "uml.fork_node" => "ForkNode",
            "uml.join_node" => "JoinNode",
            "uml.message" => "Message",
            "rust.module" => "Package",
            "rust.struct" => "Class",
            "rust.enum" => "Enumeration",
            "rust.trait" => "Interface",
            "rust.function" | "rust.method" => "Operation",
            _ => imported_name.unwrap_or(type_label),
        };
        // These are presentation aliases only; opaque source objects keep their
        // imported identity and never gain canonical UML editing capabilities.
        let name = match name {
            "Label" | "NodeLabel" | "EdgeLabel" => "Text",
            "RoundRect" => "RoundedRectangle",
            _ => name,
        };
        let (data, category) = explorer_glyph(name).unwrap_or_else(|| {
            let category = if relationship {
                ExplorerIconCategory::Relation
            } else {
                ExplorerIconCategory::Element
            };
            (
                crate::assets::symbols::glyph(element_glyph(kind, name)),
                category,
            )
        });
        Self {
            icon: data,
            category,
        }
    }

    pub fn for_tool(tool: ModelingTool) -> Self {
        let name = match tool {
            ModelingTool::ProvidedInterface => "ProvidedInterface",
            ModelingTool::RequiredInterface => "RequiredInterface",
            ModelingTool::Element(kind) => match kind {
                ModelingElementKind::Port => "Port",
                ModelingElementKind::Part => "Part",
                ModelingElementKind::DeploymentSpecification => "DeploymentSpecification",
                ModelingElementKind::CollaborationUse => "CollaborationUse",
                ModelingElementKind::DestructionOccurrenceSpecification => {
                    "DestructionOccurrenceSpecification"
                }
                ModelingElementKind::Continuation => "Continuation",
                ModelingElementKind::Choice => "Choice",
                ModelingElementKind::Junction => "Junction",
                ModelingElementKind::StateFork => "StateFork",
                ModelingElementKind::StateJoin => "StateJoin",
                ModelingElementKind::ShallowHistory => "ShallowHistory",
                ModelingElementKind::DeepHistory => "DeepHistory",
                ModelingElementKind::EntryPoint => "EntryPoint",
                ModelingElementKind::ExitPoint => "ExitPoint",
                ModelingElementKind::Terminate => "Terminate",
                ModelingElementKind::InputPin => "InputPin",
                ModelingElementKind::ValuePin => "ValuePin",
                ModelingElementKind::OutputPin => "OutputPin",
                ModelingElementKind::InteractionOperand => "InteractionOperand",
                ModelingElementKind::Class => "Class",
                ModelingElementKind::Template => "Template",
                ModelingElementKind::AssociationClass => "AssociationClass",
                ModelingElementKind::Interface => "Interface",
                ModelingElementKind::Component => "Component",
                ModelingElementKind::Artifact => "Artifact",
                ModelingElementKind::Enumeration => "Enumeration",
                ModelingElementKind::Package => "Package",
                ModelingElementKind::Model => "Model",
                ModelingElementKind::Subsystem => "Subsystem",
                ModelingElementKind::Node => "Node",
                ModelingElementKind::Device => "Device",
                ModelingElementKind::ExecutionEnvironment => "ExecutionEnvironment",
                ModelingElementKind::Collaboration => "Collaboration",
                ModelingElementKind::UseCase => "UseCase",
                ModelingElementKind::InstanceSpecification => "Object",
                ModelingElementKind::Stereotype => "Stereotype",
                ModelingElementKind::Profile => "Profile",
                ModelingElementKind::Actor => "Actor",
                ModelingElementKind::Activity => "Activity",
                ModelingElementKind::StateMachine => "StateMachine",
                ModelingElementKind::Interaction => "Interaction",
                ModelingElementKind::InteractionUse => "InteractionUse",
                ModelingElementKind::CombinedFragment => "CombinedFragment",
                ModelingElementKind::Region => "Region",
                ModelingElementKind::ActivityPartition => "ActivityPartition",
                ModelingElementKind::OpaqueAction => "Action",
                ModelingElementKind::SendSignalAction => "SendSignalAction",
                ModelingElementKind::AcceptEventAction => "AcceptEventAction",
                ModelingElementKind::InitialNode => "InitialNode",
                ModelingElementKind::Pseudostate => "Pseudostate",
                ModelingElementKind::ActivityFinalNode => "ActivityFinalNode",
                ModelingElementKind::FinalState => "FinalState",
                ModelingElementKind::FlowFinalNode => "FlowFinalNode",
                ModelingElementKind::State => "State",
                ModelingElementKind::StateInvariant => "StateInvariant",
                ModelingElementKind::DecisionNode => "DecisionNode",
                ModelingElementKind::MergeNode => "MergeNode",
                ModelingElementKind::ForkNode => "ForkNode",
                ModelingElementKind::JoinNode => "JoinNode",
                ModelingElementKind::Lifeline => "Lifeline",
                ModelingElementKind::BehaviorExecutionSpecification => {
                    "BehaviorExecutionSpecification"
                }
                ModelingElementKind::CentralBufferNode => "CentralBufferNode",
                ModelingElementKind::DataStoreNode => "DataStoreNode",
                ModelingElementKind::Comment => "Note",
                ModelingElementKind::Frame => "Frame",
                ModelingElementKind::NaryAssociation => "NaryAssociationNode",
                ModelingElementKind::Signal => "Signal",
                ModelingElementKind::DataType => "DataType",
                ModelingElementKind::PrimitiveType => "PrimitiveType",
            },
            ModelingTool::Relation(kind) => match kind {
                ModelingRelationKind::MessageSynchronous => "Message",
                ModelingRelationKind::MessageAsynchronous => "Message",
                ModelingRelationKind::MessageReply => "Message",
                ModelingRelationKind::MessageFound => "Message",
                ModelingRelationKind::MessageLost => "Message",
                ModelingRelationKind::GeneralOrdering => "GeneralOrdering",
                ModelingRelationKind::Extension => "Extension",
                ModelingRelationKind::Connector => "Connector",
                ModelingRelationKind::NoteLink => "NoteLink",
                ModelingRelationKind::ConstraintLink => "ConstraintLink",
                ModelingRelationKind::ElementImport => "ElementImport",
                ModelingRelationKind::ProfileApplication => "ProfileApplication",
                ModelingRelationKind::AssociationClassAttachment => "AssociationClassAttachment",
                ModelingRelationKind::Usage => "Usage",
                ModelingRelationKind::Abstraction => "Abstraction",
                ModelingRelationKind::Include => "Include",
                ModelingRelationKind::Extend => "Extend",
                ModelingRelationKind::PackageImport => "PackageImport",
                ModelingRelationKind::PackageMerge => "PackageMerge",
                ModelingRelationKind::Deployment => "Deployment",
                ModelingRelationKind::Manifestation => "Manifestation",
                ModelingRelationKind::ControlFlow => "ControlFlow",
                ModelingRelationKind::ObjectFlow => "ObjectFlow",
                ModelingRelationKind::Transition => "Transition",
                ModelingRelationKind::Association => "Association",
                ModelingRelationKind::DirectedAssociation => "DirectedAssociation",
                ModelingRelationKind::Containment => "Containment",
                ModelingRelationKind::Aggregation => "Aggregation",
                ModelingRelationKind::Composition => "Composition",
                ModelingRelationKind::Generalization => "Generalization",
                ModelingRelationKind::InterfaceRealization => "InterfaceRealization",
                ModelingRelationKind::Dependency => "Dependency",
                ModelingRelationKind::Realization => "Realization",
                ModelingRelationKind::TemplateBinding => "TemplateBinding",
            },
        };
        let (icon, category) = explorer_glyph(name).unwrap_or_else(|| {
            (
                glyph(fallback_tool_glyph(tool)),
                if matches!(tool, ModelingTool::Relation(_)) {
                    ExplorerIconCategory::Relation
                } else {
                    ExplorerIconCategory::Element
                },
            )
        });
        Self { icon, category }
    }

    pub fn for_quick_action(id: &str) -> Option<Self> {
        let name = match id {
            "uml.add-attribute" => "Attribute",
            "uml.add-operation" => "Operation",
            "uml.add-reception" => "Reception",
            "uml.add-note" => "Note",
            "uml.add-constraint" => "Constraint",
            "uml.add-subinterface" | "uml.add-superinterface" => "Interface",
            "uml.add-subclass" | "uml.add-superclass" | "uml.add-realizing-class" => "Class",
            "uml.add-template-parameter" => "TemplateParameter",
            "uml.add-provided-interface" => {
                return Some(Self::for_tool(ModelingTool::ProvidedInterface));
            }
            "uml.add-required-interface" => {
                return Some(Self::for_tool(ModelingTool::RequiredInterface));
            }
            "uml.add-associated-class" => "Association",
            "uml.add-aggregated-class" => "Aggregation",
            "uml.add-composited-class" => "Composition",
            "uml.add-port" => "Port",
            "uml.add-part" => "Part",
            _ => return None,
        };
        Some(Self::for_kind("", name, false))
    }

    pub fn uses_figure(self, presentation: &FigurePresentation) -> bool {
        UmlFigure::supports(presentation)
    }

    pub fn icon(self) -> &'static [u8] {
        self.icon
    }
    pub fn category(self) -> ExplorerIconCategory {
        self.category
    }

    // The projection, not the semantic icon, decides occurrence-specific forms
    // such as an Interface rectangle versus a provided/required lollipop.
    pub fn figure(
        self,
        presentation: FigurePresentation,
        name: impl Into<SharedString>,
    ) -> UmlFigure {
        UmlFigure::new(presentation, name)
    }
    pub fn classifier(self, name: impl Into<SharedString>) -> UmlClassifier {
        UmlClassifier::new(name)
    }
    pub fn relationship(
        self,
        style: RelationshipStyle,
        width: f32,
        height: f32,
        from: (f32, f32),
        to: (f32, f32),
    ) -> UmlRelationGlyph {
        UmlRelationGlyph::new(style, width, height, from, to)
    }
}

fn element_glyph(kind: &str, type_label: &str) -> crate::assets::symbols::ShapeKind {
    use crate::assets::symbols::ShapeKind;
    // Member and relationship symbols are metaclass based, never inferred from names.
    match kind {
        "uml.composition" => return ShapeKind::Composition,
        "uml.aggregation" => return ShapeKind::Aggregate,
        _ => {}
    }
    match type_label {
        "Attribute" | "Property" => return ShapeKind::Aggregate,
        "Operation" => return ShapeKind::Process,
        "Reception" => return ShapeKind::AcceptEvent,
        "Parameter" => return ShapeKind::Input,
        "Constraint" => return ShapeKind::Note,
        "Generalization" => return ShapeKind::Inheritance,
        "Frame" => return ShapeKind::Boundary,
        "Rectangle" => return ShapeKind::Process,
        "RoundRect" | "RoundedRectangle" => return ShapeKind::Topic,
        "Ellipse" => return ShapeKind::Usecase,
        _ => {}
    }
    let label = if kind == "rust.module" {
        "Package"
    } else {
        type_label
    };
    crate::assets::symbols::groups(crate::assets::symbols::Family::Uml)
        .iter()
        .flat_map(|group| group.items)
        .find(|symbol| symbol.label.eq_ignore_ascii_case(label))
        .map(|symbol| symbol.kind)
        .unwrap_or(ShapeKind::Document)
}

fn fallback_tool_glyph(tool: ModelingTool) -> ShapeKind {
    match tool {
        ModelingTool::ProvidedInterface => ShapeKind::ProvidedInterface,
        ModelingTool::RequiredInterface => ShapeKind::RequiredInterface,
        ModelingTool::Element(kind) => match kind {
            ModelingElementKind::Port => ShapeKind::Port,
            ModelingElementKind::Part => ShapeKind::Class,
            ModelingElementKind::DeploymentSpecification => ShapeKind::Document,
            ModelingElementKind::CollaborationUse => ShapeKind::Collaboration,
            ModelingElementKind::DestructionOccurrenceSpecification => ShapeKind::Terminate,
            ModelingElementKind::Continuation => ShapeKind::State,

            ModelingElementKind::Choice => ShapeKind::Decision,
            ModelingElementKind::Junction => ShapeKind::Junction,
            ModelingElementKind::StateFork | ModelingElementKind::StateJoin => ShapeKind::Fork,
            ModelingElementKind::ShallowHistory => ShapeKind::History,
            ModelingElementKind::DeepHistory => ShapeKind::DeepHistory,
            ModelingElementKind::EntryPoint => ShapeKind::EntryPoint,
            ModelingElementKind::ExitPoint => ShapeKind::FlowFinal,
            ModelingElementKind::Terminate => ShapeKind::Terminate,
            ModelingElementKind::InputPin | ModelingElementKind::ValuePin => ShapeKind::InputPin,
            ModelingElementKind::OutputPin => ShapeKind::OutputPin,
            ModelingElementKind::InteractionOperand => ShapeKind::InteractionOperand,
            ModelingElementKind::Class
            | ModelingElementKind::Template
            | ModelingElementKind::AssociationClass => ShapeKind::Class,
            ModelingElementKind::Interface => ShapeKind::Interface,
            ModelingElementKind::Component => ShapeKind::Component,
            ModelingElementKind::Artifact => ShapeKind::Document,
            ModelingElementKind::Enumeration => ShapeKind::Enum,
            ModelingElementKind::Package
            | ModelingElementKind::Model
            | ModelingElementKind::Subsystem => ShapeKind::Package,
            ModelingElementKind::Node
            | ModelingElementKind::Device
            | ModelingElementKind::ExecutionEnvironment => ShapeKind::Deployment,
            ModelingElementKind::Collaboration => ShapeKind::Collaboration,
            ModelingElementKind::UseCase => ShapeKind::Usecase,
            ModelingElementKind::InstanceSpecification | ModelingElementKind::Stereotype => {
                ShapeKind::Class
            }
            ModelingElementKind::Profile => ShapeKind::Package,
            ModelingElementKind::Actor => ShapeKind::Actor,
            ModelingElementKind::Activity => ShapeKind::Activity,
            ModelingElementKind::StateMachine => ShapeKind::State,
            ModelingElementKind::Interaction | ModelingElementKind::InteractionUse => {
                ShapeKind::Boundary
            }
            ModelingElementKind::CombinedFragment => ShapeKind::CombinedFragment,
            ModelingElementKind::Region | ModelingElementKind::ActivityPartition => {
                ShapeKind::Partition
            }
            ModelingElementKind::OpaqueAction => ShapeKind::Activity,
            ModelingElementKind::SendSignalAction => ShapeKind::SendSignal,
            ModelingElementKind::AcceptEventAction => ShapeKind::AcceptEvent,
            ModelingElementKind::InitialNode | ModelingElementKind::Pseudostate => {
                ShapeKind::InitialNode
            }
            ModelingElementKind::ActivityFinalNode | ModelingElementKind::FinalState => {
                ShapeKind::FinalNode
            }
            ModelingElementKind::FlowFinalNode => ShapeKind::FlowFinal,
            ModelingElementKind::State | ModelingElementKind::StateInvariant => ShapeKind::State,
            ModelingElementKind::DecisionNode | ModelingElementKind::MergeNode => {
                ShapeKind::Decision
            }
            ModelingElementKind::ForkNode | ModelingElementKind::JoinNode => ShapeKind::Fork,
            ModelingElementKind::Lifeline => ShapeKind::Lifeline,
            ModelingElementKind::BehaviorExecutionSpecification => ShapeKind::Execution,
            ModelingElementKind::CentralBufferNode | ModelingElementKind::DataStoreNode => {
                ShapeKind::ObjectNode
            }
            ModelingElementKind::Comment => ShapeKind::Note,
            ModelingElementKind::Frame => ShapeKind::Boundary,
            ModelingElementKind::NaryAssociation => ShapeKind::Aggregate,
            ModelingElementKind::Signal
            | ModelingElementKind::DataType
            | ModelingElementKind::PrimitiveType => ShapeKind::Class,
        },
        ModelingTool::Relation(kind) => match kind {
            ModelingRelationKind::MessageSynchronous => ShapeKind::SynchronousMessage,
            ModelingRelationKind::MessageAsynchronous => ShapeKind::Message,
            ModelingRelationKind::MessageReply => ShapeKind::ReplyMessage,
            ModelingRelationKind::MessageFound => ShapeKind::FoundMessage,
            ModelingRelationKind::MessageLost => ShapeKind::LostMessage,
            ModelingRelationKind::GeneralOrdering => ShapeKind::GeneralOrdering,

            ModelingRelationKind::Extension => ShapeKind::Extension,
            ModelingRelationKind::Connector => ShapeKind::Association,
            ModelingRelationKind::NoteLink | ModelingRelationKind::ConstraintLink => {
                ShapeKind::DashedLink
            }
            ModelingRelationKind::ElementImport | ModelingRelationKind::ProfileApplication => {
                ShapeKind::Dependency
            }
            ModelingRelationKind::AssociationClassAttachment => {
                ShapeKind::AssociationClassAttachment
            }
            ModelingRelationKind::Usage
            | ModelingRelationKind::Abstraction
            | ModelingRelationKind::Include
            | ModelingRelationKind::Extend
            | ModelingRelationKind::PackageImport
            | ModelingRelationKind::PackageMerge
            | ModelingRelationKind::Deployment
            | ModelingRelationKind::Manifestation => ShapeKind::Dependency,
            ModelingRelationKind::ControlFlow
            | ModelingRelationKind::ObjectFlow
            | ModelingRelationKind::Transition => ShapeKind::Relation,
            ModelingRelationKind::Association
            | ModelingRelationKind::DirectedAssociation
            | ModelingRelationKind::Containment => ShapeKind::Association,
            ModelingRelationKind::Aggregation => ShapeKind::Aggregate,
            ModelingRelationKind::Composition => ShapeKind::Composition,
            ModelingRelationKind::Generalization | ModelingRelationKind::InterfaceRealization => {
                ShapeKind::Inheritance
            }
            ModelingRelationKind::Dependency
            | ModelingRelationKind::Realization
            | ModelingRelationKind::TemplateBinding => ShapeKind::Dependency,
        },
    }
}
