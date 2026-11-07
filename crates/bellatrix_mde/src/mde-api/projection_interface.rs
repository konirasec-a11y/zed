use crate::{Diagram, Geometry, InterfaceConnectionRole, PathSegment, StereotypeDisplay, View};
use crate::contracts::protocol::{
    command::Point,
    projection::{InterfaceIconGlyph, InterfaceIconProjection},
};

pub(super) fn project(
    section: &crate::ModelingSection,
    diagram: &Diagram,
    view: &View,
) -> Option<InterfaceIconProjection> {
    let element = view
        .subject
        .as_deref()
        .and_then(|id| section.elements.iter().find(|element| element.id == id));
    let kind = view
        .presentation
        .notation_kind
        .as_deref()
        .or_else(|| element.map(|element| element.kind.as_str()))
        .or_else(|| {
            view.draft
                .as_ref()
                .map(|draft| draft.notation_kind.as_str())
        });
    if kind.and_then(crate::UmlType::from_kind) != Some(crate::UmlType::Interface)
        || matches!(
            view.style.uml.stereotype_display,
            Some(StereotypeDisplay::Hidden | StereotypeDisplay::Label)
        )
    {
        return None;
    }
    let mut projection = InterfaceIconProjection::default();
    let result: Result<Vec<InterfaceIconGlyph>, String> = (|| {
        let (left, top, width, height) = bounds(view)?;
        let contents = element
            .map(|element| crate::classifier_contents(section, &element.id))
            .unwrap_or_default();
        let layout = crate::interface_view_layout(view, contents)
            .ok_or("interface bounds do not establish a native icon layout")?;
        projection.label_bounds = [
            0.0,
            layout.label_top,
            width,
            (height - layout.label_top).max(0.0),
        ];
        let center = Point {
            x: left + layout.center.x,
            y: top + layout.center.y,
        };
        let rectangle = [
            center.x - layout.radius,
            center.y - layout.radius,
            center.x + layout.radius,
            center.y + layout.radius,
        ];
        let draft_role = view
            .draft
            .as_ref()
            .and_then(|draft| draft.properties.get("interface_presentation"))
            .and_then(serde_json::Value::as_str);
        let mut required = Vec::new();
        let mut provided = draft_role == Some("provided");
        for connection in diagram.all_views() {
            if connection
                .presentation
                .interface_connection
                .as_ref()
                .is_some_and(|binding| binding.target_unverified)
            {
                projection.diagnostics.push(format!(
                    "View {}: interface connection {} has an unresolved target",
                    view.id, connection.id
                ));
                continue;
            }
            let relation = connection.subject.as_deref().and_then(|subject| {
                section
                    .relations
                    .iter()
                    .find(|relation| relation.id == subject)
            });
            let attached = connection
                .endpoints
                .as_ref()
                .and_then(|endpoints| endpoints.target_view_id.as_deref())
                .map_or_else(
                    || {
                        relation.is_some_and(|relation| {
                            Some(relation.to.as_str()) == view.subject.as_deref()
                        })
                    },
                    |target| target == view.id,
                );
            if !attached {
                continue;
            }
            let role = connection
                .presentation
                .interface_connection
                .as_ref()
                .map(|binding| binding.role)
                .or_else(|| {
                    relation.and_then(|relation| match relation.kind.as_str() {
                        "uml.dependency" => Some(InterfaceConnectionRole::Required),
                        "uml.interface_realization" => Some(InterfaceConnectionRole::Provided),
                        _ => None,
                    })
                });
            match role {
                Some(InterfaceConnectionRole::Required) => required.push(connection),
                Some(InterfaceConnectionRole::Provided) => provided = true,
                None => {}
            }
        }
        let local_center = Point {
            x: layout.center.x,
            y: layout.center.y,
        };
        let mut glyphs = Vec::new();
        for connection in &required {
            let endpoint = last_point(connection)?;
            let boundary = junction(rectangle, center, endpoint).ok_or_else(|| {
                format!(
                    "interface socket direction is unresolved for {}",
                    connection.id
                )
            })?;
            let angle = (boundary.y - center.y).atan2(boundary.x - center.x);
            glyphs.push(InterfaceIconGlyph::Socket {
                center: local_center,
                radius: layout.radius,
                start_angle: angle - std::f64::consts::FRAC_PI_2,
                sweep_angle: std::f64::consts::PI,
            });
        }
        if required.is_empty() && draft_role == Some("required") {
            glyphs.push(InterfaceIconGlyph::Socket {
                center: local_center,
                radius: layout.radius,
                start_angle: std::f64::consts::FRAC_PI_2,
                sweep_angle: std::f64::consts::PI,
            });
        } else if required.is_empty() || provided {
            let radius = layout.radius - if required.is_empty() { 0.0 } else { 3.0 };
            if radius <= 0.0 {
                return Err("interface ball-and-socket inset exceeds its bounds".into());
            }
            glyphs.push(InterfaceIconGlyph::Ball {
                center: local_center,
                radius,
            });
        }
        Ok(glyphs)
    })();
    match result {
        Ok(glyphs) => projection.glyphs = glyphs,
        Err(error) => projection
            .diagnostics
            .push(format!("View {}: {error}", view.id)),
    }
    Some(projection)
}

