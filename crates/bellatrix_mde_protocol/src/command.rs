//! Independent command transport data. No domain types, validators or model mutations
//! are exposed here. Cordis handlers translate this boundary into MDE commands.
use serde::{Deserialize, Serialize};

// Missing and explicit null carry different authored intent; layout must not
// replace either with measured coordinates when saving a document.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Authored<T> {
    #[default]
    Missing,
    Null,
    Value(T),
}

impl<T> Authored<T> {
    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Value(value) => Some(value),
            _ => None,
        }
    }
}

impl<T: Serialize> Serialize for Authored<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Value(value) => {
                let value = serde_json::to_value(value).map_err(serde::ser::Error::custom)?;
                if value.is_null() {
                    return Err(serde::ser::Error::custom(
                        "authored values must not encode as null; use Authored::Null explicitly",
                    ));
                }
                value.serialize(serializer)
            }
            Self::Null => serializer.serialize_none(),
            Self::Missing => Err(serde::ser::Error::custom(
                "missing authored fields must be omitted",
            )),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Authored<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Option::<T>::deserialize(deserializer)? {
            Some(value) => Self::Value(value),
            None => Self::Null,
        })
    }
}

// Internally tagged enums buffer numbers through Serde's Content. With
// arbitrary_precision enabled for lossless MDJ data, decimal numbers arrive
// there as serde_json's private number map, which f64 cannot read directly.
fn deserialize_coordinate<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<f64, D::Error> {
    let number = serde_json::Number::deserialize(deserializer)?;
    coordinate_from_number(number)
}

fn coordinate_from_number<E: serde::de::Error>(number: serde_json::Number) -> Result<f64, E> {
    number
        .as_f64()
        .filter(|value| value.is_finite())
        .ok_or_else(|| E::custom("geometry coordinates must be finite numbers"))
}

fn deserialize_authored_coordinate<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Authored<f64>, D::Error> {
    match Option::<serde_json::Number>::deserialize(deserializer)? {
        Some(number) => coordinate_from_number(number).map(Authored::Value),
        None => Ok(Authored::Null),
    }
}

/// An authored extent is not necessarily a usable rendered size. Imported
/// layout calculations can retain negative results until layout is resolved.
#[derive(Clone, Debug, PartialEq)]
pub enum LayoutExtent {
    Resolved(f64),
    Unresolved {
        original: f64,
        reason: UnresolvedExtentReason,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnresolvedExtentReason {
    NegativeExtent,
}

impl LayoutExtent {
    pub fn original(&self) -> f64 {
        match self {
            Self::Resolved(value) => *value,
            Self::Unresolved { original, .. } => *original,
        }
    }
    pub fn resolved(&self) -> Option<f64> {
        match self {
            Self::Resolved(value) if value.is_finite() && *value >= 0.0 => Some(*value),
            _ => None,
        }
    }
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Resolved(value) => value.is_finite() && *value >= 0.0,
            Self::Unresolved {
                original,
                reason: UnresolvedExtentReason::NegativeExtent,
            } => original.is_finite() && *original < 0.0,
        }
    }
}

impl Serialize for LayoutExtent {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if !self.is_valid() {
            return Err(serde::ser::Error::custom("invalid authored layout extent"));
        }
        match self {
            Self::Resolved(value) => value.serialize(serializer),
            Self::Unresolved { original, reason } => {
                #[derive(Serialize)]
                struct Pending<'a> {
                    original: f64,
                    reason: &'a UnresolvedExtentReason,
                }
                Pending {
                    original: *original,
                    reason,
                }
                .serialize(serializer)
            }
        }
    }
}

impl<'de> Deserialize<'de> for LayoutExtent {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Pending {
            #[serde(deserialize_with = "deserialize_coordinate")]
            original: f64,
            reason: UnresolvedExtentReason,
        }
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Encoded {
            Resolved(serde_json::Number),
            Unresolved(Pending),
        }
        let extent = match Encoded::deserialize(deserializer)? {
            Encoded::Resolved(number) => Self::Resolved(coordinate_from_number(number)?),
            Encoded::Unresolved(value) => Self::Unresolved {
                original: value.original,
                reason: value.reason,
            },
        };
        if !extent.is_valid() {
            return Err(serde::de::Error::custom("invalid authored layout extent"));
        }
        Ok(extent)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutBounds {
    #[serde(
        default,
        skip_serializing_if = "Authored::is_missing",
        deserialize_with = "deserialize_authored_coordinate"
    )]
    pub x: Authored<f64>,
    #[serde(
        default,
        skip_serializing_if = "Authored::is_missing",
        deserialize_with = "deserialize_authored_coordinate"
    )]
    pub y: Authored<f64>,
    #[serde(default, skip_serializing_if = "Authored::is_missing")]
    pub width: Authored<LayoutExtent>,
    #[serde(default, skip_serializing_if = "Authored::is_missing")]
    pub height: Authored<LayoutExtent>,
}

