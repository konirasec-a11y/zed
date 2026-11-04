use crate::{Diagram, Geometry, InterfaceConnectionRole, PathSegment, StereotypeDisplay, View};
use bellatrix_mde_protocol::{
    command::Point,
    projection::{InterfaceIconGlyph, InterfaceIconProjection},
};

pub(super) fn project(diagram: &Diagram, view: &View) -> Option<InterfaceIconProjection> {
    let binding = view.presentation.interface_icon.as_ref()?;
    let mut projection = InterfaceIconProjection::default();
    let result = (|| {
        if !matches!(
            view.style.uml.stereotype_display,
            Some(StereotypeDisplay::Icon | StereotypeDisplay::IconLabel)
        ) {
            return Err("interface presentation is not an explicitly selected icon; no substitute rectangle was drawn".to_owned());
        }
        let (left, top, width, height) = bounds(view)?;
        let compartment_id = binding
            .label_compartment
            .value()
            .ok_or("interface icon label-compartment identity is unresolved")?;
        let compartment = view
            .children
            .iter()
            .find(|child| child.id == *compartment_id)
            .ok_or("interface icon label compartment is not a direct child View")?;
        if !super::visible_in_preview(compartment) {
            return Err("interface icon label compartment is not visibly represented".into());
        }
        let (_, compartment_top, _, _) = bounds(compartment)?;
        // In an arranged classifier snapshot the first visible compartment
        // begins at top + icon height. This uses its stored boundary, never
        // estimates font metrics or invents a 24-pixel default icon.
        let diameter = compartment_top - top;
        if !diameter.is_finite() || diameter <= 0.0 || diameter > width || diameter > height {
            return Err(
                "interface icon compartment boundary does not establish a fitting square".into(),
            );
        }
        let rectangle = [
            left + (width - diameter) / 2.0,
            top,
            left + (width + diameter) / 2.0,
            top + diameter,
        ];
        if rectangle.iter().any(|value| !value.is_finite()) {
            return Err("interface icon coordinates are not finite".into());
        }
        let mut required = Vec::new();
        let mut provided = false;
        // The source notation scans top-level diagram Views, not model
        // dependencies or Interface realization semantics.
        for connection in &diagram.views {
            let Some(attachment) = &connection.presentation.interface_connection else {
                continue;
            };
            if attachment.target_unverified {
                return Err(format!(
                    "interface connection {} has an unresolved target",
                    connection.id
                ));
            }
            if connection
                .endpoints
                .as_ref()
                .and_then(|endpoints| endpoints.target_view_id.as_deref())
                != Some(view.id.as_str())
            {
                continue;
            }
            match attachment.role {
                InterfaceConnectionRole::Required => required.push(connection),
                InterfaceConnectionRole::Provided => provided = true,
            }
        }
        let center = Point {
            x: ((rectangle[0] + rectangle[2]) / 2.0).floor(),
            y: ((rectangle[1] + rectangle[3]) / 2.0).floor(),
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
                center: Point {
                    x: center.x - left,
                    y: center.y - top,
                },
                radius: diameter / 2.0,
                start_angle: angle - std::f64::consts::FRAC_PI_2,
                sweep_angle: std::f64::consts::PI,
            });
        }
        if required.is_empty() || provided {
            let inset = if required.is_empty() { 0.0 } else { 3.0 };
            let radius = diameter / 2.0 - inset;
            if radius <= 0.0 {
                return Err(
                    "interface ball-and-socket inset exceeds its authored icon bounds".into(),
                );
            }
            glyphs.push(InterfaceIconGlyph::Ball {
                center: Point {
                    x: width / 2.0,
                    y: diameter / 2.0,
                },
                radius,
            });
        }
        Ok(glyphs)
    })();
    match result {
        Ok(glyphs) => {
            projection.glyphs = glyphs;
            projection.diagnostics.push(format!("View {}: interface icon uses stored compartment bounds and connection endpoints; font/edge relayout and shadow parity are not evaluated", view.id));
        }
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
