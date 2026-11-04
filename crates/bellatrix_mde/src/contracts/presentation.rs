use super::endpoint_presentation::RelationshipEndpointPresentation;
use super::uml_presentation::UmlStyle;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Diagram {
    pub id: String,
    pub name: String,
    #[serde(default = "default_width")]
    pub width: f64,
    #[serde(default = "default_height")]
    pub height: f64,
    #[serde(default)]
    pub views: Vec<View>,
    /// Required UMLDI metaclass name; explicit future names survive round trips.
    #[serde(deserialize_with = "deserialize_diagram_type")]
    pub uml_di_type: String,
    /// UMLInteractionDiagram.kind; absent uses the UMLDI default, sequence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction_kind: Option<String>,
    /// UMLDI diagram context, not the subjects of its contained Views.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub model_elements: Vec<String>,
}
fn deserialize_diagram_type<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    if value.trim().is_empty() {
        return Err(serde::de::Error::custom(
            "diagram uml_di_type must not be blank",
        ));
    }
    Ok(value)
}

fn default_width() -> f64 {
    1280.0
}
fn default_height() -> f64 {
    720.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub id: String,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft: Option<VisualDraft>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<ViewEndpoints>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<TimingPresentation>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub geometry: Geometry,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<View>,
    #[serde(default, skip_serializing_if = "ViewPresentation::is_empty")]
    pub presentation: ViewPresentation,
    #[serde(default)]
    pub style: Style,
    #[serde(default)]
    pub z_index: i32,
}

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
    #[serde(
        default,
        skip_serializing_if = "Authored::is_missing",
        deserialize_with = "deserialize_authored_coordinate"
    )]
    pub width: Authored<f64>,
    #[serde(
        default,
        skip_serializing_if = "Authored::is_missing",
        deserialize_with = "deserialize_authored_coordinate"
    )]
    pub height: Authored<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewPresentation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relationship_endpoints: Option<RelationshipEndpointPresentation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub properties: std::collections::BTreeMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub references: std::collections::BTreeMap<String, Authored<PresentationReference>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_placement: Option<LabelPlacement>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub resources: std::collections::BTreeMap<String, PresentationResource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PresentationSource>,
}

impl ViewPresentation {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "target",
    content = "id",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum PresentationReference {
    View(String),
    Model(String),
    Resource(String),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabelPlacement {
    #[serde(default, skip_serializing_if = "Authored::is_missing")]
    pub anchor: Authored<String>,
    #[serde(default, skip_serializing_if = "Authored::is_missing")]
    pub angle: Authored<f64>,
    #[serde(default, skip_serializing_if = "Authored::is_missing")]
    pub distance: Authored<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PresentationResource {
    Uri {
        uri: String,
    },
    Embedded {
        media_type: String,
        data_base64: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresentationSource {
    pub format: String,
    pub type_name: String,
    // Only unmapped source fields belong here. Mapped fields are authored once
    // in this contract, never in a second copy of the original source object.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub residual: std::collections::BTreeMap<String, serde_json::Value>,
}

impl Diagram {
    pub fn all_views(&self) -> impl Iterator<Item = &View> {
        let mut pending: Vec<_> = self.views.iter().rev().collect();
        std::iter::from_fn(move || {
            let view = pending.pop()?;
            pending.extend(view.children.iter().rev());
            Some(view)
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewEndpoints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_view_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_view_id: Option<String>,
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

impl Style {
    pub fn resolve_stroke_pattern(&self, notation_mode: LineMode) -> &[f64] {
        match self.line_mode {
            Some(LineMode::Solid) => return &[],
            Some(LineMode::Dotted) => return &[1.0, 3.0],
            _ => {}
        }
        // Existing files can carry custom patterns without an explicit mode.
        if !self.dash_pattern.is_empty()
            && self
                .dash_pattern
                .iter()
                .all(|value| value.is_finite() && *value > 0.0 && *value <= 1000.0)
        {
            return &self.dash_pattern;
        }
        match self.line_mode {
            Some(LineMode::Dashed) => &[6.0, 4.0],
            _ if notation_mode == LineMode::Dotted => &[1.0, 3.0],
            _ if notation_mode == LineMode::Dashed => &[6.0, 5.0],
            _ => &[],
        }
    }
}

/// Moves a visual representation; semantic objects remain untouched.
impl Geometry {
    pub fn translate(&mut self, dx: f64, dy: f64) {
        let move_point = |point: &mut Point| {
            point.x += dx;
            point.y += dy;
        };
        match self {
            Self::Layout { bounds } => {
                if let Authored::Value(x) = &mut bounds.x {
                    *x += dx;
                }
                if let Authored::Value(y) = &mut bounds.y {
                    *y += dy;
                }
            }
            Self::Rect { x, y, .. } => {
                *x += dx;
                *y += dy;
            }
            Self::Ellipse { cx, cy, .. } => {
                *cx += dx;
                *cy += dy;
            }
            Self::Polygon { points } => {
                for point in points {
                    move_point(point);
                }
            }
            Self::Path { start, segments } => {
                move_point(start);
                for segment in segments {
                    match segment {
                        PathSegment::Line { to } => move_point(to),
                        PathSegment::Cubic {
                            control1,
                            control2,
                            to,
                        } => {
                            move_point(control1);
                            move_point(control2);
                            move_point(to);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewOccurrence {
    pub diagram_id: String,
    pub view_id: String,
    pub member_path: Vec<String>,
}

/// Authored containment of a diagram, independent of its notation-specific context.
/// Semantic elements retain their domain ownership rather than a second parent here.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HierarchyPlacement {
    pub parent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occurrence: Option<ViewOccurrence>,
}
