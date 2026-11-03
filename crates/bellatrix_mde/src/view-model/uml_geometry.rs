//! Geometry shared by the UI projection and semantic connector routing.
use crate::{ModelingSection, Point};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClassifierContents {
    pub attributes: usize,
    pub operations: usize,
    pub receptions: usize,
    pub template_parameters: usize,
    pub stereotype: bool,
}
impl ClassifierContents {
    pub fn compartments_height(self) -> f64 {
        let counts = [self.attributes, self.operations, self.receptions];
        let rows: usize = counts.iter().sum();
        let groups = counts.iter().filter(|count| **count > 0).count();
        rows as f64 * 20.0 + groups as f64 * 9.0
    }
    pub fn minimum_interface_height(self) -> f64 {
        76.0 + self.compartments_height() + if self.stereotype { 14.0 } else { 0.0 }
    }
}
pub fn classifier_contents(section: &ModelingSection, owner: &str) -> ClassifierContents {
    let mut contents = ClassifierContents {
        stereotype: section
            .elements
            .iter()
            .find(|element| element.id == owner)
            .and_then(|element| element.properties.get("stereotype"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty()),
        ..Default::default()
    };
    for relation in section
        .relations
        .iter()
        .filter(|relation| relation.kind == "owns" && relation.from == owner)
    {
        if let Some(element) = section
            .elements
            .iter()
            .find(|element| element.id == relation.to)
        {
            match element.kind.as_str() {
                "rust.field" | "uml.attribute" => contents.attributes += 1,
                "rust.function" | "uml.operation" => contents.operations += 1,
                "uml.reception" => contents.receptions += 1,

                _ => {}
            }
        }
    }
    contents.template_parameters = crate::template_parameters(section, owner).len();
    contents
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemplateSignatureLayout {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

impl TemplateSignatureLayout {
    pub fn outsets(self, body_width: f64, body_height: f64) -> (f64, f64, f64) {
        (
            (-self.top).max(0.0),
            (self.left + self.width - body_width).max(0.0),
            (self.top + self.height - body_height).max(0.0),
        )
    }
}

pub fn template_signature_layout(
    body_width: f64,
    parameter_count: usize,
) -> Option<TemplateSignatureLayout> {
    if parameter_count == 0 || !body_width.is_finite() || body_width <= 0.0 {
        return None;
    }
    let width = body_width.min(120.0);
    let height = 12.0 + parameter_count as f64 * 20.0;
    Some(TemplateSignatureLayout {
        left: body_width - width + 16.0,
        top: 8.0 - height,
        width,
        height,
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InterfaceLayout {
    pub center: Point,
    pub radius: f64,
    pub label_top: f64,
    pub members_top: f64,
}
impl InterfaceLayout {
    pub fn anchor_toward(self, target: Point) -> Point {
        let delta_x = target.x - self.center.x;
        let delta_y = target.y - self.center.y;
        let length = delta_x.hypot(delta_y);
        if length <= f64::EPSILON {
            return Point {
                x: self.center.x + self.radius,
                y: self.center.y,
            };
        }
        Point {
            x: self.center.x + delta_x * self.radius / length,
            y: self.center.y + delta_y * self.radius / length,
        }
    }
}
pub fn interface_layout(width: f64, height: f64, contents: ClassifierContents) -> InterfaceLayout {
    let width = finite_size(width);
    // Old saved Views can be shorter than their compartments. Resolve the
    // intrinsic minimum here, so rendering and routing cannot disagree.
    let height = finite_size(height).max(contents.minimum_interface_height());
    let stereotype_height = if contents.stereotype { 14.0 } else { 0.0 };
    let reserved = 48.0 + stereotype_height + contents.compartments_height();
    let diameter = (width - 16.0)
        .min(height - reserved)
        .max(1.0)
        .min(width)
        .min(height);
    let radius = diameter / 2.0;
    let top = 8.0_f64.min((height - diameter).max(0.0) / 2.0);
    let label_top = top + diameter + 4.0 + stereotype_height;
    InterfaceLayout {
        center: Point {
            x: width / 2.0,
            y: top + radius,
        },
        radius,
        label_top,
        members_top: label_top + 22.0,
    }
}
fn finite_size(value: f64) -> f64 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        1.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FoldedNoteLayout {
    pub outline: [Point; 5],
    pub fold: [Point; 3],
    pub fold_size: f64,
}
pub fn folded_note_layout(width: f64, height: f64) -> FoldedNoteLayout {
    let width = finite_size(width);
    let height = finite_size(height);
    let inset = 0.5_f64.min(width / 4.0).min(height / 4.0);
    let fold_size = 12.0_f64.min(width / 4.0).min(height / 4.0);
    let left = inset;
    let top = inset;
    let right = width - inset;
    let bottom = height - inset;
    FoldedNoteLayout {
        outline: [
            Point { x: left, y: top },
            Point {
                x: right - fold_size,
                y: top,
            },
            Point {
                x: right,
                y: top + fold_size,
            },
            Point {
                x: right,
                y: bottom,
            },
            Point { x: left, y: bottom },
        ],
        fold: [
            Point {
                x: right - fold_size,
                y: top,
            },
            Point {
                x: right - fold_size,
                y: top + fold_size,
            },
            Point {
                x: right,
                y: top + fold_size,
            },
        ],
        fold_size,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameHeadingLayout {
    pub outline: [Point; 5],
}

pub fn frame_heading_layout(width: f64, height: f64) -> FrameHeadingLayout {
    let width = finite_size(width);
    let height = finite_size(height);
    let inset = 0.5_f64.min(width / 4.0).min(height / 4.0);
    let cut = 10.0_f64.min(width / 4.0).min(height / 3.0);
    let right = width - inset;
    let bottom = height - inset;
    FrameHeadingLayout {
        outline: [
            Point { x: inset, y: inset },
            Point { x: right, y: inset },
            Point {
                x: right,
                y: bottom - cut,
            },
            Point {
                x: right - cut,
                y: bottom,
            },
            Point {
                x: inset,
                y: bottom,
            },
        ],
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortLabelLayout {
    pub left: f64,
    pub top: f64,
}

pub fn port_label_layout(body_width: f64) -> PortLabelLayout {
    // The label is decoration, not part of the View bounds used for routing.
    PortLabelLayout {
        left: finite_size(body_width) + 4.0,
        top: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frame_heading_cuts_only_the_lower_right_corner_within_bounds() {
        for (width, height) in [(142.0, 30.0), (284.0, 60.0), (8.0, 4.0), (1.0, 1.0)] {
            let layout = frame_heading_layout(width, height);
            let [top_left, top_right, cut_top, cut_bottom, bottom_left] = layout.outline;
            assert_eq!(top_left.y, top_right.y);
            assert_eq!(top_right.x, cut_top.x);
            assert_eq!(cut_bottom.y, bottom_left.y);
            assert!(cut_top.y < cut_bottom.y);
            assert!(cut_bottom.x < cut_top.x);
            for scale in [0.25, 1.0, 3.0] {
                for point in layout.outline {
                    assert!((0.0..=width * scale).contains(&(point.x * scale)));
                    assert!((0.0..=height * scale).contains(&(point.y * scale)));
                }
            }
        }
    }

    #[test]
    fn port_label_is_outside_the_unchanged_body_at_every_scale() {
        for width in [1.0, 16.0, 32.0, 100.0] {
            let layout = port_label_layout(width);
            for scale in [0.25, 1.0, 3.0] {
                assert!(layout.left * scale > width * scale);
                assert_eq!(layout.top, 0.0);
            }
        }
    }

    #[test]
    fn port_label_projection_does_not_change_views_or_connector_anchors() {
        let project = serde_json::from_str(include_str!("../../fixtures/todo.json")).expect("fixture");
        let mut section = ModelingSection::from_model(&project);
        let port = section
            .elements
            .iter_mut()
            .find(|element| element.id == "type-list")
            .expect("owner");
        port.kind = "uml.port".into();
        port.properties
            .insert("type".into(), serde_json::json!("Service"));
        let before = section.clone();
        let anchors =
            crate::relationships::route(&section, "diagram-tasks", "type-list", "type-task")
                .expect("route");
        let port = section
            .elements
            .iter()
            .find(|element| element.id == "type-list")
            .expect("port");
        assert!(crate::element_display_label(port).ends_with(": Service {type text}"));
        let label = port_label_layout(16.0);
        assert!(label.left > 16.0);
        assert_eq!(section, before);
        assert_eq!(
            crate::relationships::route(&section, "diagram-tasks", "type-list", "type-task")
                .expect("route"),
            anchors
        );
    }

    #[test]
    fn interface_resize_changes_circle_and_keeps_anchor_on_circumference() {
        let contents = ClassifierContents::default();
        let small = interface_layout(120.0, 76.0, contents);
        let large = interface_layout(240.0, 152.0, contents);
        assert_eq!(small.radius, 14.0);
        assert!(large.radius > small.radius);
        for layout in [small, large] {
            let anchor = layout.anchor_toward(Point { x: 500.0, y: 400.0 });
            let distance = (anchor.x - layout.center.x).hypot(anchor.y - layout.center.y);
            assert!((distance - layout.radius).abs() < 1e-9);
            assert!(layout.label_top > layout.center.y + layout.radius);
        }
    }
    #[test]
    fn automatic_member_growth_does_not_enlarge_circle() {
        let contents = ClassifierContents {
            attributes: 2,
            operations: 1,
            receptions: 1,
            template_parameters: 0,
            stereotype: true,
        };
        let layout = interface_layout(120.0, contents.minimum_interface_height(), contents);
        assert_eq!(layout.radius, 14.0);
        assert!(
            layout.members_top + contents.compartments_height()
                <= contents.minimum_interface_height()
        );
        assert_eq!(interface_layout(120.0, 48.0, contents), layout);
    }
    #[test]
    fn folded_paper_never_draws_outside_its_view() {
        for (width, height) in [(80.0, 48.0), (136.0, 78.0), (400.0, 75.0), (90.0, 350.0)] {
            let paper = folded_note_layout(width, height);
            for point in paper.outline.into_iter().chain(paper.fold) {
                assert!(point.x >= 0.0 && point.x <= width);
                assert!(point.y >= 0.0 && point.y <= height);
            }
        }
    }
}
