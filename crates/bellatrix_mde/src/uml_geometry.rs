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
        let counts = [self.attributes, self.operations, self.receptions, self.template_parameters];
        let rows: usize = counts.iter().sum();
        let groups = counts.iter().filter(|count| **count > 0).count();
        rows as f64 * 20.0 + groups as f64 * 9.0
    }
    pub fn minimum_interface_height(self) -> f64 {
        76.0 + self.compartments_height() + if self.stereotype { 14.0 } else { 0.0 }
    }
}
pub fn classifier_contents(section: &ModelingSection, owner: &str) -> ClassifierContents {
    let mut contents = ClassifierContents::default();
    contents.stereotype = section.elements.iter().find(|element| element.id == owner)
        .and_then(|element| element.properties.get("stereotype"))
        .and_then(serde_json::Value::as_str).is_some_and(|value| !value.trim().is_empty());
    for relation in section.relations.iter().filter(|relation| relation.kind == "owns" && relation.from == owner) {
        if let Some(element) = section.elements.iter().find(|element| element.id == relation.to) {
            match element.kind.as_str() {
                "rust.field" | "uml.attribute" => contents.attributes += 1,
                "rust.function" | "uml.operation" => contents.operations += 1,
                "uml.reception" => contents.receptions += 1,
                "uml.template_parameter" => contents.template_parameters += 1,
                _ => {}
            }
        }
    }
    contents
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
        if length <= f64::EPSILON { return Point { x: self.center.x + self.radius, y: self.center.y }; }
        Point { x: self.center.x + delta_x * self.radius / length, y: self.center.y + delta_y * self.radius / length }
    }
}
pub fn interface_layout(width: f64, height: f64, contents: ClassifierContents) -> InterfaceLayout {
    let width = finite_size(width);
    // Old saved Views can be shorter than their compartments. Resolve the
    // intrinsic minimum here, so rendering and routing cannot disagree.
    let height = finite_size(height).max(contents.minimum_interface_height());
    let stereotype_height = if contents.stereotype { 14.0 } else { 0.0 };
    let reserved = 48.0 + stereotype_height + contents.compartments_height();
    let diameter = (width - 16.0).min(height - reserved).max(1.0).min(width).min(height);
    let radius = diameter / 2.0;
    let top = 8.0_f64.min((height - diameter).max(0.0) / 2.0);
    let label_top = top + diameter + 4.0 + stereotype_height;
    InterfaceLayout { center: Point { x: width / 2.0, y: top + radius }, radius, label_top, members_top: label_top + 22.0 }
}
fn finite_size(value: f64) -> f64 { if value.is_finite() && value > 0.0 { value } else { 1.0 } }

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
        outline: [Point { x: left, y: top }, Point { x: right - fold_size, y: top },
            Point { x: right, y: top + fold_size }, Point { x: right, y: bottom }, Point { x: left, y: bottom }],
        fold: [Point { x: right - fold_size, y: top }, Point { x: right - fold_size, y: top + fold_size }, Point { x: right, y: top + fold_size }],
        fold_size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let contents = ClassifierContents { attributes: 2, operations: 1, receptions: 1, template_parameters: 0, stereotype: true };
        let layout = interface_layout(120.0, contents.minimum_interface_height(), contents);
        assert_eq!(layout.radius, 14.0);
        assert!(layout.members_top + contents.compartments_height() <= contents.minimum_interface_height());
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
