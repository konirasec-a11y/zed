use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DocumentationEnvelope {
    pub api_version: u32,
    pub document: Documentation,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Documentation {
    pub name: String,
    pub elements: Vec<ElementPage>,
    pub relations: Vec<RelationLink>,
    pub diagrams: Vec<DiagramPage>,
}

impl Documentation {
    pub fn element(&self, id: &str) -> Option<&ElementPage> {
        self.elements.iter().find(|element| element.id == id)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ElementPage {
    pub id: String,
    pub name: String,
    pub label: String,
    pub kind: String,
    pub description: Option<String>,
    pub properties: BTreeMap<String, String>,
    pub annotations: Vec<SourceText>,
    pub implementations: Vec<SourceText>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceText {
    pub language: String,
    pub source: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RelationLink {
    pub id: String,
    pub kind: String,
    pub from: String,
    pub to: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DiagramPage {
    pub id: String,
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub page_subjects: Vec<String>,
    pub views: Vec<RenderedView>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RenderedView {
    pub id: String,
    pub subject: Option<String>,
    pub label: String,
    pub geometry: Geometry,
    pub z_index: i32,
    pub style: Appearance,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Appearance {
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub stroke_width: Option<f64>,
    pub font_family: Option<String>,
    pub font_size: Option<f64>,
    pub opacity: Option<f64>,
    pub text_color: Option<String>,
    pub text_align: Option<TextAlignment>,
    pub text_vertical_align: Option<TextVerticalAlignment>,
    pub corner_radius: Option<f64>,
    pub dash_pattern: Vec<f64>,
    pub show_shadow: Option<bool>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Geometry {
    Layout,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Point {
    #[serde(deserialize_with = "deserialize_coordinate")]
    pub x: f64,
    #[serde(deserialize_with = "deserialize_coordinate")]
    pub y: f64,
}

// Feature unification can enable arbitrary_precision in the caller. Tagged
// enums then buffer decimal numbers as serde_json's private number map.
fn deserialize_coordinate<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<f64, D::Error> {
    let number = serde_json::Number::deserialize(deserializer)?;
    number
        .as_f64()
        .filter(|coordinate| coordinate.is_finite())
        .ok_or_else(|| serde::de::Error::custom("diagram coordinates must be finite numbers"))
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum PathSegment {
    Line {
        to: Point,
    },
    Cubic {
        control1: Point,
        control2: Point,
        to: Point,
    },
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TextAlignment {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TextVerticalAlignment {
    Top,
    Center,
    Bottom,
}
