use super::*;
use crate::{
    AssociationEndConvention, EndpointMarkerBinding, EndpointMarkerSource, EndpointMarkerTarget,
    PathSegment, Point, RelationshipEndpointPresentation, RelationshipRouting, ViewEndpoints,
};

pub(super) fn take_path(residual: &mut BTreeMap<String, Value>) -> Option<Geometry> {
    let points = read_points(residual.get("points")?.as_str()?, residual.get("lineStyle"))?;
    let (start, rest) = points.split_first()?;
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

fn read_points(text: &str, line_style: Option<&Value>) -> Option<Vec<Point>> {
    // Curved/rounded source routes cannot be flattened into straight segments.
    if line_style.is_some_and(|value| !matches!(value.as_i64(), Some(0 | 1))) {
        return None;
    }
    let points = text
        .split(';')
        .map(|pair| {
            let (x, y) = pair.split_once(':')?;
            Some(Point {
                x: read_coordinate(x)?,
                y: read_coordinate(y)?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    (points.len() >= 2).then_some(points)
}

fn read_coordinate(text: &str) -> Option<f64> {
    let value: f64 = text.parse().ok()?;
    (value.is_finite() && decimal_key(text) == decimal_key(&value.to_string())).then_some(value)
}

pub(super) fn take_endpoints(residual: &mut BTreeMap<String, Value>) -> Option<ViewEndpoints> {
    let source_view_id = take_reference(residual, "tail");
    let target_view_id = take_reference(residual, "head");
    (source_view_id.is_some() || target_view_id.is_some()).then_some(ViewEndpoints {
        source_view_id,
        target_view_id,
    })
}

pub(super) fn export_path(
    view: &View,
    original: Option<&mdj::Object<'_>>,
    object: &mut Map<String, Value>,
) -> Result<bool, MdjError> {
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
    // Source bytes only supply spelling after the same import validation. The
    // authored geometry remains authoritative, even if provenance was replaced.
    let original_points = original.and_then(|original| {
        let text = original.properties.get("points")?.as_str()?;
        if read_points(text, original.properties.get("lineStyle"))?.len() != points.len() {
            return None;
        }
        text.split(';')
            .map(|pair| pair.split_once(':'))
            .collect::<Option<Vec<_>>>()
    });
    insert_mapped(
        object,
        "points",
        Value::String(
            points
                .iter()
                .enumerate()
                .map(|(position, point)| {
                    let original = original_points
                        .as_ref()
                        .and_then(|points| points.get(position));
                    format!(
                        "{}:{}",
                        export_coordinate(point.x, original.map(|(x, _)| *x)),
                        export_coordinate(point.y, original.map(|(_, y)| *y)),
                    )
                })
                .collect::<Vec<_>>()
                .join(";"),
        ),
    )?;
    Ok(true)
}

fn export_coordinate(value: f64, original: Option<&str>) -> String {
    original
        .filter(|text| {
            // Ordinary f64 equality would replay the old spelling after an
            // authored change between positive and negative zero.
            read_coordinate(text).is_some_and(|previous| previous.to_bits() == value.to_bits())
        })
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
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
    let canonical_associations = project
        .relations
        .iter()
        .filter(|relation| relation.kind == "uml.association")
        .map(|relation| relation.id.clone())
        .collect::<BTreeSet<_>>();
    fn bind(
        views: &mut [View],
        index: &mdj::Index<'_>,
        canonical_associations: &BTreeSet<String>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
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
                            Some(if canonical_associations.contains(object.id) {
                                AssociationEndConvention::Uml
                            } else {
                                AssociationEndConvention::StarUml
                            });
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
            bind(&mut view.children, index, canonical_associations, diagnostics);
        }
    }
    for diagram in &mut project.diagrams {
        bind(&mut diagram.views, index, &canonical_associations, diagnostics);
    }
}