fn bounds(view: &View) -> Result<(f64, f64, f64, f64), String> {
    let values = match &view.geometry {
        Geometry::Layout { bounds } => (
            *bounds.x.value().ok_or("layout left is unresolved")?,
            *bounds.y.value().ok_or("layout top is unresolved")?,
            bounds
                .width
                .value()
                .and_then(crate::LayoutExtent::resolved)
                .ok_or("layout width is unresolved")?,
            bounds
                .height
                .value()
                .and_then(crate::LayoutExtent::resolved)
                .ok_or("layout height is unresolved")?,
        ),
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => (*x, *y, *width, *height),
        _ => return Err("interface icon requires authored rectangular layout bounds".into()),
    };
    if ![values.0, values.1, values.2, values.3]
        .iter()
        .all(|value| value.is_finite())
        || values.2 < 0.0
        || values.3 < 0.0
    {
        return Err("interface icon bounds are not finite nonnegative dimensions".into());
    }
    Ok(values)
}

fn last_point(view: &View) -> Result<Point, String> {
    let Geometry::Path { segments, .. } = &view.geometry else {
        return Err(format!(
            "interface connection {} has no supported stored path",
            view.id
        ));
    };
    let Some(PathSegment::Line { to } | PathSegment::Cubic { to, .. }) = segments.last() else {
        return Err(format!(
            "interface connection {} has no terminal point",
            view.id
        ));
    };
    if !to.x.is_finite() || !to.y.is_finite() {
        return Err(format!(
            "interface connection {} has a non-finite terminal point",
            view.id
        ));
    }
    Ok(Point { x: to.x, y: to.y })
}

fn junction(rectangle: [f64; 4], center: Point, endpoint: Point) -> Option<Point> {
    let [left, top, right, bottom] = rectangle;
    let boundary = if center.x == endpoint.x || center.y == endpoint.y {
        if left < endpoint.x && endpoint.x < right {
            Point {
                x: endpoint.x,
                y: if top >= endpoint.y { top } else { bottom },
            }
        } else if top < endpoint.y && endpoint.y < bottom {
            Point {
                x: if left >= endpoint.x { left } else { right },
                y: endpoint.y,
            }
        } else if left == endpoint.x || right == endpoint.x {
            Point {
                x: endpoint.x,
                y: if top >= endpoint.y { top } else { bottom },
            }
        } else if top == endpoint.y || bottom == endpoint.y {
            Point {
                x: if left >= endpoint.x { left } else { right },
                y: endpoint.y,
            }
        } else {
            return None;
        }
    } else {
        let slope = (endpoint.y - center.y) / (endpoint.x - center.x);
        [
            Point {
                x: left,
                y: slope * (left - center.x) + center.y,
            },
            Point {
                x: right,
                y: slope * (right - center.x) + center.y,
            },
            Point {
                x: (top - center.y) / slope + center.x,
                y: top,
            },
            Point {
                x: (bottom - center.y) / slope + center.x,
                y: bottom,
            },
        ]
        .into_iter()
        .find(|point| {
            point.x >= left
                && point.x <= right
                && point.y >= top
                && point.y <= bottom
                && point.x >= center.x.min(endpoint.x)
                && point.x <= center.x.max(endpoint.x)
                && point.y >= center.y.min(endpoint.y)
                && point.y <= center.y.max(endpoint.y)
        })?
    };
    (boundary.x.is_finite()
        && boundary.y.is_finite()
        && (boundary.x != center.x || boundary.y != center.y))
        .then_some(boundary)
}
