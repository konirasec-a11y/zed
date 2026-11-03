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
pub struct View {
    pub id: String,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    pub geometry: Geometry,
    #[serde(default)]
    pub style: Style,
    #[serde(default)]
    pub z_index: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Geometry {
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    Ellipse {
        cx: f64,
        cy: f64,
        rx: f64,
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StereotypeDisplay {
    #[serde(rename = "none")]
    Hidden,
    Label,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stereotype_display: Option<StereotypeDisplay>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suppress_attributes: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suppress_operations: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suppress_receptions: Option<bool>,
}

impl Style {
    pub fn should_show_stereotype(&self) -> bool {
        self.stereotype_display != Some(StereotypeDisplay::Hidden)
    }

    pub fn resolve_stroke_pattern(&self, notation_dashed: bool) -> &[f64] {
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
            _ if notation_dashed => &[6.0, 5.0],
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
