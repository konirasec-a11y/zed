use crate::contracts::protocol::command::{
    LineMode, TextAlignment, TextVerticalAlignment, TimingPresentation,
};

/// Read-only paint inputs. They cannot be applied back to a model or persisted as its state.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderStyle {
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub stroke_width: Option<f64>,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub font_weight: Option<u16>,
    pub font_italic: Option<bool>,
    pub font_underline: Option<bool>,
    pub opacity: Option<f64>,
    pub text_color: Option<String>,
    pub text_align: Option<TextAlignment>,
    pub text_vertical_align: Option<TextVerticalAlignment>,
    pub corner_radius: Option<f64>,
    /// None follows the glyph's stroke; Some(empty) explicitly selects a solid line.
    pub stroke_pattern: Option<Vec<f64>>,
    pub word_wrap: Option<bool>,
    pub show_shadow: Option<bool>,
    pub shadow: Option<RenderShadow>,
    pub show_stereotype: bool,
    pub show_attributes: bool,
    pub show_operations: bool,
    pub show_receptions: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderShadow {
    pub offset_x: f64,
    pub offset_y: f64,
    pub blur_radius: f64,
    pub color: String,
}

impl Default for RenderStyle {
    fn default() -> Self {
        Self {
            fill: None,
            stroke: None,
            stroke_width: None,
            font_family: None,
            font_size: None,
            font_weight: None,
            font_italic: None,
            font_underline: None,
            opacity: None,
            text_color: None,
            text_align: None,
            text_vertical_align: None,
            corner_radius: None,
            stroke_pattern: None,
            word_wrap: None,
            show_shadow: None,
            shadow: None,
            show_stereotype: true,
            show_attributes: true,
            show_operations: true,
            show_receptions: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UmlHeader {
    pub keyword: Option<String>,
    pub applied_stereotype: Option<String>,
    pub classifier_header_height: f64,
}

impl Default for UmlHeader {
    fn default() -> Self {
        Self {
            keyword: None,
            applied_stereotype: None,
            classifier_header_height: 34.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClassifierPresentation {
    pub name_is_italic: bool,
    pub double_sided: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemberPresentation {
    pub underlined: bool,
}

/// A renderer strategy, not a UML metaclass or an editable semantic object.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UmlNotation {
    Rectangle,
    Label,
    Compartment,
    ClassifierRectangle,
    PackageTab,
    FoldedNote,
    Frame,
    AssociationDiamond,
    Port,
    Part,
    Actor,
    UseCase,
    DeploymentNode,
    Collaboration,
    Instance,
    Action,
    State,
    InitialNode,
    FinalNode,
    FlowFinal,
    Decision,
    Fork,
    Lifeline,
    Execution,
    InteractionUse,
    CombinedFragment,
    InteractionOperand,
    ObjectNode,
    SendSignal,
    AcceptEvent,
    Partition,
    TimingState,
    History,
    Terminate,
    #[default]
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterfacePresentation {
    Provided,
    Required,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FigurePresentation {
    pub notation: UmlNotation,
    pub header: UmlHeader,
    pub frame_kind: Option<String>,
    pub title_underlined: bool,
    pub fixed_message_anchor_x: Option<f64>,
    pub resizable: bool,
    pub interface: Option<InterfacePresentation>,
    /// Some selects the timing renderer, including an empty authored trace.
    pub timing: Option<TimingPresentation>,
    pub history_label: String,
    pub composite_role: bool,
    pub interaction_operator: Option<String>,
    pub slot_labels: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationshipMarker {
    None,
    OpenArrow,
    Triangle,
    Diamond,
    FilledDiamond,
    FilledTriangle,
    FilledCircle,
    OpenArrowAndCircle,
    FilledTriangleAndCircle,
    Containment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelationshipStyle {
    pub source_marker: RelationshipMarker,
    pub target_marker: RelationshipMarker,
    pub line_mode: LineMode,
    pub middle_marker: RelationshipMarker,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RelationshipEndpointLabels {
    pub role: Option<String>,
    pub multiplicity: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RelationshipPresentation {
    pub name: Option<String>,
    pub source_labels: RelationshipEndpointLabels,
    pub target_labels: RelationshipEndpointLabels,
    pub type_label: String,
}

pub use crate::contracts::{
    EndpointMarkerComposition, EndpointMarkerGlyph, EndpointMarkerPart, EndpointMarkerPlacement,
};

impl UmlNotation {
    pub fn minimum_size(self) -> (f64, f64) {
        match self {
            Self::InitialNode
            | Self::FinalNode
            | Self::FlowFinal
            | Self::History
            | Self::Terminate => (20.0, 20.0),
            Self::Fork => (20.0, 8.0),
            Self::Execution => (12.0, 24.0),
            Self::Port => (12.0, 12.0),
            Self::Decision => (32.0, 32.0),
            _ => (80.0, 48.0),
        }
    }
}
