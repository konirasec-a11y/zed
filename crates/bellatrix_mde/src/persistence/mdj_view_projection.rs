use super::*;
use crate::{
    AssociationEndConvention, EndpointMarkerBinding, EndpointMarkerSource, EndpointMarkerTarget,
    PathSegment, Point, RelationshipEndpointPresentation, RelationshipRouting, ViewEndpoints,
};

pub(super) fn take_path(residual: &mut BTreeMap<String, Value>) -> Option<Geometry> {
    // Curved/rounded source routes cannot be flattened into straight segments.
    if residual
        .get("lineStyle")
        .is_some_and(|value| !matches!(value.as_i64(), Some(0 | 1)))
    {
        return None;
    }
    let text = residual.get("points")?.as_str()?;
    let points = text
        .split(';')
        .map(|pair| {
            let (x, y) = pair.split_once(':')?;
            let coordinate = |text: &str| {
                let value: f64 = text.parse().ok()?;
                (value.is_finite() && decimal_key(text) == decimal_key(&value.to_string()))
                    .then_some(value)
            };
            Some(Point {
                x: coordinate(x)?,
                y: coordinate(y)?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let (start, rest) = points.split_first()?;
    if rest.is_empty() {
        return None;
    }
    let geometry = Geometry::Path {
        start: *start,
        segments: rest
            .iter()
            .map(|point| PathSegment::Line { to: *point })
            .collect(),
    };
    residual.remove("points");
    Some(geometry)
}

pub(super) fn take_endpoints(residual: &mut BTreeMap<String, Value>) -> Option<ViewEndpoints> {
    let source_view_id = take_reference(residual, "tail");
    let target_view_id = take_reference(residual, "head");
    (source_view_id.is_some() || target_view_id.is_some()).then_some(ViewEndpoints {
        source_view_id,
        target_view_id,
    })
}

pub(super) fn export_path(view: &View, object: &mut Map<String, Value>) -> Result<bool, MdjError> {
    let Geometry::Path { start, segments } = &view.geometry else {
        return Ok(false);
    };
    if segments.is_empty() {
        return Err(failure(&view.id, "MDJ path requires at least two points"));
    }
    let mut points = vec![*start];
    for segment in segments {
        let PathSegment::Line { to } = segment else {
            return Err(failure(
                &view.id,
                "MDJ points do not encode cubic path control points losslessly",
            ));
        };
        points.push(*to);
    }
    if points
        .iter()
        .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return Err(failure(&view.id, "non-finite MDJ path coordinate"));
    }
    insert_mapped(
        object,
        "points",
        Value::String(
            points
                .iter()
                .map(|point| format!("{}:{}", point.x, point.y))
                .collect::<Vec<_>>()
                .join(";"),
        ),
    )?;
    Ok(true)
}

pub(super) fn export_endpoints(
    view: &View,
    object: &mut Map<String, Value>,
) -> Result<(), MdjError> {
    if let Some(endpoints) = &view.endpoints {
        for (field, id) in [
            ("tail", &endpoints.source_view_id),
            ("head", &endpoints.target_view_id),
        ] {
            if let Some(id) = id {
                insert_mapped(object, field, serde_json::json!({"$ref": id}))?;
            }
        }
    }
    Ok(())
}

pub(super) fn bind_markers(
    index: &mdj::Index<'_>,
    project: &mut Project,
    diagnostics: &mut Vec<Diagnostic>,
) {
    fn bind(views: &mut [View], index: &mdj::Index<'_>, diagnostics: &mut Vec<Diagnostic>) {
        for view in views {
            if view.presentation.kind.as_deref() == Some("UMLAssociationView") {
                if let Some(object) = view
                    .subject
                    .as_deref()
                    .and_then(|id| index.object(id))
                    .filter(|object| object.type_name == "UMLAssociation")
                {
                    let ends = ["end1", "end2"].map(|field| {
                        object
                            .properties
                            .get(field)
                            .and_then(|value| value.get("_id"))
                            .and_then(Value::as_str)
                    });
                    if let [Some(source), Some(target)] = ends {
                        let binding = |id: &str| EndpointMarkerBinding {
                            target: EndpointMarkerTarget::ModelEnd { id: id.into() },
                            marker: EndpointMarkerSource::Model,
                        };
                        view.presentation.relationship_endpoints =
                            Some(RelationshipEndpointPresentation {
                                source: binding(source),
                                target: binding(target),
                            });
                        view.style.uml.association_end_convention =
                            Some(AssociationEndConvention::StarUml);
                    } else {
                        diagnostics.push(Diagnostic { path: view.id.clone(), message: "association view preserved without end markers: source end identities are unavailable".into() });
                    }
                }
            }
            if let Some(source) = &view.presentation.source {
                for field in ["points", "tail", "head"] {
                    if source.residual.contains_key(field) {
                        diagnostics.push(Diagnostic { path: format!("{}/{field}", view.id), message: "view field retained as source residue: this encoding has no exact path/attachment projection".into() });
                    }
                }
            }
            if matches!(view.geometry, Geometry::Path { .. }) {
                view.style.routing = Some(RelationshipRouting::Manual);
            }
            bind(&mut view.children, index, diagnostics);
        }
    }
    for diagram in &mut project.diagrams {
        bind(&mut diagram.views, index, diagnostics);
    }
}
