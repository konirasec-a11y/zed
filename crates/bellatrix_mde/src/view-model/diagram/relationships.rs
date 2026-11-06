//! One relationship contract for model operations and diagram projections.
//! GPUI paints and hit-tests; it does not define relationship semantics.
pub use crate::relationship_notation::{
    RelationshipEndpointLabels, RelationshipMarker, RelationshipPresentation, RelationshipStyle,
    relationship_presentation, relationship_style_for_notation,
};
use crate::{Geometry, ModelingSection, PathSegment, Point, Relation, RelationshipRouting, View};
use crate::{UmlType, classifier_contents};

pub use crate::relationship_tools::ModelingRelationKind;

#[derive(Clone, Copy)]
pub(crate) struct Endpoint {
    pub(crate) center: Point,
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) circle: Option<crate::InterfaceLayout>,
}
impl Endpoint {
    pub(crate) fn anchor_toward(self, target: Point) -> Point {
        if let Some(circle) = self.circle {
            return circle.anchor_toward(target);
        }
        let delta_x = target.x - self.center.x;
        let delta_y = target.y - self.center.y;
        if delta_x.abs() < f64::EPSILON && delta_y.abs() < f64::EPSILON {
            return Point {
                x: self.center.x + self.width / 2.0,
                y: self.center.y,
            };
        }
        let horizontal = if delta_x.abs() > f64::EPSILON {
            self.width / 2.0 / delta_x.abs()
        } else {
            f64::MAX
        };
        let vertical = if delta_y.abs() > f64::EPSILON {
            self.height / 2.0 / delta_y.abs()
        } else {
            f64::MAX
        };
        let distance = horizontal.min(vertical);
        Point {
            x: self.center.x + delta_x * distance,
            y: self.center.y + delta_y * distance,
        }
    }
}

pub fn route(
    section: &ModelingSection,
    diagram_id: &str,
    from: &str,
    to: &str,
) -> Result<Geometry, String> {
    route_with_lane(section, diagram_id, from, to, None, 0.0)
}

pub(crate) fn resolve_view_endpoints(
    section: &ModelingSection,
    diagram_id: &str,
    from: &str,
    to: &str,
) -> Result<crate::ViewEndpoints, String> {
    let diagram = section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?;
    let view_id = |subject: &str| {
        diagram
            .all_views()
            .find(|view| view.subject.as_deref() == Some(subject))
            .map(|view| view.id.clone())
            .ok_or_else(|| format!("endpoint {subject} has no View in this diagram"))
    };
    Ok(crate::ViewEndpoints {
        source_view_id: Some(view_id(from)?),
        target_view_id: Some(view_id(to)?),
    })
}

pub(crate) fn route_at_views(
    section: &ModelingSection,
    diagram_id: &str,
    from: &str,
    to: &str,
    endpoints: &crate::ViewEndpoints,
) -> Result<Geometry, String> {
    route_with_lane(section, diagram_id, from, to, Some(endpoints), 0.0)
}

