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
    #[serde(default)]
    pub corner_radius: Option<f64>,
    #[serde(default)]
    pub dash_pattern: Vec<f64>,
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