// These are authored display events, not evaluated UML time or value specifications.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimingPresentation {
    pub mode: TimingMode,
    #[serde(default)]
    pub points: Vec<TimingPoint>,
}

impl TimingPresentation {
    pub fn validate(&self) -> Result<(), String> {
        if self.points.iter().any(|point| !point.time.is_finite()) {
            return Err("timing event times must be finite".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimingMode {
    State,
    Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimingPoint {
    #[serde(
        serialize_with = "serialize_timing_time",
        deserialize_with = "deserialize_timing_time"
    )]
    pub time: f64,
    pub value: String,
}

fn serialize_timing_time<S: serde::Serializer>(
    time: &f64,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if !time.is_finite() {
        return Err(serde::ser::Error::custom(
            "timing event times must be finite",
        ));
    }
    serializer.serialize_f64(*time)
}

fn deserialize_timing_time<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<f64, D::Error> {
    let time = f64::deserialize(deserializer)?;
    if !time.is_finite() {
        return Err(serde::de::Error::custom(
            "timing event times must be finite",
        ));
    }
    Ok(time)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisualDraft {
    pub notation_kind: String,
    #[serde(default)]
    pub properties: std::collections::BTreeMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_view_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Geometry {
    Layout {
        #[serde(default)]
        bounds: LayoutBounds,
    },
    Rect {
        #[serde(deserialize_with = "deserialize_coordinate")]
        x: f64,
        #[serde(deserialize_with = "deserialize_coordinate")]
        y: f64,
        #[serde(deserialize_with = "deserialize_coordinate")]
        width: f64,
        #[serde(deserialize_with = "deserialize_coordinate")]
        height: f64,
    },
    Ellipse {
        #[serde(deserialize_with = "deserialize_coordinate")]
        cx: f64,
        #[serde(deserialize_with = "deserialize_coordinate")]
        cy: f64,
        #[serde(deserialize_with = "deserialize_coordinate")]
        rx: f64,
        #[serde(deserialize_with = "deserialize_coordinate")]
        ry: f64,
    },
    Polygon {
        points: Vec<Point>,
    },
    Path {
        start: Point,
        segments: Vec<PathSegment>,
    },
}

impl Default for Geometry {
    fn default() -> Self {
        Self::Layout {
            bounds: LayoutBounds::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    #[serde(deserialize_with = "deserialize_coordinate")]
    pub x: f64,
    #[serde(deserialize_with = "deserialize_coordinate")]
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PathSegment {
    Line {
        to: Point,
    },
    Cubic {
        control1: Point,
        control2: Point,
        to: Point,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipRouting {
    Manual,
    Straight,
    Automatic,
    Curve,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineMode {
    Solid,
    Dotted,
    Dashed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextVerticalAlignment {
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Style {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routing: Option<RelationshipRouting>,
    #[serde(default)]
    pub fill: Option<String>,
    #[serde(default)]
    pub stroke: Option<String>,
    #[serde(default)]
    pub stroke_width: Option<f64>,
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_italic: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_underline: Option<bool>,
    #[serde(default)]
    pub opacity: Option<f64>,
    #[serde(default)]
    pub text_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_align: Option<TextAlignment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_vertical_align: Option<TextVerticalAlignment>,
    #[serde(default)]
    pub corner_radius: Option<f64>,
    #[serde(default)]
    pub dash_pattern: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_mode: Option<LineMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub word_wrap: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_shadow: Option<bool>,
    #[serde(default, skip_serializing_if = "UmlStyle::is_empty")]
    pub uml: UmlStyle,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewOccurrence {
    pub diagram_id: String,
    pub view_id: String,
    pub member_path: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewEndpoints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_view_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_view_id: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StereotypeDisplay {
    #[serde(rename = "none")]
    Hidden,
    Label,
    Icon,
    IconLabel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssociationEndConvention {
    Uml,
    StarUml,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UmlStyle {
    /// Selects end-marker placement only; never changes Property aggregation
    /// or treats an unspecified navigation declaration as false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub association_end_convention: Option<AssociationEndConvention>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stereotype_display: Option<StereotypeDisplay>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suppress_attributes: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suppress_operations: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suppress_receptions: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_visibility: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_type: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_property: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_multiplicity: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_operation_signature: Option<bool>,
}

impl UmlStyle {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    pub fn should_show_visibility(&self) -> bool {
        self.show_visibility != Some(false)
    }
    pub fn should_show_type(&self) -> bool {
        self.show_type != Some(false)
    }
    pub fn should_show_property(&self) -> bool {
        self.show_property != Some(false)
    }
    pub fn should_show_stereotype(&self) -> bool {
        !matches!(
            self.stereotype_display,
            Some(StereotypeDisplay::Hidden | StereotypeDisplay::Icon)
        )
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Aggregation {
    None,
    Shared,
    Composite,
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationKnowledge {
    Known(bool),
    Unverified,
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