fn relationship_endpoint(
    section: &ModelingSection,
    diagram_id: &str,
    subject: Option<&str>,
    view_id: Option<&str>,
    attached: bool,
) -> Result<Endpoint, String> {
    let diagram = section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?;
    let view = if attached {
        let view_id = view_id.ok_or("relationship View has an unattached endpoint")?;
        diagram
            .all_views()
            .find(|view| view.id == view_id)
            .ok_or_else(|| format!("endpoint View {view_id} is missing from this diagram"))?
    } else {
        let subject = subject.ok_or("relationship endpoint requires a View attachment")?;
        diagram
            .all_views()
            .find(|view| view.subject.as_deref() == Some(subject))
            .ok_or_else(|| format!("endpoint {subject} has no View in this diagram"))?
    };
    if subject.is_some() && view.subject.as_deref() != subject {
        return Err("relationship endpoint View no longer represents its semantic endpoint".into());
    }
    let (left, top, width, height) = match &view.geometry {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } if *width > 0.0 && *height > 0.0 => (*x, *y, *width, *height),
        _ => return Err("relationship endpoint has no valid node View".to_owned()),
    };
    let element = view.subject.as_deref().and_then(|subject| {
        section
            .elements
            .iter()
            .find(|element| element.id == subject)
    });
    let kind = view
        .presentation
        .notation_kind
        .as_deref()
        .or_else(|| element.map(|element| element.kind.as_str()))
        .and_then(UmlType::from_kind);
    let contents = view
        .subject
        .as_deref()
        .map(|subject| classifier_contents(section, subject))
        .unwrap_or_default();
    if kind == Some(UmlType::Interface)
        && !matches!(
            view.style.uml.stereotype_display,
            Some(crate::StereotypeDisplay::Hidden | crate::StereotypeDisplay::Label)
        )
    {
        let mut layout = crate::interface_view_layout(view, contents)
            .ok_or("interface View has no native icon layout")?;
        layout.center.x += left;
        layout.center.y += top;
        return Ok(Endpoint {
            center: layout.center,
            width: layout.radius * 2.0,
            height: layout.radius * 2.0,
            circle: Some(layout),
        });
    }
    let authored_layout = view.presentation.internal
        || view
            .children
            .iter()
            .any(|child| child.presentation.internal);
    let height = if kind == Some(UmlType::Interface) && !authored_layout {
        height.max(contents.minimum_interface_height())
    } else {
        height
    };
    let height = element
        .filter(|_| !authored_layout)
        .map(|element| {
            let notation = crate::uml_notation(&element.kind);
            let header = crate::uml_header(
                &element.kind,
                crate::stereotype_presentation(section, element).label(),
            );
            match notation {
                crate::UmlNotation::ClassifierRectangle | crate::UmlNotation::PackageTab => {
                    height.max(header.minimum_height(notation))
                }
                _ => height,
            }
        })
        .unwrap_or(height);
    Ok(Endpoint {
        center: Point {
            x: left + width / 2.0,
            y: top + height / 2.0,
        },
        width,
        height,
        circle: None,
    })
}

fn route_with_lane(
    section: &ModelingSection,
    diagram_id: &str,
    from: &str,
    to: &str,
    endpoints: Option<&crate::ViewEndpoints>,
    lane: f64,
) -> Result<Geometry, String> {
    let source_view_id = endpoints.and_then(|endpoints| endpoints.source_view_id.as_deref());
    let target_view_id = endpoints.and_then(|endpoints| endpoints.target_view_id.as_deref());
    let source = relationship_endpoint(
        section,
        diagram_id,
        Some(from),
        source_view_id,
        endpoints.is_some(),
    )?;
    let target = relationship_endpoint(
        section,
        diagram_id,
        Some(to),
        target_view_id,
        endpoints.is_some(),
    )?;
    if endpoints.map_or(from == to, |_| source_view_id == target_view_id) {
        let start = source.anchor_toward(Point {
            x: source.center.x + source.width,
            y: source.center.y - source.height,
        });
        let end = source.anchor_toward(Point {
            x: source.center.x + source.width,
            y: source.center.y + source.height,
        });
        return Ok(Geometry::Path {
            start,
            segments: vec![PathSegment::Cubic {
                control1: Point {
                    x: start.x + 62.0,
                    y: start.y - 56.0,
                },
                control2: Point {
                    x: end.x + 62.0,
                    y: end.y + 56.0,
                },
                to: end,
            }],
        });
    }
    let same_center = (source.center.x - target.center.x).abs() < f64::EPSILON
        && (source.center.y - target.center.y).abs() < f64::EPSILON;
    // Re-anchor each parallel lane, rather than translating it outside the shape.
    let dx = target.center.x - source.center.x;
    let dy = target.center.y - source.center.y;
    let distance = dx.hypot(dy);
    let offset = if distance > f64::EPSILON {
        let direction = if from <= to { 1.0 } else { -1.0 };
        Point {
            x: -dy / distance * lane * direction,
            y: dx / distance * lane * direction,
        }
    } else {
        Point { x: 0.0, y: 0.0 }
    };
    let start = source.anchor_toward(if same_center {
        Point {
            x: source.center.x + source.width,
            y: source.center.y,
        }
    } else {
        Point {
            x: target.center.x + offset.x,
            y: target.center.y + offset.y,
        }
    });
    let to = target.anchor_toward(if same_center {
        Point {
            x: target.center.x - target.width,
            y: target.center.y,
        }
    } else {
        Point {
            x: source.center.x + offset.x,
            y: source.center.y + offset.y,
        }
    });
    if lane.abs() < f64::EPSILON || same_center {
        return Ok(Geometry::Path {
            start,
            segments: vec![PathSegment::Line { to }],
        });
    }
    let control1 = Point {
        x: start.x + (to.x - start.x) / 3.0 + offset.x * 1.25,
        y: start.y + (to.y - start.y) / 3.0 + offset.y * 1.25,
    };
    let control2 = Point {
        x: start.x + (to.x - start.x) * 2.0 / 3.0 + offset.x * 1.25,
        y: start.y + (to.y - start.y) * 2.0 / 3.0 + offset.y * 1.25,
    };
    Ok(Geometry::Path {
        start,
        segments: vec![PathSegment::Cubic {
            control1,
            control2,
            to,
        }],
    })
}

