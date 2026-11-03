use crate::{Diagram, Geometry, PathSegment, Point};

fn center(geometry: &Geometry) -> Option<Point> {
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
            let end = segments
                .last()
                .map(|segment| match segment {
                    PathSegment::Line { to } | PathSegment::Cubic { to, .. } => *to,
                })
                .unwrap_or(*start);
            Some(Point {
                x: (start.x + end.x) / 2.0,
                y: (start.y + end.y) / 2.0,
            })
        }
    }
}

/// Refresh stored endpoint positions without changing intermediate route segments.
pub fn reroute_visual_drafts(diagram: &mut Diagram) {
    let endpoints: std::collections::HashMap<String, crate::relationships::Endpoint> = diagram
        .views
        .iter()
        .filter_map(|view| {
            let center = center(&view.geometry)?;
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
    for view in &mut diagram.views {
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
        }
    }
}
