//! Paint the domain's relationship notation with native GPUI paths.
use bellatrix_design_system::Palette;
use bellatrix_mde::{RelationshipMarker, RelationshipPresentation, RelationshipStyle};
use gpui::{
    App, Bounds, IntoElement, PathBuilder, Pixels, RenderOnce, Window, canvas, div, point,
    prelude::*, px, rgb,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UmlLineSegment {
    Line {
        to: (f32, f32),
    },
    Cubic {
        control1: (f32, f32),
        control2: (f32, f32),
        to: (f32, f32),
    },
}
#[derive(IntoElement)]
pub struct UmlRelationGlyph {
    style: RelationshipStyle,
    from: (f32, f32),
    to: (f32, f32),
    segments: Vec<UmlLineSegment>,
    selected: bool,
    labels: Option<RelationshipPresentation>,
    scale: f32,
}
impl UmlRelationGlyph {
    pub fn new(
        style: RelationshipStyle,
        _width: f32,
        _height: f32,
        from: (f32, f32),
        to: (f32, f32),
    ) -> Self {
        Self {
            style,
            from,
            to,
            segments: Vec::new(),
            selected: false,
            labels: None,
            scale: 1.0,
        }
    }
    pub fn labels(mut self, presentation: RelationshipPresentation) -> Self {
        self.labels = Some(presentation);
        self
    }
    pub fn view_scale(mut self, scale: f32) -> Self {
        if scale.is_finite() && scale > 0.0 {
            self.scale = scale;
        }
        self
    }
    pub fn cubic(mut self, control1: (f32, f32), control2: (f32, f32)) -> Self {
        self.segments = vec![UmlLineSegment::Cubic {
            control1,
            control2,
            to: self.to,
        }];
        self
    }
    pub fn segments(mut self, segments: Vec<UmlLineSegment>) -> Self {
        self.segments = segments;
        self
    }
    pub fn selected(mut self, value: bool) -> Self {
        self.selected = value;
        self
    }
}
fn tangent(start: (f32, f32), end: (f32, f32)) -> (f32, f32) {
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let length = dx.hypot(dy);
    if length <= f32::EPSILON {
        (1.0, 0.0)
    } else {
        (dx / length, dy / length)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct LabelAnchor {
    position: (f32, f32),
    right_aligned: bool,
    above: bool,
}
fn endpoint_label_anchor(endpoint: (f32, f32), inward: (f32, f32), scale: f32) -> LabelAnchor {
    let normal = if inward.0 >= 0.0 {
        (inward.1, -inward.0)
    } else {
        (-inward.1, inward.0)
    };
    LabelAnchor {
        position: (
            endpoint.0 + (24.0 * inward.0 + 8.0 * normal.0) * scale,
            endpoint.1 + (24.0 * inward.1 + 8.0 * normal.1) * scale,
        ),
        right_aligned: inward.0 < 0.0,
        above: normal.1 < 0.0 || (normal.1 == 0.0 && inward.1 < 0.0),
    }
}
fn path_label_anchors(
    from: (f32, f32),
    segments: &[UmlLineSegment],
    scale: f32,
) -> (LabelAnchor, LabelAnchor, (f32, f32)) {
    let mut last = from;
    let mut initial_direction = (1.0, 0.0);
    let mut final_direction = initial_direction;
    let mut middle = from;
    for (index, segment) in segments.iter().enumerate() {
        let (next, start_direction, end_direction, midpoint) = match *segment {
            UmlLineSegment::Line { to } => {
                let direction = tangent(last, to);
                (
                    to,
                    direction,
                    direction,
                    ((last.0 + to.0) * 0.5, (last.1 + to.1) * 0.5),
                )
            }
            UmlLineSegment::Cubic {
                control1,
                control2,
                to,
            } => (
                to,
                tangent(last, control1),
                tangent(control2, to),
                (
                    (last.0 + 3.0 * control1.0 + 3.0 * control2.0 + to.0) / 8.0,
                    (last.1 + 3.0 * control1.1 + 3.0 * control2.1 + to.1) / 8.0,
                ),
            ),
        };
        if index == 0 {
            initial_direction = start_direction;
        }
        if index == segments.len() / 2 {
            middle = midpoint;
        }
        final_direction = end_direction;
        last = next;
    }
    (
        endpoint_label_anchor(from, initial_direction, scale),
        endpoint_label_anchor(last, (-final_direction.0, -final_direction.1), scale),
        middle,
    )
}
fn label_block(
    anchor: LabelAnchor,
    lines: impl IntoIterator<Item = String>,
    scale: f32,
    color: u32,
) -> impl IntoElement {
    div()
        .absolute()
        .left(px(anchor.position.0))
        .top(px(anchor.position.1))
        .size(px(0.0))
        .child(
            div()
                .absolute()
                .flex()
                .flex_col()
                .whitespace_nowrap()
                .text_size(px(12.0 * scale))
                .text_color(rgb(color))
                .when(anchor.right_aligned, |element| {
                    element.right(px(0.0)).items_end()
                })
                .when(!anchor.right_aligned, |element| {
                    element.left(px(0.0)).items_start()
                })
                .when(anchor.above, |element| element.bottom(px(0.0)))
                .when(!anchor.above, |element| element.top(px(0.0)))
                .children(
                    lines
                        .into_iter()
                        .filter(|line| !line.is_empty())
                        .map(|line| div().child(line)),
                ),
        )
}
fn stroke_line(
    window: &mut Window,
    from: (f32, f32),
    to: (f32, f32),
    bounds: Bounds<Pixels>,
    color: gpui::Rgba,
    width: f32,
) {
    let absolute = |position: (f32, f32)| {
        point(
            bounds.origin.x + px(position.0),
            bounds.origin.y + px(position.1),
        )
    };
    let mut builder = PathBuilder::stroke(px(width));
    builder.move_to(absolute(from));
    builder.line_to(absolute(to));
    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}
fn polygon(
    window: &mut Window,
    vertices: &[(f32, f32)],
    bounds: Bounds<Pixels>,
    outline: gpui::Rgba,
    background: gpui::Rgba,
    filled: bool,
    width: f32,
) {
    let absolute = vertices
        .iter()
        .map(|position| {
            point(
                bounds.origin.x + px(position.0),
                bounds.origin.y + px(position.1),
            )
        })
        .collect::<Vec<_>>();
    let mut fill = PathBuilder::fill();
    fill.add_polygon(&absolute, true);
    if let Ok(path) = fill.build() {
        window.paint_path(path, if filled { outline } else { background });
    }
    let mut border = PathBuilder::stroke(px(width));
    border.add_polygon(&absolute, true);
    if let Ok(path) = border.build() {
        window.paint_path(path, outline);
    }
}
fn paint_marker(
    window: &mut Window,
    marker: RelationshipMarker,
    endpoint: (f32, f32),
    direction: (f32, f32),
    bounds: Bounds<Pixels>,
    stroke: gpui::Rgba,
    background: gpui::Rgba,
    width: f32,
) {
    let (ux, uy) = direction;
    let perpendicular = (-uy, ux);
    let offset = |forward: f32, side: f32| {
        (
            endpoint.0 + forward * ux + side * perpendicular.0,
            endpoint.1 + forward * uy + side * perpendicular.1,
        )
    };
    use RelationshipMarker::*;
    match marker {
        None => {}
        OpenArrow => {
            stroke_line(window, offset(-12.0, 6.0), endpoint, bounds, stroke, width);
            stroke_line(window, endpoint, offset(-12.0, -6.0), bounds, stroke, width);
        }
        Triangle => polygon(
            window,
            &[endpoint, offset(-17.0, 8.0), offset(-17.0, -8.0)],
            bounds,
            stroke,
            background,
            false,
            width,
        ),
        Diamond | FilledDiamond => polygon(
            window,
            &[
                endpoint,
                offset(10.0, 7.0),
                offset(21.0, 0.0),
                offset(10.0, -7.0),
            ],
            bounds,
            stroke,
            background,
            marker == FilledDiamond,
            width,
        ),
        Containment => {
            let mut outline = PathBuilder::stroke(px(width));
            let center = point(
                bounds.origin.x + px(endpoint.0),
                bounds.origin.y + px(endpoint.1),
            );
            let radius = px(7.0);
            outline.move_to(point(center.x + radius, center.y));
            outline.arc_to(
                point(radius, radius),
                px(0.0),
                false,
                false,
                point(center.x - radius, center.y),
            );
            outline.arc_to(
                point(radius, radius),
                px(0.0),
                false,
                false,
                point(center.x + radius, center.y),
            );
            if let Ok(path) = outline.build() {
                window.paint_path(path, stroke);
            }
            stroke_line(
                window,
                offset(0.0, -4.0),
                offset(0.0, 4.0),
                bounds,
                stroke,
                width,
            );
            stroke_line(
                window,
                offset(-4.0, 0.0),
                offset(4.0, 0.0),
                bounds,
                stroke,
                width,
            );
        }
    }
}
impl RenderOnce for UmlRelationGlyph {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let palette = Palette::current(cx);
        let stroke = rgb(if self.selected {
            palette.selection
        } else {
            palette.text
        });
        let background = rgb(palette.canvas);
        let RelationshipStyle {
            source_marker,
            target_marker,
            dashed,
        } = self.style;
        let from = self.from;
        let to = self.to;
        let segments = if self.segments.is_empty() {
            vec![UmlLineSegment::Line { to }]
        } else {
            self.segments
        };
        let width = if self.selected { 2.6 } else { 1.8 };
        let (source_anchor, target_anchor, middle) =
            path_label_anchors(from, &segments, self.scale);
        let labels = self.labels;
        let scale = self.scale;
        let glyph = canvas(
            |_, _, _| (),
            move |bounds, (), window, _| {
                let absolute = |position: (f32, f32)| {
                    point(
                        bounds.origin.x + px(position.0),
                        bounds.origin.y + px(position.1),
                    )
                };
                let mut builder = PathBuilder::stroke(px(width));
                if dashed {
                    builder = builder.dash_array(&[px(6.0), px(5.0)]);
                }
                builder.move_to(absolute(from));
                let mut last = from;
                let mut initial_tangent = tangent(from, to);
                let mut final_tangent = initial_tangent;
                for (index, segment) in segments.iter().enumerate() {
                    match *segment {
                        UmlLineSegment::Line { to: next } => {
                            let direction = tangent(last, next);
                            if index == 0 {
                                initial_tangent = direction;
                            }
                            final_tangent = direction;
                            builder.line_to(absolute(next));
                            last = next;
                        }
                        UmlLineSegment::Cubic {
                            control1,
                            control2,
                            to: next,
                        } => {
                            if index == 0 {
                                initial_tangent = tangent(last, control1);
                            }
                            final_tangent = tangent(control2, next);
                            builder.cubic_bezier_to(
                                absolute(next),
                                absolute(control1),
                                absolute(control2),
                            );
                            last = next;
                        }
                    }
                }
                if let Ok(path) = builder.build() {
                    window.paint_path(path, stroke);
                }
                paint_marker(
                    window,
                    source_marker,
                    from,
                    initial_tangent,
                    bounds,
                    stroke,
                    background,
                    width,
                );
                paint_marker(
                    window,
                    target_marker,
                    last,
                    final_tangent,
                    bounds,
                    stroke,
                    background,
                    width,
                );
            },
        )
        .size_full();
        div()
            .relative()
            .size_full()
            .child(glyph)
            .when_some(labels, |element, labels| {
                element
                    .child(label_block(
                        source_anchor,
                        labels
                            .source_labels
                            .role
                            .into_iter()
                            .chain(labels.source_labels.multiplicity),
                        scale,
                        palette.text,
                    ))
                    .child(label_block(
                        target_anchor,
                        labels
                            .target_labels
                            .role
                            .into_iter()
                            .chain(labels.target_labels.multiplicity),
                        scale,
                        palette.text,
                    ))
                    .when_some(
                        labels.name.filter(|name| !name.is_empty()),
                        |element, name| {
                            element.child(label_block(
                                LabelAnchor {
                                    position: (middle.0, middle.1 - 8.0 * scale),
                                    right_aligned: false,
                                    above: true,
                                },
                                [name],
                                scale,
                                palette.text,
                            ))
                        },
                    )
            })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use bellatrix_mde::relationship_style_for_notation;
    #[test]
    fn endpoint_labels_follow_their_own_ends_and_view_scale() {
        let segments = [UmlLineSegment::Line { to: (200.0, 40.0) }];
        let (source, target, middle) = path_label_anchors((0.0, 40.0), &segments, 1.0);
        assert_eq!(source.position, (24.0, 32.0));
        assert_eq!(target.position, (176.0, 32.0));
        assert!(!source.right_aligned);
        assert!(target.right_aligned);
        assert!(source.above && target.above);
        assert_eq!(middle, (100.0, 40.0));
        let scaled = [UmlLineSegment::Line { to: (400.0, 80.0) }];
        let (source, target, middle) = path_label_anchors((0.0, 80.0), &scaled, 2.0);
        assert_eq!(source.position, (48.0, 64.0));
        assert_eq!(target.position, (352.0, 64.0));
        assert_eq!(middle, (200.0, 80.0));
    }
    #[test]
    fn reversed_and_vertical_routes_keep_source_and_target_labels_distinct() {
        let reversed = [UmlLineSegment::Line { to: (0.0, 40.0) }];
        let (source, target, _) = path_label_anchors((200.0, 40.0), &reversed, 1.0);
        assert_eq!(source.position, (176.0, 32.0));
        assert_eq!(target.position, (24.0, 32.0));
        assert!(source.right_aligned);
        assert!(!target.right_aligned);
        let vertical = [UmlLineSegment::Line { to: (40.0, 200.0) }];
        let (source, target, _) = path_label_anchors((40.0, 0.0), &vertical, 1.0);
        assert_eq!(source.position, (48.0, 24.0));
        assert_eq!(target.position, (32.0, 176.0));
        assert!(!source.above);
        assert!(target.above);
    }
    #[test]
    fn curved_and_self_edges_use_path_tangents_and_bezier_midpoint() {
        let segments = [UmlLineSegment::Cubic {
            control1: (40.0, 0.0),
            control2: (40.0, 80.0),
            to: (0.0, 80.0),
        }];
        let (source, target, middle) = path_label_anchors((0.0, 0.0), &segments, 1.0);
        assert_eq!(source.position, (24.0, -8.0));
        assert_eq!(target.position, (24.0, 72.0));
        assert_eq!(middle, (30.0, 40.0));
        let zero = [UmlLineSegment::Line { to: (2.0, 3.0) }];
        let (source, target, middle) = path_label_anchors((2.0, 3.0), &zero, 1.0);
        assert_eq!(source.position, (26.0, -5.0));
        assert_eq!(target.position, (-22.0, -5.0));
        assert_eq!(middle, (2.0, 3.0));
    }

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
    fn zero_length_edges_have_a_finite_tangent() {
        assert_eq!(tangent((2.0, 3.0), (2.0, 3.0)), (1.0, 0.0));
    }
}