fn lane_for_relation(section: &ModelingSection, diagram_id: &str, relation: &Relation) -> f64 {
    let Some(diagram) = section.diagrams.iter().find(|d| d.id == diagram_id) else {
        return 0.0;
    };
    let peers = section
        .relations
        .iter()
        .filter(|other| {
            (other.from == relation.from && other.to == relation.to)
                || (other.from == relation.to && other.to == relation.from)
        })
        .filter(|other| {
            diagram
                .all_views()
                .any(|v| v.subject.as_deref() == Some(other.id.as_str()))
        })
        .collect::<Vec<_>>();
    if peers.len() < 2 {
        return 0.0;
    }
    let index = peers
        .iter()
        .position(|peer| peer.id == relation.id)
        .unwrap_or(0);
    (index as f64 - (peers.len() - 1) as f64 / 2.0) * 48.0
}

pub fn has_relation_view(section: &ModelingSection, diagram_id: &str, id: &str) -> bool {
    section
        .diagrams
        .iter()
        .find(|d| d.id == diagram_id)
        .is_some_and(|d| {
            d.all_views().any(|v| {
                v.subject.as_deref() == Some(id) && matches!(&v.geometry, Geometry::Path { .. })
            })
        })
}

/// Pure projection; only a revisioned command persists the resulting geometry.
pub fn projected_relationship_paths(
    section: &ModelingSection,
    diagram_id: &str,
) -> std::collections::BTreeMap<String, Geometry> {
    let Some(diagram) = section.diagrams.iter().find(|d| d.id == diagram_id) else {
        return Default::default();
    };
    section
        .relations
        .iter()
        .filter_map(|relation| {
            let view = diagram
                .all_views()
                .find(|v| v.subject.as_deref() == Some(relation.id.as_str()))?;
            let geometry = projected_relationship_view_path(section, diagram_id, relation, view)?;
            Some((relation.id.clone(), geometry))
        })
        .collect()
}

fn projected_relationship_view_path(
    section: &ModelingSection,
    diagram_id: &str,
    relation: &Relation,
    view: &View,
) -> Option<Geometry> {
    if !matches!(view.geometry, Geometry::Path { .. }) {
        return None;
    }
    match view.style.routing {
        Some(RelationshipRouting::Manual) => Some(view.geometry.clone()),
        Some(routing @ (RelationshipRouting::Automatic | RelationshipRouting::Straight)) => {
            route_for_routing(
                section,
                diagram_id,
                relation,
                view.endpoints.as_ref(),
                routing,
            )
            .ok()
        }
        Some(RelationshipRouting::Curve) | None => reanchor_path(
            section,
            diagram_id,
            relation,
            view.endpoints.as_ref(),
            &view.geometry,
        )
        .ok(),
    }
}

