//! Paint the domain's relationship notation with native GPUI paths.
use bellatrix_design_system::Palette;
use bellatrix_mde::{RelationshipMarker, RelationshipStyle};
use gpui::{App, Bounds, IntoElement, PathBuilder, Pixels, RenderOnce, Window, canvas, point, prelude::*, px, rgb};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UmlLineSegment {
    Line { to: (f32, f32) },
    Cubic { control1: (f32, f32), control2: (f32, f32), to: (f32, f32) },
}
#[derive(IntoElement)]
pub struct UmlRelationGlyph {
    style: RelationshipStyle,
    from: (f32, f32),
    to: (f32, f32),
    segments: Vec<UmlLineSegment>,
    selected: bool,
}
impl UmlRelationGlyph {
    pub fn new(style: RelationshipStyle, _width: f32, _height: f32, from: (f32, f32), to: (f32, f32)) -> Self {
        Self { style, from, to, segments: Vec::new(), selected: false }
    }
    pub fn cubic(mut self, control1: (f32, f32), control2: (f32, f32)) -> Self {
        self.segments = vec![UmlLineSegment::Cubic { control1, control2, to: self.to }]; self
    }
    pub fn segments(mut self, segments: Vec<UmlLineSegment>) -> Self { self.segments = segments; self }
    pub fn selected(mut self, value: bool) -> Self { self.selected = value; self }
}
fn tangent(start: (f32, f32), end: (f32, f32)) -> (f32, f32) {
    let dx = end.0 - start.0; let dy = end.1 - start.1; let length = dx.hypot(dy);
    if length <= f32::EPSILON { (1.0, 0.0) } else { (dx / length, dy / length) }
}
fn stroke_line(window: &mut Window, from: (f32, f32), to: (f32, f32), bounds: Bounds<Pixels>, color: gpui::Rgba, width: f32) {
    let absolute = |position: (f32, f32)| point(bounds.origin.x + px(position.0), bounds.origin.y + px(position.1));
    let mut builder = PathBuilder::stroke(px(width));
    builder.move_to(absolute(from)); builder.line_to(absolute(to));
    if let Ok(path) = builder.build() { window.paint_path(path, color); }
}
fn polygon(window: &mut Window, vertices: &[(f32, f32)], bounds: Bounds<Pixels>,
    outline: gpui::Rgba, background: gpui::Rgba, filled: bool, width: f32) {
    let absolute = vertices.iter().map(|position|
        point(bounds.origin.x + px(position.0), bounds.origin.y + px(position.1))).collect::<Vec<_>>();
    let mut fill = PathBuilder::fill(); fill.add_polygon(&absolute, true);
    if let Ok(path) = fill.build() { window.paint_path(path, if filled { outline } else { background }); }
    let mut border = PathBuilder::stroke(px(width)); border.add_polygon(&absolute, true);
    if let Ok(path) = border.build() { window.paint_path(path, outline); }
}
fn paint_marker(window: &mut Window, marker: RelationshipMarker, endpoint: (f32, f32), direction: (f32, f32),
    bounds: Bounds<Pixels>, stroke: gpui::Rgba, background: gpui::Rgba, width: f32) {
    let (ux, uy) = direction;
    let perpendicular = (-uy, ux);
    let offset = |forward: f32, side: f32| (
        endpoint.0 + forward * ux + side * perpendicular.0,
        endpoint.1 + forward * uy + side * perpendicular.1,
    );
    use RelationshipMarker::*;
    match marker {
        None => {}
        OpenArrow => {
            stroke_line(window, offset(-12.0, 6.0), endpoint, bounds, stroke, width);
            stroke_line(window, endpoint, offset(-12.0, -6.0), bounds, stroke, width);
        }
        Triangle => polygon(window, &[endpoint, offset(-17.0, 8.0), offset(-17.0, -8.0)], bounds, stroke, background, false, width),
        Diamond | FilledDiamond => polygon(window, &[endpoint, offset(10.0, 7.0), offset(21.0, 0.0), offset(10.0, -7.0)],
            bounds, stroke, background, marker == FilledDiamond, width),
        Containment => {
            let mut outline = PathBuilder::stroke(px(width));
            let center = point(bounds.origin.x + px(endpoint.0), bounds.origin.y + px(endpoint.1));
            let radius = px(7.0);
            outline.move_to(point(center.x + radius, center.y));
            outline.arc_to(point(radius, radius), px(0.0), false, false, point(center.x - radius, center.y));
            outline.arc_to(point(radius, radius), px(0.0), false, false, point(center.x + radius, center.y));
            if let Ok(path) = outline.build() { window.paint_path(path, stroke); }
            stroke_line(window, offset(0.0, -4.0), offset(0.0, 4.0), bounds, stroke, width);
            stroke_line(window, offset(-4.0, 0.0), offset(4.0, 0.0), bounds, stroke, width);
        }
    }
}
impl RenderOnce for UmlRelationGlyph {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let palette = Palette::current(cx);
        let stroke = rgb(if self.selected { palette.selection } else { palette.text });
        let background = rgb(palette.canvas);
        let RelationshipStyle { source_marker, target_marker, dashed } = self.style;
        let from = self.from; let to = self.to;
        let segments = if self.segments.is_empty() { vec![UmlLineSegment::Line { to }] } else { self.segments };
        let width = if self.selected { 2.6 } else { 1.8 };
        canvas(|_, _, _| (), move |bounds, (), window, _| {
            let absolute = |position: (f32, f32)| point(bounds.origin.x + px(position.0), bounds.origin.y + px(position.1));
            let mut builder = PathBuilder::stroke(px(width));
            if dashed { builder = builder.dash_array(&[px(6.0), px(5.0)]); }
            builder.move_to(absolute(from));
            let mut last = from; let mut initial_tangent = tangent(from, to); let mut final_tangent = initial_tangent;
            for (index, segment) in segments.iter().enumerate() {
                match *segment {
                    UmlLineSegment::Line { to: next } => {
                        let direction = tangent(last, next);
                        if index == 0 { initial_tangent = direction; }
                        final_tangent = direction; builder.line_to(absolute(next)); last = next;
                    }
                    UmlLineSegment::Cubic { control1, control2, to: next } => {
                        if index == 0 { initial_tangent = tangent(last, control1); }
                        final_tangent = tangent(control2, next);
                        builder.cubic_bezier_to(absolute(next), absolute(control1), absolute(control2)); last = next;
                    }
                }
            }
            if let Ok(path) = builder.build() { window.paint_path(path, stroke); }
            paint_marker(window, source_marker, from, initial_tangent, bounds, stroke, background, width);
            paint_marker(window, target_marker, last, final_tangent, bounds, stroke, background, width);
        }).size_full()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use bellatrix_mde::relationship_style_for_notation;
    #[test]
    fn renderer_uses_graph_domain_markers_without_reinterpreting_semantics() {
        let association = relationship_style_for_notation("uml.association");
        let aggregation = relationship_style_for_notation("uml.aggregation");
        let generalization = relationship_style_for_notation("uml.generalization");
        assert_eq!(association.source_marker, RelationshipMarker::None);
        assert_eq!(aggregation.source_marker, RelationshipMarker::Diamond);
        assert_eq!(generalization.target_marker, RelationshipMarker::Triangle);
        assert!(!generalization.dashed);
    }
    #[test]
    fn zero_length_edges_have_a_finite_tangent() { assert_eq!(tangent((2.0, 3.0), (2.0, 3.0)), (1.0, 0.0)); }
}
