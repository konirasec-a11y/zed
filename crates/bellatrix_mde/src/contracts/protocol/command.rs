pub use crate::contracts::{
    AssociationEndConvention, Authored, Geometry, LayoutBounds, LayoutExtent, LineMode,
    NavigationKnowledge, PathSegment, Point, RelationshipRouting, StereotypeDisplay, Style,
    TextAlignment, TextVerticalAlignment, TimingMode, TimingPoint, TimingPresentation, UmlStyle,
    UnresolvedExtentReason, ViewEndpoints, ViewOccurrence, VisualDraft,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelingElementKind {
    Port,
    Part,
    DeploymentSpecification,
    CollaborationUse,
    DestructionOccurrenceSpecification,
    Continuation,
    Class,
    Interface,
    Component,
    Artifact,
    Signal,
    DataType,
    PrimitiveType,
    Enumeration,
    Frame,
    NaryAssociation,
    AssociationClass,
    Package,
    Model,
    Subsystem,
    Comment,
    Template,
    Node,
    Device,
    ExecutionEnvironment,
    Collaboration,
    InstanceSpecification,
    Profile,
    Stereotype,
    Actor,
    UseCase,
    Activity,
    StateMachine,
    Interaction,
    OpaqueAction,
    InitialNode,
    ActivityFinalNode,
    FlowFinalNode,
    DecisionNode,
    MergeNode,
    ForkNode,
    JoinNode,
    CentralBufferNode,
    DataStoreNode,
    SendSignalAction,
    AcceptEventAction,
    ActivityPartition,
    Region,
    State,
    Pseudostate,
    FinalState,
    Lifeline,
    BehaviorExecutionSpecification,
    InteractionUse,
    CombinedFragment,
    StateInvariant,
    InputPin,
    OutputPin,
    ValuePin,
    InteractionOperand,
    Choice,
    Junction,
    StateFork,
    StateJoin,
    ShallowHistory,
    DeepHistory,
    EntryPoint,
    ExitPoint,
    Terminate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelingMemberKind {
    Attribute,
    Operation,
    Parameter,
    Reception,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelingRelationKind {
    ElementImport,
    ProfileApplication,
    Extension,
    Connector,
    NoteLink,
    ConstraintLink,
    MessageSynchronous,
    MessageAsynchronous,
    MessageReply,
    MessageFound,
    MessageLost,
    GeneralOrdering,
    AssociationClassAttachment,
    Association,
    DirectedAssociation,
    Aggregation,
    Composition,
    Dependency,
    Generalization,
    InterfaceRealization,
    TemplateBinding,
    Realization,
    Containment,
    Usage,
    Abstraction,
    Include,
    Extend,
    PackageImport,
    PackageMerge,
    Deployment,
    Manifestation,
    ControlFlow,
    ObjectFlow,
    Transition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructuralDiagramKind {
    Class,
    Object,
    Package,
    Component,
    CompositeStructure,
    Deployment,
    Profile,
    UseCase,
    Activity,
    StateMachine,
    Sequence,
    Communication,
    InteractionOverview,
    Timing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UmlQuickCreate {
    Note,
    Constraint,
    Reception,
    Subtype,
    Supertype,
    RealizingClass,
    TemplateParameter,
    ProvidedInterface,
    RequiredInterface,
    AssociatedClass,
    AggregatedClass,
    CompositedClass,
    Port,
    Part,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ViewOrder {
    BringToFront,
    SendToBack,
    BringForward,
    SendBackward,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelingTool {
    ProvidedInterface,
    RequiredInterface,
    Element(ModelingElementKind),
    Relation(ModelingRelationKind),
}

pub use crate::contracts::uml::schema::Aggregation;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssociationEndOwner {
    Association,
    Classifier(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssociationEndDraft {
    pub name: String,
    pub type_id: String,
    pub owner: AssociationEndOwner,
    pub aggregation: Aggregation,
    pub multiplicity: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemovedAssociationEnd {
    KeepClassifierOwned,
    ReparentToClassifier(String),
    DeleteAssociationOwned,
}

/// Removal always states what happens to the Property identity.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssociationEndEdit {
    Insert {
        position: usize,
        end: AssociationEndDraft,
    },
    Attach {
        position: usize,
        property_id: String,
        owner: Option<AssociationEndOwner>,
    },
    Remove {
        property_id: String,
        disposition: RemovedAssociationEnd,
    },
    Move {
        property_id: String,
        position: usize,
    },
    Reparent {
        property_id: String,
        owner: AssociationEndOwner,
    },
    Retype {
        property_id: String,
        type_id: String,
    },
}

/// Integration intent. Domain validation and mutation belong to the engine.
#[derive(Clone, Debug)]
pub enum Command {
    AdoptRootModel,
    SetOwner {
        element_id: String,
        owner_id: Option<String>,
    },
    SetExplorerParent {
        subject_id: String,
        parent_id: Option<String>,
        parent_occurrence: Option<ViewOccurrence>,
    },
    CreateDiagram {
        kind: StructuralDiagramKind,
        name: String,
        model_elements: Vec<String>,
    },
    RenameDiagram {
        diagram_id: String,
        name: String,
    },
    DeleteDiagram {
        diagram_id: String,
    },
    CreateExplorerDiagram {
        kind: StructuralDiagramKind,
        name: String,
        model_elements: Vec<String>,
        parent_id: Option<String>,
        parent_occurrence: Option<ViewOccurrence>,
    },
    CreateModelElement {
        owner_id: Option<String>,
        kind: ModelingElementKind,
        name: Option<String>,
    },
    CreateElement {
        diagram_id: String,
        owner_id: Option<String>,
        kind: ModelingElementKind,
        name: Option<String>,
        x: f64,
        y: f64,
    },
    CreateElementOrVisualDraft {
        diagram_id: String,
        owner_id: Option<String>,
        kind: ModelingElementKind,
        name: Option<String>,
        x: f64,
        y: f64,
    },
    CreateElementVisualDraft {
        diagram_id: String,
        owner_id: Option<String>,
        kind: ModelingElementKind,
        name: Option<String>,
        x: f64,
        y: f64,
    },
    CreateRelationOrVisualDraft {
        diagram_id: String,
        owner_id: Option<String>,
        kind: ModelingRelationKind,
        from: String,
        to: String,
    },
    CreateRelationAtViewsOrVisualDraft {
        diagram_id: String,
        owner_id: Option<String>,
        kind: ModelingRelationKind,
        source_view_id: String,
        target_view_id: String,
        from: Option<String>,
        to: Option<String>,
    },
    CreateRelationAtViews {
        diagram_id: String,
        owner_id: Option<String>,
        kind: ModelingRelationKind,
        from: String,
        to: String,
        endpoints: ViewEndpoints,
    },
    CreateRelation {
        diagram_id: String,
        owner_id: Option<String>,
        kind: ModelingRelationKind,
        from: String,
        to: String,
    },
    AddMember {
        owner_id: String,
        kind: ModelingMemberKind,
    },
    MoveParameter {
        parameter_id: String,
        up: bool,
    },
    MigrateAssociation {
        association_id: String,
    },
    SetRedefinedElements {
        element_id: String,
        redefined_ids: Vec<String>,
    },
    SetSubsettedProperties {
        property_id: String,
        subsetted_ids: Vec<String>,
    },
    SetNamespaceImport {
        import_id: String,
        namespace_id: String,
        target_id: String,
        is_package: bool,
        alias: Option<String>,
        visibility: String,
    },
    SetAssociationEndNavigation {
        association_id: String,
        property_id: String,
        navigation: NavigationKnowledge,
    },
    SetAssociationMemberEnds {
        association_id: String,
        member_end_ids: Vec<String>,
    },
    EditAssociationEnds {
        association_id: String,
        edits: Vec<AssociationEndEdit>,
    },
    QuickEdit {
        diagram_id: String,
        element_id: String,
        view_id: Option<String>,
        action: UmlQuickCreate,
    },
    CreateVisualDraft {
        diagram_id: String,
        notation_kind: String,
        label: Option<String>,
        geometry: Geometry,
        properties: std::collections::BTreeMap<String, serde_json::Value>,
        parent_view_id: Option<String>,
        source_view_id: Option<String>,
        target_view_id: Option<String>,
    },
    PlaceElementView {
        diagram_id: String,
        element_id: String,
        x: f64,
        y: f64,
    },
    RemoveView {
        diagram_id: String,
        view_id: String,
    },
    ReorderView {
        diagram_id: String,
        view_id: String,
        order: ViewOrder,
    },
    SetViewLabel {
        view_id: String,
        label: Option<String>,
    },
    SetViewTiming {
        view_id: String,
        timing: Option<TimingPresentation>,
    },
    SetViewEndpoints {
        diagram_id: String,
        view_id: String,
        endpoints: ViewEndpoints,
    },
    SetViewStyle {
        diagram_id: String,
        view_id: String,
        style: Style,
    },
    SetViewPath {
        diagram_id: String,
        view_id: String,
        geometry: Geometry,
    },
    SetViewRouting {
        diagram_id: String,
        view_id: String,
        routing: RelationshipRouting,
    },
    MoveView {
        view_id: String,
        dx: f64,
        dy: f64,
    },
    ResizeView {
        view_id: String,
        width: f64,
        height: f64,
    },
    DeleteElement {
        element_id: String,
    },
    SetProperty {
        element_id: String,
        path: String,
        value: serde_json::Value,
    },
    SetNameExpression {
        element_id: String,
        expression: String,
    },
    Rename {
        element_id: String,
        name: String,
    },
}

/// Restricted edits supported by lossless native MDJ interchange.
#[derive(Clone, Debug)]
pub enum MdjEditCommand {
    SetViewPath {
        diagram_id: String,
        view_id: String,
        geometry: Geometry,
    },
    SetViewLabel {
        view_id: String,
        label: Option<String>,
    },
    SetViewStyle {
        diagram_id: String,
        view_id: String,
        style: Style,
    },
    Rename {
        element_id: String,
        name: String,
    },
    RenameDiagram {
        diagram_id: String,
        name: String,
    },
    SetProperty {
        element_id: String,
        path: String,
        value: serde_json::Value,
    },
}

impl From<MdjEditCommand> for Command {
    fn from(value: MdjEditCommand) -> Self {
        match value {
            MdjEditCommand::SetViewPath {
                diagram_id,
                view_id,
                geometry,
            } => Self::SetViewPath {
                diagram_id,
                view_id,
                geometry,
            },
            MdjEditCommand::SetViewLabel { view_id, label } => {
                Self::SetViewLabel { view_id, label }
            }
            MdjEditCommand::SetViewStyle {
                diagram_id,
                view_id,
                style,
            } => Self::SetViewStyle {
                diagram_id,
                view_id,
                style,
            },
            MdjEditCommand::Rename { element_id, name } => Self::Rename { element_id, name },
            MdjEditCommand::RenameDiagram { diagram_id, name } => {
                Self::RenameDiagram { diagram_id, name }
            }
            MdjEditCommand::SetProperty {
                element_id,
                path,
                value,
            } => Self::SetProperty {
                element_id,
                path,
                value,
            },
        }
    }
}