pub(crate) fn route_for_routing(
    section: &ModelingSection,
    diagram_id: &str,
    relation: &Relation,
    endpoints: Option<&crate::ViewEndpoints>,
    routing: RelationshipRouting,
) -> Result<Geometry, String> {
    let lane = match routing {
        RelationshipRouting::Manual => {
            return Err(
                "Manual routing preserves the authored path; edit its points directly".into(),
            );
        }
        RelationshipRouting::Straight => 0.0,
        RelationshipRouting::Automatic => lane_for_relation(section, diagram_id, relation),
        RelationshipRouting::Curve => 48.0,
    };
    let mut geometry = route_with_lane(
        section,
        diagram_id,
        &relation.from,
        &relation.to,
        endpoints,
        lane,
    )?;
    if routing == RelationshipRouting::Curve
        && let Geometry::Path { start, segments } = &mut geometry
        && let [PathSegment::Line { to }] = segments.as_slice()
    {
        let to = *to;
        *segments = vec![PathSegment::Cubic {
            control1: Point {
                x: start.x + (to.x - start.x) / 3.0,
                y: start.y + (to.y - start.y) / 3.0 - 48.0,
            },
            control2: Point {
                x: start.x + (to.x - start.x) * 2.0 / 3.0,
                y: start.y + (to.y - start.y) * 2.0 / 3.0 - 48.0,
            },
            to,
        }];
    }
    Ok(geometry)
}

fn reanchor_path(
    section: &ModelingSection,
    diagram_id: &str,
    relation: &Relation,
    endpoints: Option<&crate::ViewEndpoints>,
    original: &Geometry,
) -> Result<Geometry, String> {
    let Geometry::Path { start, segments } = original else {
        return Err("relationship View must be Path".into());
    };
    if segments.is_empty() {
        return Err("relationship path has no segments".into());
    }
    let new_route = route_with_lane(
        section,
        diagram_id,
        &relation.from,
        &relation.to,
        endpoints,
        0.0,
    )?;
    let Geometry::Path {
        start: fresh_start,
        segments: fresh_segments,
    } = new_route
    else {
        return Err("relationship route must be Path".into());
    };
    let last_point = |segments: &[PathSegment]| match segments.last() {
        Some(PathSegment::Line { to } | PathSegment::Cubic { to, .. }) => Some(*to),
        None => None,
    };
    let old_end = last_point(segments).ok_or("empty manual path")?;
    let fresh_end = last_point(&fresh_segments).ok_or("empty resolved path")?;
    let start_delta = Point {
        x: fresh_start.x - start.x,
        y: fresh_start.y - start.y,
    };
    let end_delta = Point {
        x: fresh_end.x - old_end.x,
        y: fresh_end.y - old_end.y,
    };
    let move_point = |point: Point, weight: f64| Point {
        x: point.x + start_delta.x * (1.0 - weight) + end_delta.x * weight,
        y: point.y + start_delta.y * (1.0 - weight) + end_delta.y * weight,
    };
    let count = segments.len() as f64;
    let reanchored = segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            let begin = index as f64 / count;
            let end = (index + 1) as f64 / count;
            match *segment {
                PathSegment::Line { to } => PathSegment::Line {
                    to: move_point(to, end),
                },
                PathSegment::Cubic {
                    control1,
                    control2,
                    to,
                } => PathSegment::Cubic {
                    control1: move_point(control1, begin + (end - begin) / 3.0),
                    control2: move_point(control2, begin + (end - begin) * 2.0 / 3.0),
                    to: move_point(to, end),
                },
            }
        })
        .collect();
    Ok(Geometry::Path {
        start: fresh_start,
        segments: reanchored,
    })
}

pub fn reroute_relationships(section: &mut ModelingSection, diagram_id: &str) {
    reroute_matching_relationships(section, diagram_id, |_| true);
}

