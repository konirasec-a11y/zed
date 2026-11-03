use crate::{Diagram, Geometry, PathSegment, Point};

pub fn resolve_visual_center(geometry: &Geometry) -> Option<Point> {
    match geometry {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => Some(Point {
            x: x + width / 2.0,
            y: y + height / 2.0,
        }),
        Geometry::Ellipse { cx, cy, .. } => Some(Point { x: *cx, y: *cy }),
        Geometry::Polygon { points } => {
            let count = points.len();
            if count == 0 {
                return None;
            }
            Some(Point {
                x: points.iter().map(|point| point.x).sum::<f64>() / count as f64,
                y: points.iter().map(|point| point.y).sum::<f64>() / count as f64,
            })
        }
        Geometry::Path { start, segments } => {
            let index = segments.len() / 2;
            let from = if index == 0 {
                *start
            } else {
                match segments[index - 1] {
                    PathSegment::Line { to } | PathSegment::Cubic { to, .. } => to,
                }
            };
            Some(match segments.get(index) {
                Some(PathSegment::Line { to }) => Point {
                    x: (from.x + to.x) / 2.0,
                    y: (from.y + to.y) / 2.0,
                },
                Some(PathSegment::Cubic {
                    control1,
                    control2,
                    to,
                }) => Point {
                    x: (from.x + 3.0 * control1.x + 3.0 * control2.x + to.x) / 8.0,
                    y: (from.y + 3.0 * control1.y + 3.0 * control2.y + to.y) / 8.0,
                },
                None => from,
            })
        }
    }
}

/// Refresh stored endpoint positions without changing intermediate route segments.
pub fn reroute_visual_drafts(diagram: &mut Diagram) {
    let mut endpoints: std::collections::HashMap<String, crate::relationships::Endpoint> = diagram
        .views
        .iter()
        .filter_map(|view| {
            let center = resolve_visual_center(&view.geometry)?;
            let (width, height) = match &view.geometry {
                Geometry::Rect { width, height, .. } => (*width, *height),
                Geometry::Ellipse { rx, ry, .. } => (rx * 2.0, ry * 2.0),
                Geometry::Polygon { points } => {
                    let (left, right, top, bottom) = points.iter().fold(
                        (
                            f64::INFINITY,
                            f64::NEG_INFINITY,
                            f64::INFINITY,
                            f64::NEG_INFINITY,
                        ),
                        |(left, right, top, bottom), point| {
                            (
                                left.min(point.x),
                                right.max(point.x),
                                top.min(point.y),
                                bottom.max(point.y),
                            )
                        },
                    );
                    (right - left, bottom - top)
                }
                Geometry::Path { .. } => (0.0, 0.0),
            };
            Some((
                view.id.clone(),
                crate::relationships::Endpoint {
                    center,
                    width,
                    height,
                },
            ))
        })
        .collect();
    let order = resolve_routing_order(diagram);
    for index in order {
        let view = &mut diagram.views[index];
        let Some(draft) = &view.draft else { continue };
        let Some(source) = draft
            .source_view_id
            .as_ref()
            .and_then(|id| endpoints.get(id))
        else {
            continue;
        };
        let Some(target) = draft
            .target_view_id
            .as_ref()
            .and_then(|id| endpoints.get(id))
        else {
            continue;
        };
        if let Geometry::Path { start, segments } = &mut view.geometry {
            if segments.is_empty() {
                segments.push(PathSegment::Line { to: target.center });
            }
            let first_direction = match segments.first() {
                Some(PathSegment::Line { to }) if segments.len() > 1 => *to,
                Some(PathSegment::Cubic { control1, .. }) => *control1,
                _ => target.center,
            };
            let last_direction = if segments.len() > 1 {
                match &segments[segments.len() - 2] {
                    PathSegment::Line { to } | PathSegment::Cubic { to, .. } => *to,
                }
            } else {
                source.center
            };
            *start = source.anchor_toward(first_direction);
            if let Some(segment) = segments.last_mut() {
                match segment {
                    PathSegment::Line { to } | PathSegment::Cubic { to, .. } => {
                        *to = target.anchor_toward(last_direction)
                    }
                }
            }
            if let Some(center) = resolve_visual_center(&view.geometry) {
                endpoints.insert(
                    view.id.clone(),
                    crate::relationships::Endpoint {
                        center,
                        width: 0.0,
                        height: 0.0,
                    },
                );
            }
        }
    }
}

fn resolve_routing_order(diagram: &Diagram) -> Vec<usize> {
    let lookup: std::collections::HashMap<_, _> = diagram
        .views
        .iter()
        .enumerate()
        .map(|(index, view)| (view.id.as_str(), index))
        .collect();
    let mut order = Vec::new();
    let mut visited = std::collections::HashSet::new();
    for index in 0..diagram.views.len() {
        let mut stack = vec![(index, false)];
        while let Some((index, complete)) = stack.pop() {
            if complete {
                order.push(index);
                continue;
            }
            if !visited.insert(index) {
                continue;
            }
            stack.push((index, true));
            if let Some(draft) = &diagram.views[index].draft {
                for id in [&draft.source_view_id, &draft.target_view_id]
                    .into_iter()
                    .flatten()
                {
                    if let Some(dependency) = lookup.get(id.as_str()) {
                        stack.push((*dependency, false));
                    }
                }
            }
        }
    }
    order
}
