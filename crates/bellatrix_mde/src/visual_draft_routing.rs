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
            let self_loop_offset = if draft
                .properties
                .get("self_message")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
            {
                match segments.first() {
                    Some(PathSegment::Line { to }) => Some(to.x - start.x),
                    _ => None,
                }
            } else {
                None
            };
            *start = resolve_authored_anchor(&draft.properties, "source_anchor", *source)
                .unwrap_or_else(|| source.anchor_toward(first_direction));
            if let Some(segment) = segments.last_mut() {
                match segment {
                    PathSegment::Line { to } | PathSegment::Cubic { to, .. } => {
                        *to = resolve_authored_anchor(&draft.properties, "target_anchor", *target)
                            .unwrap_or_else(|| target.anchor_toward(last_direction))
                    }
                }
            }
            if let (
                Some(offset),
                [
                    PathSegment::Line { to: first },
                    PathSegment::Line { to: second },
                    PathSegment::Line { to: end },
                ],
            ) = (self_loop_offset, segments.as_mut_slice())
            {
                first.x = start.x + offset;
                first.y = start.y;
                second.x = first.x;
                second.y = end.y;
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

fn resolve_authored_anchor(
    properties: &std::collections::BTreeMap<String, serde_json::Value>,
    key: &str,
    endpoint: crate::relationships::Endpoint,
) -> Option<Point> {
    let anchor = properties.get(key)?;
    let x = anchor.get("x")?.as_f64()?;
    let y = anchor.get("y")?.as_f64()?;
    (x.is_finite() && y.is_finite()).then_some(Point {
        x: endpoint.center.x + (x - 0.5) * endpoint.width,
        y: endpoint.center.y + (y - 0.5) * endpoint.height,
    })
}

/// Moving a visual message changes its drawn event heights, not semantic occurrences.
pub(crate) fn translate_message_anchors(diagram: &mut Diagram, view_id: &str, dy: f64) {
    let heights: std::collections::HashMap<_, _> = diagram
        .views
        .iter()
        .filter_map(|view| match view.geometry {
            Geometry::Rect { height, .. } if height > 0.0 => Some((view.id.clone(), height)),
            _ => None,
        })
        .collect();
    let Some(draft) = diagram
        .views
        .iter_mut()
        .find(|view| view.id == view_id)
        .and_then(|view| view.draft.as_mut())
        .filter(|draft| draft.notation_kind == "uml.message")
    else {
        return;
    };
    for (key, endpoint) in [
        ("source_anchor", &draft.source_view_id),
        ("target_anchor", &draft.target_view_id),
    ] {
        let Some(height) = endpoint.as_ref().and_then(|id| heights.get(id)) else {
            continue;
        };
        if let Some(anchor) = draft
            .properties
            .get_mut(key)
            .and_then(serde_json::Value::as_object_mut)
            && let Some(y) = anchor.get("y").and_then(serde_json::Value::as_f64)
        {
            let translated = y + dy / height;
            if translated.is_finite() {
                anchor.insert("y".into(), serde_json::json!(translated));
            }
        }
    }
}