pub(crate) fn reanchor_manual_paths(
    previous: &ModelingSection,
    current: &mut ModelingSection,
    diagram_id: &str,
) -> Result<(), String> {
    let previous_diagram = previous
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("previous diagram not found")?;
    let current_diagram = current
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?;
    let changed: std::collections::BTreeSet<_> = current_diagram
        .all_views()
        .filter(|view| {
            previous_diagram
                .all_views()
                .find(|previous| previous.id == view.id)
                .is_some_and(|previous| previous.geometry != view.geometry)
        })
        .map(|view| view.id.clone())
        .collect();
    let transform = |point: Point, before: Endpoint, after: Endpoint| -> Result<Point, String> {
        let result = Point {
            x: if before.width == after.width {
                point.x + (after.center.x - before.center.x)
            } else {
                after.center.x + ((point.x - before.center.x) / before.width) * after.width
            },
            y: if before.height == after.height {
                point.y + (after.center.y - before.center.y)
            } else {
                after.center.y + ((point.y - before.center.y) / before.height) * after.height
            },
        };
        if !result.x.is_finite() || !result.y.is_finite() {
            return Err(
                "manual relationship endpoint cannot be reanchored to finite coordinates".into(),
            );
        }
        Ok(result)
    };
    let mut updates = Vec::new();
    for view in current_diagram.all_views() {
        if view.draft.is_some() {
            continue;
        }
        let relation = current
            .relations
            .iter()
            .find(|relation| Some(relation.id.as_str()) == view.subject.as_deref());
        // Attached presentation paths do not need a recognized semantic Relation.
        // Their authored points remain authoritative, just like a manual relation.
        if relation.is_some() && view.style.routing != Some(RelationshipRouting::Manual)
            || relation.is_none() && view.endpoints.is_none()
        {
            continue;
        }
        let Geometry::Path { start, segments } = &view.geometry else {
            continue;
        };
        let Some(PathSegment::Line { to } | PathSegment::Cubic { to, .. }) = segments.last() else {
            continue;
        };
        let previous_view = previous_diagram
            .all_views()
            .find(|previous| previous.id == view.id)
            .ok_or("previous manual relationship View not found")?;
        let Geometry::Path {
            start: previous_start,
            segments: previous_segments,
        } = &previous_view.geometry
        else {
            return Err("previous manual relationship did not have Path geometry".into());
        };
        let Some(
            PathSegment::Line { to: previous_end }
            | PathSegment::Cubic {
                to: previous_end, ..
            },
        ) = previous_segments.last()
        else {
            return Err("previous manual relationship had no endpoint".into());
        };
        let endpoint_id = |source: bool| {
            if let Some(endpoints) = &view.endpoints {
                if source {
                    endpoints.source_view_id.as_deref()
                } else {
                    endpoints.target_view_id.as_deref()
                }
            } else {
                let relation = relation?;
                let subject = if source { &relation.from } else { &relation.to };
                current_diagram
                    .all_views()
                    .find(|view| view.subject.as_ref() == Some(subject))
                    .map(|view| view.id.as_str())
            }
        };
        let source_id = endpoint_id(true);
        let target_id = endpoint_id(false);
        let source = source_id.filter(|identifier| changed.contains(*identifier));
        let target = target_id.filter(|identifier| changed.contains(*identifier));
        if source.is_none() && target.is_none() && !changed.contains(&view.id) {
            continue;
        }
        // A containing View can move the route and its endpoint nodes together.
        // Transform the old anchors once, while keeping the route's transformed bends.
        let mut start = if source_id.is_some() {
            *previous_start
        } else {
            *start
        };
        let mut end = if target_id.is_some() {
            *previous_end
        } else {
            *to
        };
        if let Some(identifier) = source {
            let subject = relation.map(|relation| relation.from.as_str());
            start = transform(
                start,
                relationship_endpoint(previous, diagram_id, subject, Some(identifier), true)?,
                relationship_endpoint(current, diagram_id, subject, Some(identifier), true)?,
            )?;
        }
        if let Some(identifier) = target {
            let subject = relation.map(|relation| relation.to.as_str());
            end = transform(
                end,
                relationship_endpoint(previous, diagram_id, subject, Some(identifier), true)?,
                relationship_endpoint(current, diagram_id, subject, Some(identifier), true)?,
            )?;
        }
        updates.push((view.id.clone(), start, end));
    }
    let diagram = current
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?;
    for (identifier, new_start, new_end) in updates {
        let view = diagram
            .view_mut(&identifier)
            .ok_or("manual relationship View disappeared")?;
        let Geometry::Path { start, segments } = &mut view.geometry else {
            return Err("manual relationship no longer has Path geometry".into());
        };
        *start = new_start;
        match segments.last_mut() {
            Some(PathSegment::Line { to } | PathSegment::Cubic { to, .. }) => *to = new_end,
            None => return Err("manual relationship no longer has a final segment".into()),
        }
    }
    Ok(())
}

