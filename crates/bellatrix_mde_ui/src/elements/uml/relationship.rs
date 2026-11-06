//! Paint the domain's relationship notation with native GPUI paths.
use crate::elements::uml::class::ViewAppearance;
use bellatrix_design_system::Palette;
use bellatrix_mde_cordis::LineMode;
use bellatrix_mde_cordis::presentation_contract::{
    EndpointMarkerComposition, EndpointMarkerGlyph, EndpointMarkerPlacement, RelationshipMarker,
    RelationshipPresentation, RelationshipStyle, RenderStyle,
};
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
    appearance: ViewAppearance,
    communication_message: bool,
    endpoint_markers: Option<(EndpointMarkerComposition, EndpointMarkerComposition)>,
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
            appearance: ViewAppearance::default(),
            communication_message: false,
            endpoint_markers: None,
        }
    }
    pub fn communication_message(mut self, value: bool) -> Self {
        self.communication_message = value;
        self
    }
    pub fn view_style(mut self, style: &RenderStyle) -> Self {
        self.appearance = ViewAppearance(style.clone());
        self
    }
    pub fn endpoint_markers(
        mut self,
        source: EndpointMarkerComposition,
        target: EndpointMarkerComposition,
    ) -> Self {
        self.endpoint_markers = Some((source, target));
        self
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
// Repeated vertices and collapsed Bézier handles do not determine a direction.
// Use the first/last nonzero derivative so markers stay aligned after editing.
fn endpoint_directions(from: (f32, f32), segments: &[UmlLineSegment]) -> ((f32, f32), (f32, f32)) {
    let mut last = from;
    let mut initial = None;
    let mut final_direction = (1.0, 0.0);
    for segment in segments {
        let points = match *segment {
            UmlLineSegment::Line { to } => [last, to, to, to],
            UmlLineSegment::Cubic {
                control1,
                control2,
                to,
            } => [last, control1, control2, to],
        };
        let end = *points.last().unwrap_or(&last);
        if let Some(next) = points
            .iter()
            .skip(1)
            .find(|next| (next.0 - last.0).hypot(next.1 - last.1) > f32::EPSILON)
        {
            initial.get_or_insert_with(|| tangent(last, *next));
        }
        if let Some(previous) = points
            .iter()
            .rev()
            .skip(1)
            .find(|previous| (end.0 - previous.0).hypot(end.1 - previous.1) > f32::EPSILON)
        {
            final_direction = tangent(*previous, end);
        }
        last = end;
    }
    (initial.unwrap_or((1.0, 0.0)), final_direction)
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
) -> (LabelAnchor, LabelAnchor, (f32, f32), (f32, f32)) {
    let mut last = from;
    let (initial_direction, final_direction) = endpoint_directions(from, segments);
    let mut middle = from;
    let mut middle_direction = initial_direction;
    for (index, segment) in segments.iter().enumerate() {
        let (next, start_direction, _end_direction, midpoint) = match *segment {
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
        if index == segments.len() / 2 {
            middle = midpoint;
            middle_direction = match *segment {
                UmlLineSegment::Line { .. } => start_direction,
                UmlLineSegment::Cubic {
                    control1,
                    control2,
                    to,
                } => tangent(
                    (last.0 + control1.0, last.1 + control1.1),
                    (control2.0 + to.0, control2.1 + to.1),
                ),
            };
        }
        last = next;
    }
    (
        endpoint_label_anchor(from, initial_direction, scale),
        endpoint_label_anchor(last, (-final_direction.0, -final_direction.1), scale),
        middle,
        middle_direction,
    )
}
fn label_block(
    anchor: LabelAnchor,
    lines: impl IntoIterator<Item = String>,
    color: gpui::Rgba,
    font_size: f32,
    appearance: &ViewAppearance,
) -> gpui::AnyElement {
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
                .when(appearance.0.word_wrap == Some(true), |element| {
                    element.w(px(font_size * 16.0))
                })
                .text_size(px(font_size))
                .text_color(color)
                .when(anchor.right_aligned, |element| {
                    element.right(px(0.0)).items_end()
                })
                .when(!anchor.right_aligned, |element| {
                    element.left(px(0.0)).items_start()
                })
                .when(anchor.above, |element| element.bottom(px(0.0)))
                .when(!anchor.above, |element| element.top(px(0.0)))
                .min_h(px(font_size * 1.5))
                .map(|element| appearance.align_column(element))
                .children(
                    lines
                        .into_iter()
                        .filter(|line| !line.is_empty())
                        .map(|line| div().child(line)),
                ),
        )
        .into_any_element()
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
    inward_direction: (f32, f32),
    bounds: Bounds<Pixels>,
    stroke: gpui::Rgba,
    background: gpui::Rgba,
    width: f32,
    scale: f32,
) {
    // Marker bodies point toward the relationship, independently of endpoint.
    let (ux, uy) = inward_direction;
    let perpendicular = (-uy, ux);
    let offset = |forward: f32, side: f32| {
        (
            endpoint.0 + (forward * ux + side * perpendicular.0) * scale,
            endpoint.1 + (forward * uy + side * perpendicular.1) * scale,
        )
    };
    use RelationshipMarker::*;
    match marker {
        None => {}
        OpenArrow => {
            stroke_line(window, offset(12.0, 6.0), endpoint, bounds, stroke, width);
            stroke_line(window, endpoint, offset(12.0, -6.0), bounds, stroke, width);
        }
        Triangle | FilledTriangle => polygon(
            window,
            &[endpoint, offset(17.0, 8.0), offset(17.0, -8.0)],
            bounds,
            stroke,
            background,
            marker == FilledTriangle,
            width,
        ),
        OpenArrowAndCircle | FilledTriangleAndCircle => {
            paint_marker(
                window,
                FilledCircle,
                endpoint,
                inward_direction,
                bounds,
                stroke,
                background,
                width,
                scale,
            );
            let arrow = if marker == FilledTriangleAndCircle {
                FilledTriangle
            } else {
                OpenArrow
            };
            paint_marker(
                window,
                arrow,
                offset(5.0, 0.0),
                inward_direction,
                bounds,
                stroke,
                background,
                width,
                scale,
            );
        }
        FilledCircle => {
            let points = (0..32)
                .map(|step| {
                    let angle = step as f32 * std::f32::consts::TAU / 32.0;
                    offset(5.0 * angle.cos(), 5.0 * angle.sin())
                })
                .collect::<Vec<_>>();
            polygon(window, &points, bounds, stroke, background, true, width);
        }
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
            let radius = px(7.0 * scale);
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
fn paint_marker_composition(
    window: &mut Window,
    composition: &EndpointMarkerComposition,
    endpoint: (f32, f32),
    inward_direction: (f32, f32),
    bounds: Bounds<Pixels>,
    stroke: gpui::Rgba,
    background: gpui::Rgba,
    width: f32,
    scale: f32,
) {
    let mut previous_end = 0.0;
    let arrow_length = 11.0 * (std::f32::consts::PI / 8.0).cos();
    let arrow_half_width = 11.0 * (std::f32::consts::PI / 8.0).sin();
    for part in &composition.glyphs {
        let origin = match part.placement {
            EndpointMarkerPlacement::Endpoint => 0.0,
            EndpointMarkerPlacement::AfterPrevious => previous_end,
        };
        let offset = |forward: f32, side: f32| {
            (
                endpoint.0
                    + ((origin + forward) * inward_direction.0 - side * inward_direction.1) * scale,
                endpoint.1
                    + ((origin + forward) * inward_direction.1 + side * inward_direction.0) * scale,
            )
        };
        use EndpointMarkerGlyph::*;
        let extent = match part.glyph {
            OpenArrow => {
                stroke_line(
                    window,
                    offset(arrow_length, arrow_half_width),
                    offset(0.0, 0.0),
                    bounds,
                    stroke,
                    width,
                );
                stroke_line(
                    window,
                    offset(0.0, 0.0),
                    offset(arrow_length, -arrow_half_width),
                    bounds,
                    stroke,
                    width,
                );
                arrow_length
            }
            SolidArrow | BlankArrow | OpenTriangle | FilledTriangle => {
                let size = if matches!(part.glyph, OpenTriangle | FilledTriangle) {
                    2.0
                } else {
                    1.0
                };
                polygon(
                    window,
                    &[
                        offset(0.0, 0.0),
                        offset(arrow_length * size, arrow_half_width * size),
                        offset(arrow_length * size, -arrow_half_width * size),
                    ],
                    bounds,
                    stroke,
                    background,
                    matches!(part.glyph, SolidArrow | FilledTriangle),
                    width,
                );
                arrow_length * size
            }
            OpenDiamond | FilledDiamond => {
                polygon(
                    window,
                    &[
                        offset(0.0, 0.0),
                        offset(arrow_length, arrow_half_width),
                        offset(22.0, 0.0),
                        offset(arrow_length, -arrow_half_width),
                    ],
                    bounds,
                    stroke,
                    background,
                    part.glyph == FilledDiamond,
                    width,
                );
                22.0
            }
            Cross => {
                let diagonal = 5.0 / std::f32::consts::SQRT_2;
                for side in [-1.0, 1.0] {
                    stroke_line(
                        window,
                        offset(7.0 - diagonal, side * diagonal),
                        offset(7.0 + diagonal, -side * diagonal),
                        bounds,
                        stroke,
                        width,
                    );
                }
                7.0 + diagonal
            }
            CrowfootOne | CrowfootMany | CrowfootZeroOne | CrowfootZeroMany => {
                if matches!(part.glyph, CrowfootMany | CrowfootZeroMany) {
                    for side in [-7.0, 7.0] {
                        stroke_line(
                            window,
                            offset(14.0, 0.0),
                            offset(0.0, side),
                            bounds,
                            stroke,
                            width,
                        );
                    }
                }
                if matches!(part.glyph, CrowfootZeroOne | CrowfootZeroMany) {
                    let vertices = (0..32)
                        .map(|step| {
                            let angle = step as f32 * std::f32::consts::TAU / 32.0;
                            offset(22.0 + 7.0 * angle.cos(), 7.0 * angle.sin())
                        })
                        .collect::<Vec<_>>();
                    polygon(
                        window,
                        &vertices,
                        bounds,
                        stroke,
                        rgb(0xffffff),
                        false,
                        width,
                    );
                }
                if matches!(part.glyph, CrowfootOne | CrowfootZeroOne) {
                    stroke_line(
                        window,
                        offset(7.0, -7.0),
                        offset(7.0, 7.0),
                        bounds,
                        stroke,
                        width,
                    );
                }
                if matches!(part.glyph, CrowfootOne | CrowfootMany) {
                    stroke_line(
                        window,
                        offset(14.0, -7.0),
                        offset(14.0, 7.0),
                        bounds,
                        stroke,
                        width,
                    );
                    14.0
                } else {
                    29.0
                }
            }
            OpenCircle | FilledCircle | Dot | Containment => {
                let radius = if part.glyph == Dot { 3.0 } else { 7.0 };
                let vertices = (0..32)
                    .map(|step| {
                        let angle = step as f32 * std::f32::consts::TAU / 32.0;
                        offset(radius + radius * angle.cos(), radius * angle.sin())
                    })
                    .collect::<Vec<_>>();
                polygon(
                    window,
                    &vertices,
                    bounds,
                    stroke,
                    background,
                    matches!(part.glyph, FilledCircle | Dot),
                    width,
                );
                if part.glyph == Containment {
                    stroke_line(
                        window,
                        offset(0.0, 0.0),
                        offset(radius * 2.0, 0.0),
                        bounds,
                        stroke,
                        width,
                    );
                    stroke_line(
                        window,
                        offset(radius, -radius),
                        offset(radius, radius),
                        bounds,
                        stroke,
                        width,
                    );
                }
                radius * 2.0
            }
        };
        previous_end = origin + extent;
    }
}
impl RenderOnce for UmlRelationGlyph {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let palette = Palette::current(cx);
        let appearance = self.appearance;
        let stroke = appearance.stroke(palette, self.selected);
        let background = rgb(palette.canvas);
        let RelationshipStyle {
            source_marker,
            target_marker,
            line_mode,
            middle_marker,
        } = self.style;
        let from = self.from;
        let to = self.to;
        let segments = if self.segments.is_empty() {
            vec![UmlLineSegment::Line { to }]
        } else {
            self.segments
        };
        let width = appearance.width(
            if self.selected { 2.6 } else { 1.8 },
            self.scale,
            self.selected,
        );
        let communication_message = self.communication_message;
        let endpoint_markers = self.endpoint_markers;
        let dashes = appearance.resolve_stroke_pattern(
            if communication_message {
                LineMode::Solid
            } else {
                line_mode
            },
            self.scale,
        );
        let text_color = appearance.text(palette.text);
        let font_size = appearance.font_size(12.0, self.scale);
        let (source_anchor, target_anchor, middle, middle_direction) =
            path_label_anchors(from, &segments, self.scale);
        let labels = self.labels;
        let selected = self.selected;
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
                if !dashes.is_empty() {
                    builder = builder.dash_array(&dashes);
                }
                builder.move_to(absolute(from));
                let mut last = from;
                let (initial_tangent, final_tangent) = endpoint_directions(from, &segments);
                for segment in &segments {
                    match *segment {
                        UmlLineSegment::Line { to: next } => {
                            builder.line_to(absolute(next));
                            last = next;
                        }
                        UmlLineSegment::Cubic {
                            control1,
                            control2,
                            to: next,
                        } => {
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
                let resolve_link_marker = |marker| {
                    if !communication_message {
                        return marker;
                    }
                    match marker {
                        RelationshipMarker::FilledCircle
                        | RelationshipMarker::OpenArrowAndCircle
                        | RelationshipMarker::FilledTriangleAndCircle => {
                            RelationshipMarker::FilledCircle
                        }
                        _ => RelationshipMarker::None,
                    }
                };
                if let Some((source, target)) = &endpoint_markers {
                    paint_marker_composition(
                        window,
                        source,
                        from,
                        initial_tangent,
                        bounds,
                        stroke,
                        background,
                        width,
                        scale,
                    );
                    paint_marker_composition(
                        window,
                        target,
                        last,
                        (-final_tangent.0, -final_tangent.1),
                        bounds,
                        stroke,
                        background,
                        width,
                        scale,
                    );
                } else {
                    paint_marker(
                        window,
                        resolve_link_marker(source_marker),
                        from,
                        initial_tangent,
                        bounds,
                        stroke,
                        background,
                        width,
                        scale,
                    );
                    paint_marker(
                        window,
                        resolve_link_marker(target_marker),
                        last,
                        (-final_tangent.0, -final_tangent.1),
                        bounds,
                        stroke,
                        background,
                        width,
                        scale,
                    );
                }
                if middle_marker != RelationshipMarker::None {
                    paint_marker(
                        window,
                        middle_marker,
                        middle,
                        (-middle_direction.0, -middle_direction.1),
                        bounds,
                        stroke,
                        background,
                        width,
                        scale,
                    );
                }
                if communication_message {
                    let direction = tangent(from, last);
                    let center = (middle.0 - 20.0 * scale, middle.1 - 18.0 * scale);
                    let tail = (
                        center.0 - direction.0 * 11.0 * scale,
                        center.1 - direction.1 * 11.0 * scale,
                    );
                    let head = (
                        center.0 + direction.0 * 11.0 * scale,
                        center.1 + direction.1 * 11.0 * scale,
                    );
                    stroke_line(window, tail, head, bounds, stroke, width);
                    paint_marker(
                        window,
                        RelationshipMarker::OpenArrow,
                        head,
                        (-direction.0, -direction.1),
                        bounds,
                        stroke,
                        background,
                        width,
                        scale * 0.6,
                    );
                }
            },
        )
        .size_full();
        div()
            .relative()
            .map(|element| appearance.apply_surface(element, scale))
            .when_some(appearance.0.font_family.clone(), |element, family| {
                element.font_family(family)
            })
            .size_full()
            .child(glyph)
            .when_some(labels, |element, labels| {
                let type_label = labels.type_label;
                element
                    .when(selected, |element| {
                        element.child(label_block(
                            LabelAnchor {
                                position: (middle.0, middle.1 + 8.0 * scale),
                                right_aligned: false,
                                above: false,
                            },
                            [type_label],
                            text_color,
                            font_size,
                            &appearance,
                        ))
                    })
                    .child(label_block(
                        source_anchor,
                        labels
                            .source_labels
                            .role
                            .into_iter()
                            .chain(labels.source_labels.multiplicity),
                        text_color,
                        font_size,
                        &appearance,
                    ))
                    .child(label_block(
                        target_anchor,
                        labels
                            .target_labels
                            .role
                            .into_iter()
                            .chain(labels.target_labels.multiplicity),
                        text_color,
                        font_size,
                        &appearance,
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
                                text_color,
                                font_size,
                                &appearance,
                            ))
                        },
                    )
            })
    }
}
