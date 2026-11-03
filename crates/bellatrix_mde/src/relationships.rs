//! One relationship contract for model operations and diagram projections.
//! GPUI paints and hit-tests; it does not define relationship semantics.
pub use crate::relationship_notation::{RelationshipMarker, RelationshipStyle, RelationshipEndpointLabels, RelationshipPresentation, relationship_style_for_notation, relationship_presentation};
use crate::{Geometry, ModelingSection, PathSegment, Point, Relation, RelationshipRouting, View};
use crate::{UmlType, classifier_contents};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModelingRelationKind {
    AssociationClassAttachment,
    Association,
    DirectedAssociation,
    Aggregation,
    Composition,
    Dependency,
    Generalization,
    InterfaceRealization,
    TemplateBinding,
    Realization,
    Containment,
    Usage,
    Abstraction,
    Include,
    Extend,
    PackageImport,
    PackageMerge,
    Deployment,
    Manifestation,
    ControlFlow,
    ObjectFlow,
    Transition,
}
impl ModelingRelationKind {
    pub const BASIC: &'static [Self] = &[
        Self::Association,
        Self::DirectedAssociation,
        Self::Aggregation,
        Self::Composition,
        Self::Dependency,
        Self::Generalization,
        Self::InterfaceRealization,
    ];
    pub const ADVANCED: &'static [Self] = &[Self::TemplateBinding, Self::Realization, Self::AssociationClassAttachment];
    pub const BEHAVIOR: &'static [Self] = &[
        Self::Include,
        Self::Extend,
        Self::ControlFlow,
        Self::ObjectFlow,
        Self::Transition,
    ];
    pub const STRUCTURAL: &'static [Self] = &[
        Self::Usage,
        Self::Abstraction,
        Self::PackageImport,
        Self::PackageMerge,
        Self::Deployment,
        Self::Manifestation,
    ];
    pub const PACKAGES: &'static [Self] = &[Self::Containment];
    pub fn label(self) -> &'static str {
        match self {
            Self::AssociationClassAttachment => "Association-class attachment",
            Self::Association => "Association",
            Self::DirectedAssociation => "Directed Association",
            Self::Aggregation => "Aggregation",
            Self::Composition => "Composition",
            Self::Dependency => "Dependency",
            Self::Generalization => "Generalization",
            Self::InterfaceRealization => "Interface Realization",
            Self::TemplateBinding => "Template Binding",
            Self::Realization => "Realization",
            Self::Containment => "Containment",
            Self::Usage => "Usage",
            Self::Abstraction => "Abstraction",
            Self::Include => "Include",
            Self::Extend => "Extend",
            Self::PackageImport => "Package Import",
            Self::PackageMerge => "Package Merge",
            Self::Deployment => "Deployment",
            Self::Manifestation => "Manifestation",
            Self::ControlFlow => "Control Flow",
            Self::ObjectFlow => "Object Flow",
            Self::Transition => "Transition",
        }
    }
    pub fn model_kind(self) -> &'static str {
        match self {
            Self::AssociationClassAttachment => "view.association_class_attachment",
            Self::Association => "uml.association",
            Self::DirectedAssociation => "uml.directed_association",
            Self::Aggregation => "uml.aggregation",
            Self::Composition => "uml.composition",
            Self::Dependency => "uml.dependency",
            Self::Generalization => "uml.generalization",
            Self::InterfaceRealization => "uml.interface_realization",
            Self::TemplateBinding => "uml.template_binding",
            Self::Realization => "uml.realization",
            Self::Containment => "uml.containment",
            Self::Usage => "uml.usage",
            Self::Abstraction => "uml.abstraction",
            Self::Include => "uml.include",
            Self::Extend => "uml.extend",
            Self::PackageImport => "uml.package_import",
            Self::PackageMerge => "uml.package_merge",
            Self::Deployment => "uml.deployment",
            Self::Manifestation => "uml.manifestation",
            Self::ControlFlow => "uml.control_flow",
            Self::ObjectFlow => "uml.object_flow",
            Self::Transition => "uml.transition",
        }
    }
    /// The first click is Relation.from, not necessarily the arrowhead end.
    pub fn endpoint_instruction(self) -> &'static str {
        match self {
            Self::AssociationClassAttachment => "click the association path, then its classifier box (visual attachment)",
            Self::Association => "click either first class, then the second (no arrow)",
            Self::DirectedAssociation => "click source, then navigable target (arrow at target)",
            Self::Aggregation => "click the whole, then the part (hollow diamond at first)",
            Self::Composition => "click the whole, then the part (filled diamond at first)",
            Self::Dependency => "click dependent client, then supplier (arrow at target)",
            Self::Generalization => "click subtype, then supertype (triangle at target)",
            Self::InterfaceRealization => {
                "click implementing class, then interface (triangle at target)"
            }
            Self::TemplateBinding => "click bound element, then template (arrow at target)",
            Self::Realization => {
                "click implementing classifier, then specification (triangle at target)"
            }
            Self::Containment => "click container, then contained element (circle at first)",
            Self::Usage | Self::Abstraction => "click client, then supplier",
            Self::Include => "click including use case, then included use case",
            Self::Extend => "click extending use case, then extended use case",
            Self::PackageImport => "click importing namespace, then imported package",
            Self::PackageMerge => "click receiving package, then merged package",
            Self::Deployment => "click deployed artifact, then deployment target",
            Self::Manifestation => "click artifact, then manifested model element",
            Self::ControlFlow | Self::ObjectFlow => {
                "click source activity node, then target in the same activity"
            }
            Self::Transition => "click source vertex, then target in the same region",
        }
    }

    pub fn is_visual_only(self) -> bool { self == Self::AssociationClassAttachment }
    pub fn validate_endpoints(self, source: &str, target: &str) -> Result<(), &'static str> {
        let conforms = |kind: &str, expected: &str| {
            crate::uml::root::object_metaclass(kind)
                .is_some_and(|actual| crate::uml::metamodel::conforms_to(actual, expected))
        };
        let valid = match self {
            Self::Include | Self::Extend => {
                Some(conforms(source, "UseCase") && conforms(target, "UseCase"))
            }
            Self::PackageImport => {
                Some(conforms(source, "Namespace") && conforms(target, "Package"))
            }
            Self::PackageMerge => Some(conforms(source, "Package") && conforms(target, "Package")),
            Self::Deployment => {
                Some(conforms(source, "DeployedArtifact") && conforms(target, "DeploymentTarget"))
            }
            Self::Manifestation => {
                Some(conforms(source, "Artifact") && conforms(target, "PackageableElement"))
            }
            Self::ControlFlow | Self::ObjectFlow => {
                Some(conforms(source, "ActivityNode") && conforms(target, "ActivityNode"))
            }
            Self::Transition => Some(conforms(source, "Vertex") && conforms(target, "Vertex")),
            Self::Usage | Self::Abstraction => {
                Some(conforms(source, "NamedElement") && conforms(target, "NamedElement"))
            }
            _ => None,
        };
        if let Some(valid) = valid {
            return if valid {
                Ok(())
            } else {
                Err("endpoints do not conform to this UML relationship's types")
            };
        }
        const PACKAGES: &[&str] = &["uml.package", "uml.model", "uml.subsystem"];
        if self == Self::Containment {
            return if PACKAGES.contains(&source) {
                Ok(())
            } else {
                Err("containment source must be a UML Package/Model/Subsystem")
            };
        }
        if self == Self::InterfaceRealization && target != "rust.trait" && target != "uml.interface"
        {
            return Err("interface realization must target an Interface");
        }
        if self == Self::Generalization
            && (!conforms(source, "Classifier") || !conforms(target, "Classifier"))
        {
            return Err("generalization requires classifier endpoints");
        }
        if self == Self::TemplateBinding && source != "uml.template" && target != "uml.template" {
            return Err("template binding requires a Template endpoint");
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Endpoint {
    pub(crate) center: Point,
    pub(crate) width: f64,
    pub(crate) height: f64,
}
impl Endpoint {
    pub(crate) fn anchor_toward(self, target: Point) -> Point {
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
    route_with_lane(section, diagram_id, from, to, 0.0)
}

fn route_with_lane(
    section: &ModelingSection,
    diagram_id: &str,
    from: &str,
    to: &str,
    lane: f64,
) -> Result<Geometry, String> {
    let diagram = section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?;
    let endpoint = |subject: &str| -> Result<Endpoint, String> {
        let view = diagram
            .views
            .iter()
            .find(|view| view.subject.as_deref() == Some(subject))
            .ok_or_else(|| format!("endpoint {subject} has no View in this diagram"))?;
        let (left, top, width, height) = match &view.geometry {
            Geometry::Rect {
                x,
                y,
                width,
                height,
            } if *width > 0.0 && *height > 0.0 => (*x, *y, *width, *height),
            _ => return Err("relationship endpoint has no valid node View".to_owned()),
        };
        let kind = section
            .elements
            .iter()
            .find(|element| element.id == subject)
            .and_then(|element| UmlType::from_kind(&element.kind));
        let height = if kind == Some(UmlType::Interface) {
            height.max(classifier_contents(section, subject).minimum_interface_height())
        } else {
            height
        };
        let height = section
            .elements
            .iter()
            .find(|element| element.id == subject)
            .map(|element| {
                let notation = crate::uml_notation(&element.kind);
                let header = crate::uml_header(
                    &element.kind,
                    element
                        .properties
                        .get("stereotype")
                        .and_then(serde_json::Value::as_str),
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
        })
    };
    let source = endpoint(from)?;
    let target = endpoint(to)?;
    if from == to {
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
                .views
                .iter()
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
            d.views.iter().any(|v| {
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
                .views
                .iter()
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
        Some(routing @ (RelationshipRouting::Automatic | RelationshipRouting::Straight)) => {
            route_for_routing(section, diagram_id, relation, routing).ok()
        }
        Some(RelationshipRouting::Curve) | None => {
            reanchor_path(section, diagram_id, relation, &view.geometry).ok()
        }
    }
}

pub(crate) fn route_for_routing(
    section: &ModelingSection,
    diagram_id: &str,
    relation: &Relation,
    routing: RelationshipRouting,
) -> Result<Geometry, String> {
    let lane = match routing {
        RelationshipRouting::Straight => 0.0,
        RelationshipRouting::Automatic => lane_for_relation(section, diagram_id, relation),
        RelationshipRouting::Curve => 48.0,
    };
    let mut geometry = route_with_lane(section, diagram_id, &relation.from, &relation.to, lane)?;
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
    original: &Geometry,
) -> Result<Geometry, String> {
    let Geometry::Path { start, segments } = original else {
        return Err("relationship View must be Path".into());
    };
    if segments.is_empty() {
        return Err("relationship path has no segments".into());
    }
    let new_route = route(section, diagram_id, &relation.from, &relation.to)?;
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
    let routes: std::collections::BTreeMap<_, _> = diagram
        .views
        .iter()
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
        for view in &mut diagram.views {
            if let Some(geometry) = routes.get(&view.id) {
                view.geometry = geometry.clone();
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
        let view = diagram.views.iter().find(|view| view.id == view_id)?;
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
    fn endpoint_text_projection_preserves_raw_values_and_model_identity() {
        let relation = Relation {
            id: "r".into(),
            kind: "uml.association".into(),
            from: "source".into(),
            to: "target".into(),
            properties: [
                ("name", "owns"),
                ("sourceRole", " owner "),
                ("targetRole", "items"),
                ("sourceMultiplicity", "1"),
                ("targetMultiplicity", " 0..* "),
            ]
            .into_iter()
            .map(|(key, value)| (key.into(), serde_json::json!(value)))
            .collect(),
        };
        let before = relation.clone();
        let presentation = relationship_presentation(&relation);
        assert_eq!(presentation.name.as_deref(), Some("owns"));
        assert_eq!(presentation.source_labels.role.as_deref(), Some(" owner "));
        assert_eq!(presentation.target_labels.role.as_deref(), Some("items"));
        assert_eq!(
            presentation.source_labels.multiplicity.as_deref(),
            Some("1")
        );
        assert_eq!(
            presentation.target_labels.multiplicity.as_deref(),
            Some(" 0..* ")
        );
        assert_eq!(
            presentation.style,
            relationship_style_for_notation("uml.association")
        );
        assert_eq!(relation, before);
    }

    #[test]
    fn endpoint_text_projection_does_not_invent_or_cross_fill_labels() {
        let mut relation = Relation {
            id: "r".into(),
            kind: "uml.composition".into(),
            from: "source".into(),
            to: "target".into(),
            properties: Default::default(),
        };
        let absent = relationship_presentation(&relation);
        assert_eq!(absent.name, None);
        assert_eq!(absent.source_labels, RelationshipEndpointLabels::default());
        assert_eq!(absent.target_labels, RelationshipEndpointLabels::default());
        relation
            .properties
            .insert("sourceRole".into(), serde_json::json!(""));
        relation.properties.insert(
            "targetMultiplicity".into(),
            serde_json::json!("unvalidated"),
        );
        relation
            .properties
            .insert("sourceMultiplicity".into(), serde_json::json!(7));
        let independent = relationship_presentation(&relation);
        assert_eq!(independent.source_labels.role.as_deref(), Some(""));
        assert_eq!(independent.source_labels.multiplicity, None);
        assert_eq!(independent.target_labels.role, None);
        assert_eq!(
            independent.target_labels.multiplicity.as_deref(),
            Some("unvalidated")
        );
    }

    #[test]
    fn toolbox_and_legacy_notation_have_one_direction_table() {
        use RelationshipMarker::*;
        assert_eq!(
            relationship_style_for_notation("uml.generalization").target_marker,
            Triangle
        );
        assert_eq!(
            relationship_style_for_notation("uml.interface_realization").target_marker,
            Triangle
        );
        assert_eq!(
            relationship_style_for_notation("uml.dependency").target_marker,
            OpenArrow
        );
        assert_eq!(
            relationship_style_for_notation("depends_on"),
            relationship_style_for_notation("uml.dependency")
        );
        assert_eq!(
            relationship_style_for_notation("uml.aggregation").source_marker,
            Diamond
        );
        assert_eq!(
            relationship_style_for_notation("uml.composition").source_marker,
            FilledDiamond
        );
        assert_eq!(
            relationship_style_for_notation("uml.association").target_marker,
            None
        );
        let make = |kind: &str| RelationshipPresentation {
            name: Option::None,
            source_labels: RelationshipEndpointLabels::default(),
            target_labels: RelationshipEndpointLabels::default(),
            label: kind.into(),
            notation: kind.into(),
            style: relationship_style_for_notation(kind),
            source_role: if kind == "uml.composition" {
                "whole"
            } else {
                "subtype"
            },
            target_role: if kind == "uml.composition" {
                "part"
            } else {
                "supertype"
            },
        };
        assert!(
            make("uml.composition")
                .describe("A", "B")
                .contains("A (whole)")
        );
        assert!(
            make("uml.generalization")
                .describe("A", "B")
                .contains("B (supertype)")
        );
    }
    #[test]
    fn moving_class_reanchors_existing_cubic_dependency_without_losing_its_shape() {
        let project = serde_json::from_str(include_str!("../fixtures/todo.json")).unwrap();
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
    #[test]
    fn model_explorer_and_canvas_query_identical_visible_edges() {
        let project = serde_json::from_str(include_str!("../fixtures/todo.json")).unwrap();
        let section = ModelingSection::from_model(&project);
        let graph = RelationshipGraph::new(&section, "diagram-tasks");
        let ids = graph
            .visible_relations()
            .iter()
            .map(|relation| relation.id.as_str())
            .collect::<Vec<_>>();
        let projected = graph.paths();
        assert_eq!(ids, vec!["ref-task"]);
        assert_eq!(
            projected.keys().map(String::as_str).collect::<Vec<_>>(),
            ids
        );
        let presentation = relationship_presentation(graph.visible_relations()[0]);
        assert_eq!(presentation.label, "dependency");
        assert_eq!(
            presentation.style.target_marker,
            RelationshipMarker::OpenArrow
        );
    }
    #[test]
    fn quick_edit_composes_same_endpoints_without_reversing_roles() {
        use crate::UmlQuickCreate as A;
        assert_eq!(
            quick_relation_rule(A::Subtype).unwrap(),
            QuickRelationRule {
                kind: "uml.generalization",
                child_is_source: true,
                aggregation: None
            }
        );
        assert!(!quick_relation_rule(A::Supertype).unwrap().child_is_source);
        assert!(
            !quick_relation_rule(A::ProvidedInterface)
                .unwrap()
                .child_is_source
        );
        assert!(
            quick_relation_rule(A::RealizingClass)
                .unwrap()
                .child_is_source
        );
        assert_eq!(
            quick_relation_rule(A::AggregatedClass).unwrap().aggregation,
            Some("shared")
        );
        assert_eq!(
            quick_relation_rule(A::CompositedClass).unwrap().aggregation,
            Some("composite")
        );
    }
}