pub(crate) fn reroute_relationships_for_endpoint(
    section: &mut ModelingSection,
    diagram_id: &str,
    endpoint: &str,
) {
    reroute_matching_relationships(section, diagram_id, |relation| {
        relation.from == endpoint || relation.to == endpoint
    });
}

fn reroute_matching_relationships(
    section: &mut ModelingSection,
    diagram_id: &str,
    matches: impl Fn(&Relation) -> bool,
) {
    let Some(diagram) = section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
    else {
        return;
    };
    let attachments: Vec<_> = diagram
        .all_views()
        .filter_map(|view| {
            if view.endpoints.is_some() {
                return None;
            }
            let relation = section
                .relations
                .iter()
                .find(|relation| Some(relation.id.as_str()) == view.subject.as_deref())?;
            if !matches(relation) || !matches!(view.geometry, Geometry::Path { .. }) {
                return None;
            }
            let endpoints =
                resolve_view_endpoints(section, diagram_id, &relation.from, &relation.to).ok()?;
            Some((view.id.clone(), endpoints))
        })
        .collect();
    if let Some(diagram) = section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
    {
        for (view_id, endpoints) in attachments {
            if let Some(view) = diagram.view_mut(&view_id) {
                view.endpoints = Some(endpoints);
            }
        }
    }
    let Some(diagram) = section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
    else {
        return;
    };
    let routes: std::collections::BTreeMap<_, _> = diagram
        .all_views()
        .filter_map(|view| {
            let subject = view.subject.as_deref()?;
            let relation = section
                .relations
                .iter()
                .find(|relation| relation.id == subject)?;
            if !matches(relation) {
                return None;
            }
            let geometry = projected_relationship_view_path(section, diagram_id, relation, view)?;
            Some((view.id.clone(), geometry))
        })
        .collect();
    if let Some(diagram) = section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
    {
        for (view_id, geometry) in routes {
            if let Some(view) = diagram.view_mut(&view_id) {
                view.geometry = geometry;
            }
        }
    }
}

pub struct RelationshipGraph<'a> {
    section: &'a ModelingSection,
    diagram_id: &'a str,
}
impl<'a> RelationshipGraph<'a> {
    pub fn new(section: &'a ModelingSection, diagram_id: &'a str) -> Self {
        Self {
            section,
            diagram_id,
        }
    }
    pub fn visible_relations(&self) -> Vec<&'a Relation> {
        self.section
            .relations
            .iter()
            .filter(|relation| has_relation_view(self.section, self.diagram_id, &relation.id))
            .collect()
    }
    pub fn view_path(&self, view_id: &str) -> Option<Geometry> {
        let diagram = self
            .section
            .diagrams
            .iter()
            .find(|diagram| diagram.id == self.diagram_id)?;
        let view = diagram.all_views().find(|view| view.id == view_id)?;
        let subject = view.subject.as_deref()?;
        let relation = self
            .section
            .relations
            .iter()
            .find(|relation| relation.id == subject)?;
        projected_relationship_view_path(self.section, self.diagram_id, relation, view)
    }
    pub fn paths(&self) -> std::collections::BTreeMap<String, Geometry> {
        projected_relationship_paths(self.section, self.diagram_id)
    }
}

/// Aggregation and Composition are properties of Association ends, not new metaclasses.
pub fn relation_kind_for_creation(
    kind: ModelingRelationKind,
) -> (&'static str, Option<&'static str>) {
    match kind {
        ModelingRelationKind::DirectedAssociation => ("uml.association", None),
        ModelingRelationKind::Aggregation => ("uml.association", Some("shared")),
        ModelingRelationKind::Composition => ("uml.association", Some("composite")),
        other => (other.model_kind(), None),
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuickRelationRule {
    pub kind: &'static str,
    pub child_is_source: bool,
    pub aggregation: Option<&'static str>,
}
pub fn quick_relation_rule(action: crate::UmlQuickCreate) -> Option<QuickRelationRule> {
    use crate::UmlQuickCreate as A;
    let rule = match action {
        // These drawing kinds project reference slots, not UML Relationship contracts.
        A::Note => QuickRelationRule {
            kind: "uml.note_link",
            child_is_source: false,
            aggregation: None,
        },
        A::Constraint => QuickRelationRule {
            kind: "uml.constraint_link",
            child_is_source: true,
            aggregation: None,
        },
        A::Subtype => QuickRelationRule {
            kind: "uml.generalization",
            child_is_source: true,
            aggregation: None,
        },
        A::Supertype => QuickRelationRule {
            kind: "uml.generalization",
            child_is_source: false,
            aggregation: None,
        },
        A::RealizingClass => QuickRelationRule {
            kind: "uml.interface_realization",
            child_is_source: true,
            aggregation: None,
        },
        A::ProvidedInterface => QuickRelationRule {
            kind: "uml.interface_realization",
            child_is_source: false,
            aggregation: None,
        },
        A::RequiredInterface => QuickRelationRule {
            kind: "uml.usage",
            child_is_source: false,
            aggregation: None,
        },
        A::AssociatedClass => QuickRelationRule {
            kind: "uml.association",
            child_is_source: false,
            aggregation: None,
        },
        A::AggregatedClass => QuickRelationRule {
            kind: "uml.association",
            child_is_source: false,
            aggregation: Some("shared"),
        },
        A::CompositedClass => QuickRelationRule {
            kind: "uml.association",
            child_is_source: false,
            aggregation: Some("composite"),
        },
        _ => return None,
    };
    Some(rule)
}

#[cfg(test)]
mod graph_relationship_contract_tests {
    use super::*;

    #[test]
    fn moving_class_reanchors_existing_cubic_dependency_without_losing_its_shape() {
        let project = serde_json::from_str(include_str!("../../../fixtures/todo.json")).unwrap();
        let mut section = ModelingSection::from_model(&project);
        let diagram_id = "diagram-tasks";
        let original_view = section.diagrams[0]
            .views
            .iter()
            .find(|view| view.subject.as_deref() == Some("ref-task"))
            .unwrap()
            .geometry
            .clone();
        let original_start = match &original_view {
            Geometry::Path { start, .. } => *start,
            _ => panic!("fixture dependency must be a curve"),
        };
        let changed_node = section.diagrams[0]
            .views
            .iter_mut()
            .find(|view| view.subject.as_deref() == Some("type-list"))
            .unwrap();
        changed_node.geometry.translate(0.0, -80.0);
        let saved_before = section.clone();
        let projection = RelationshipGraph::new(&section, diagram_id).paths();
        assert_eq!(
            section, saved_before,
            "rendering is a pure graph projection"
        );
        let Geometry::Path { start, segments } = &projection["ref-task"] else {
            panic!("legacy dependency must remain a Path");
        };
        assert_ne!(
            *start, original_start,
            "source endpoint must follow the moved Class"
        );
        assert!(
            matches!(segments.first(), Some(PathSegment::Cubic { .. })),
            "manually shaped curve should retain its cubic bend"
        );
        reroute_relationships(&mut section, diagram_id);
        assert_eq!(
            section.diagrams[0]
                .views
                .iter()
                .find(|view| view.subject.as_deref() == Some("ref-task"))
                .unwrap()
                .geometry,
            projection["ref-task"],
            "persisted geometry follows the same graph contract as the renderer"
        );
        crate::Graph::new(&section.to_model("project-todo", "Todo"))
            .validate()
            .unwrap();
    }
}
