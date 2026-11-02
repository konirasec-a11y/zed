//! Single graph-theoretic contract for all MDE relationships.
//!
//! Bellatrix model IDs and oriented Relation edges are domain truth. A diagram
//! View is the geometric projection of an edge, not another relationship.
//! Graph constraints, target validation, lane routing, legacy compatibility
//! and notation metadata live here. Cordis owns the versioned command transaction;
//! GPUI owns only stroke painting and hit testing. Future typed-graph rules extend
//! this module rather than being scattered among Canvas and UI commands.
use bellatrix_mde_model::{Geometry, ModelingSection, PathSegment, Point, Relation};
use crate::{UmlType, InterfaceLayout, classifier_contents, interface_layout};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModelingRelationKind {
    Association, DirectedAssociation, Aggregation, Composition, Dependency,
    Generalization, InterfaceRealization, TemplateBinding, Realization, Containment,
}
impl ModelingRelationKind {
    pub const BASIC: &'static [Self] = &[
        Self::Association, Self::DirectedAssociation, Self::Aggregation, Self::Composition,
        Self::Dependency, Self::Generalization, Self::InterfaceRealization,
    ];
    pub const ADVANCED: &'static [Self] = &[Self::TemplateBinding, Self::Realization];
    pub const PACKAGES: &'static [Self] = &[Self::Containment];
    pub fn label(self) -> &'static str {
        match self {
            Self::Association => "Association", Self::DirectedAssociation => "Directed Association",
            Self::Aggregation => "Aggregation", Self::Composition => "Composition",
            Self::Dependency => "Dependency", Self::Generalization => "Generalization",
            Self::InterfaceRealization => "Interface Realization",
            Self::TemplateBinding => "Template Binding", Self::Realization => "Realization",
            Self::Containment => "Containment",
        }
    }
    pub fn model_kind(self) -> &'static str {
        match self {
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
        }
    }
    /// Endpoint order is a graph contract, not a geometric convention.
    /// The first Toolbox click becomes Relation.from and the second Relation.to.
    /// The marker can appear on either end according to UML semantics.
    pub fn endpoint_instruction(self) -> &'static str {
        match self {
            Self::Association => "click either first class, then the second (no arrow)",
            Self::DirectedAssociation => "click source, then navigable target (arrow at target)",
            Self::Aggregation => "click the whole, then the part (hollow diamond at first)",
            Self::Composition => "click the whole, then the part (filled diamond at first)",
            Self::Dependency => "click dependent client, then supplier (arrow at target)",
            Self::Generalization => "click subtype, then supertype (triangle at target)",
            Self::InterfaceRealization => "click implementing class, then interface (triangle at target)",
            Self::TemplateBinding => "click bound element, then template (arrow at target)",
            Self::Realization => "click implementing classifier, then specification (triangle at target)",
            Self::Containment => "click container, then contained element (circle at first)",
        }
    }

    pub fn validate_endpoints(self, source: &str, target: &str) -> Result<(), &'static str> {
        const CLASSIFIERS: &[&str] = &[
            "rust.struct", "rust.trait", "uml.class", "uml.interface",
            "uml.signal", "uml.data_type", "uml.primitive_type",
            "uml.enumeration", "uml.association_class", "uml.template",
        ];
        const PACKAGES: &[&str] = &["uml.package", "uml.model", "uml.subsystem"];
        if self == Self::Containment {
            return if PACKAGES.contains(&source) { Ok(()) }
                else { Err("containment source must be a UML Package/Model/Subsystem") };
        }
        if self == Self::InterfaceRealization && target != "rust.trait" && target != "uml.interface" {
            return Err("interface realization must target an Interface");
        }
        if self == Self::Generalization && (!CLASSIFIERS.contains(&source) || !CLASSIFIERS.contains(&target)) {
            return Err("generalization requires classifier endpoints");
        }
        if self == Self::TemplateBinding && source != "uml.template" && target != "uml.template" {
            return Err("template binding requires a Template endpoint");
        }
        Ok(())
    }
}


#[derive(Clone, Copy)]
struct Endpoint {
    center: Point,
    width: f64,
    height: f64,
    circle: Option<InterfaceLayout>,
}
impl Endpoint {
    fn anchor_toward(self, target: Point) -> Point {
        if let Some(circle) = self.circle { return circle.anchor_toward(target); }
        let delta_x = target.x - self.center.x;
        let delta_y = target.y - self.center.y;
        if delta_x.abs() < f64::EPSILON && delta_y.abs() < f64::EPSILON {
            return Point { x: self.center.x + self.width / 2.0, y: self.center.y };
        }
        let horizontal = if delta_x.abs() > f64::EPSILON { self.width / 2.0 / delta_x.abs() } else { f64::MAX };
        let vertical = if delta_y.abs() > f64::EPSILON { self.height / 2.0 / delta_y.abs() } else { f64::MAX };
        let distance = horizontal.min(vertical);
        Point { x: self.center.x + delta_x * distance, y: self.center.y + delta_y * distance }
    }
}

/// Base route for a single relationship (or a hand-authored curved View).
pub fn route(section: &ModelingSection, diagram_id: &str, from: &str, to: &str) -> Result<Geometry, String> {
    route_with_lane(section, diagram_id, from, to, 0.0)
}

fn route_with_lane(section: &ModelingSection, diagram_id: &str, from: &str, to: &str, lane: f64) -> Result<Geometry, String> {
    let diagram = section.diagrams.iter().find(|diagram| diagram.id == diagram_id).ok_or("diagram not found")?;
    let endpoint = |subject: &str| -> Result<Endpoint, String> {
        let view = diagram.views.iter().find(|view| view.subject.as_deref() == Some(subject))
            .ok_or_else(|| format!("endpoint {subject} has no View in this diagram"))?;
        let (left, top, width, height) = match &view.geometry {
            Geometry::Rect { x, y, width, height } if *width > 0.0 && *height > 0.0 => (*x, *y, *width, *height),
            _ => return Err("relationship endpoint has no valid node View".to_owned()),
        };
        let kind = section.elements.iter().find(|element| element.id == subject)
            .and_then(|element| UmlType::from_kind(&element.kind));
        if kind == Some(UmlType::Interface) {
            let mut layout = interface_layout(width, height, classifier_contents(section, subject));
            layout.center.x += left;
            layout.center.y += top;
            return Ok(Endpoint { center: layout.center, width: layout.radius * 2.0,
                height: layout.radius * 2.0, circle: Some(layout) });
        }
        Ok(Endpoint { center: Point { x: left + width / 2.0, y: top + height / 2.0 }, width, height, circle: None })
    };
    let source = endpoint(from)?;
    let target = endpoint(to)?;
    if from == to {
        let start = source.anchor_toward(Point { x: source.center.x + source.width, y: source.center.y - source.height });
        let end = source.anchor_toward(Point { x: source.center.x + source.width, y: source.center.y + source.height });
        return Ok(Geometry::Path { start, segments: vec![PathSegment::Cubic {
            control1: Point { x: start.x + 62.0, y: start.y - 56.0 },
            control2: Point { x: end.x + 62.0, y: end.y + 56.0 }, to: end,
        }] });
    }
    let same_center = (source.center.x - target.center.x).abs() < f64::EPSILON
        && (source.center.y - target.center.y).abs() < f64::EPSILON;
    // Parallel relationships share semantic endpoints but must not occupy
    // identical screen paths. Re-anchor each lane on the node boundaries,
    // rather than translating a finished line outside the classifier shape.
    let dx = target.center.x - source.center.x;
    let dy = target.center.y - source.center.y;
    let distance = dx.hypot(dy);
    let offset = if distance > f64::EPSILON {
        let direction = if from <= to { 1.0 } else { -1.0 };
        Point { x: -dy / distance * lane * direction,
            y: dx / distance * lane * direction }
    } else { Point { x: 0.0, y: 0.0 } };
    let start = source.anchor_toward(if same_center {
        Point { x: source.center.x + source.width, y: source.center.y }
    } else { Point { x: target.center.x + offset.x, y: target.center.y + offset.y } });
    let to = target.anchor_toward(if same_center {
        Point { x: target.center.x - target.width, y: target.center.y }
    } else { Point { x: source.center.x + offset.x, y: source.center.y + offset.y } });
    if lane.abs() < f64::EPSILON || same_center {
        return Ok(Geometry::Path { start, segments: vec![PathSegment::Line { to }] });
    }
    let control1 = Point {
        x: start.x + (to.x - start.x) / 3.0 + offset.x * 1.25,
        y: start.y + (to.y - start.y) / 3.0 + offset.y * 1.25,
    };
    let control2 = Point {
        x: start.x + (to.x - start.x) * 2.0 / 3.0 + offset.x * 1.25,
        y: start.y + (to.y - start.y) * 2.0 / 3.0 + offset.y * 1.25,
    };
    Ok(Geometry::Path { start, segments: vec![
        PathSegment::Cubic { control1, control2, to },
    ] })
}

/// Stable slots include hand-authored diagram relationships, but those Views
/// are never modified. Auto-routed siblings distribute across separate lanes.
fn lane_for_relation(section: &ModelingSection, diagram_id: &str, relation: &Relation) -> f64 {
    let Some(diagram) = section.diagrams.iter().find(|d| d.id == diagram_id) else {
        return 0.0;
    };
    let peers = section.relations.iter()
        .filter(|other| {
            (other.from == relation.from && other.to == relation.to)
                || (other.from == relation.to && other.to == relation.from)
        })
        .filter(|other| diagram.views.iter().any(|v|
            v.subject.as_deref() == Some(other.id.as_str())
        ))
        .collect::<Vec<_>>();
    if peers.len() < 2 { return 0.0; }
    let index = peers.iter().position(|peer| peer.id == relation.id).unwrap_or(0);
    (index as f64 - (peers.len() - 1) as f64 / 2.0) * 48.0
}

/// Whether the graph edge has a visible relationship View in this diagram.
pub fn has_relation_view(section: &ModelingSection, diagram_id: &str, id: &str) -> bool {
    section.diagrams.iter().find(|d| d.id == diagram_id)
        .is_some_and(|d| d.views.iter().any(|v|
            v.subject.as_deref() == Some(id) && matches!(v.geometry, Geometry::Path { .. })
        ))
}

/// Read-only graph projection. Legacy dependencies have a Path and must follow
/// their endpoints just like newly-created UML relationships. The projection
/// never writes data: only a versioned ProjectReplace commits geometry.
pub fn projected_relationship_paths(
    section: &ModelingSection, diagram_id: &str
) -> std::collections::BTreeMap<String, Geometry> {
    let Some(diagram) = section.diagrams.iter().find(|d| d.id == diagram_id) else {
        return Default::default();
    };
    section.relations.iter()
        .filter_map(|relation| {
            let view = diagram.views.iter().find(|v| v.subject.as_deref() == Some(relation.id.as_str()))?;
            let Geometry::Path { .. } = &view.geometry else { return None };
            let routing = relation.properties.get("routing").and_then(serde_json::Value::as_str);
            let geometry = if routing == Some("automatic") {
                route_with_lane(section, diagram_id, &relation.from, &relation.to,
                    lane_for_relation(section, diagram_id, relation)).ok()?
            } else {
                // No routing flag in old ProjectDocuments means "preserve
                // bend shape", not "detach the edge from its graph nodes".
                reanchor_path(section, diagram_id, relation, &view.geometry).ok()?
            };
            Some((relation.id.clone(), geometry))
        })
        .collect()
}

/// Adjust a manually shaped path while respecting its original control points.
/// Both endpoints come from the same graph anchor resolver as automatic routes.
/// Cubic controls follow their nearest endpoint; the bend is preserved.
fn reanchor_path(
    section: &ModelingSection, diagram_id: &str, relation: &Relation, original: &Geometry
) -> Result<Geometry, String> {
    let Geometry::Path { start, segments } = original else {
        return Err("relationship View must be Path".into())
    };
    if segments.is_empty() { return Err("relationship path has no segments".into()) }
    let new_route = route(section, diagram_id, &relation.from, &relation.to)?;
    let Geometry::Path { start: fresh_start, segments: fresh_segments } = new_route else {
        return Err("relationship route must be Path".into())
    };
    let last_point = |segments: &[PathSegment]| match segments.last() {
        Some(PathSegment::Line { to } | PathSegment::Cubic { to, .. }) => Some(*to),
        None => None,
    };
    let old_end = last_point(segments).ok_or("empty manual path")?;
    let fresh_end = last_point(&fresh_segments).ok_or("empty resolved path")?;
    let start_delta = Point { x: fresh_start.x - start.x, y: fresh_start.y - start.y };
    let end_delta = Point { x: fresh_end.x - old_end.x, y: fresh_end.y - old_end.y };
    let move_point = |point: Point, weight: f64| Point {
        x: point.x + start_delta.x * (1.0 - weight) + end_delta.x * weight,
        y: point.y + start_delta.y * (1.0 - weight) + end_delta.y * weight,
    };
    let count = segments.len() as f64;
    let reanchored = segments.iter().enumerate().map(|(index, segment)| {
        let begin = index as f64 / count;
        let end = (index + 1) as f64 / count;
        match *segment {
            PathSegment::Line { to } => PathSegment::Line { to: move_point(to, end) },
            PathSegment::Cubic { control1, control2, to } => PathSegment::Cubic {
                control1: move_point(control1, begin + (end - begin) / 3.0),
                control2: move_point(control2, begin + (end - begin) * 2.0 / 3.0),
                to: move_point(to, end),
            },
        }
    }).collect();
    Ok(Geometry::Path { start: fresh_start, segments: reanchored })
}

/// The same graph projection is persisted only as part of a real, revisioned
/// mutation (node move, resize, relation create, etc). Rendering stays pure.
pub fn reroute_relationships(section: &mut ModelingSection, diagram_id: &str) {
    let routes = projected_relationship_paths(section, diagram_id);
    if let Some(diagram) = section.diagrams.iter_mut().find(|d| d.id == diagram_id) {
        for view in &mut diagram.views {
            if let Some(subject) = &view.subject {
                if let Some(geometry) = routes.get(subject) {
                    view.geometry = geometry.clone();
                }
            }
        }
    }
}



/// UML edge-marker grammar. A source marker is drawn at Relation.from and
/// a target marker at Relation.to. Geometry does not decide orientation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationshipMarker {
    None, OpenArrow, Triangle, Diamond, FilledDiamond, Containment,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelationshipStyle {
    pub source_marker: RelationshipMarker,
    pub target_marker: RelationshipMarker,
    pub dashed: bool,
}
/// One authoritative notation table for Toolbox, Quick Edit and legacy data.
pub fn relationship_style_for_notation(kind: &str) -> RelationshipStyle {
    use RelationshipMarker::*;
    let (source_marker, target_marker, dashed) = match kind {
        "uml.generalization" => (None, Triangle, false),
        "uml.interface_realization" | "uml.realization" => (None, Triangle, true),
        "uml.directed_association" => (None, OpenArrow, false),
        "uml.dependency" | "uml.template_binding" | "depends_on" =>
            (None, OpenArrow, true),
        "uml.aggregation" => (Diamond, None, false),
        "uml.composition" => (FilledDiamond, None, false),
        "uml.containment" => (Containment, None, false),
        "uml.note_link" | "uml.constraint_link" => (None, None, true),
        _ => (None, None, false),
    };
    RelationshipStyle {source_marker, target_marker, dashed}
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationshipPresentation {
    pub label: String,
    pub notation: String,
    pub style: RelationshipStyle,
}
/// Sole mapping from semantic relationship into visible behavior.
/// The legacy "depends_on" is a graph Dependency, not a "project" decoration.
pub fn relationship_presentation(relation: &Relation) -> RelationshipPresentation {
    let notation = match relation.kind.as_str() {
        "depends_on" => "uml.dependency",
        "uml.association" => match relation.properties.get("end1Aggregation")
            .and_then(serde_json::Value::as_str) {
                Some("shared") => "uml.aggregation",
                Some("composite") => "uml.composition",
                _ => "uml.association",
            },
        other => other,
    };
    let label = notation.strip_prefix("uml.").unwrap_or(notation).replace('_', " ");
    RelationshipPresentation {
        label,
        notation: notation.to_owned(),
        style: relationship_style_for_notation(notation),
    }
}
/// Authoritative graph projection consumed by both Canvas and Model Explorer.
pub struct RelationshipGraph<'a> {
    section: &'a ModelingSection,
    diagram_id: &'a str,
}
impl<'a> RelationshipGraph<'a> {
    pub fn new(section: &'a ModelingSection, diagram_id: &'a str) -> Self {
        Self { section, diagram_id }
    }
    pub fn visible_relations(&self) -> Vec<&'a Relation> {
        self.section.relations.iter().filter(|relation|
            has_relation_view(self.section, self.diagram_id, &relation.id)
        ).collect()
    }
    pub fn paths(&self) -> std::collections::BTreeMap<String, Geometry> {
        projected_relationship_paths(self.section, self.diagram_id)
    }
}

/// Create-time semantics: Aggregation and Composition specialize an Association
/// end (whole side), not the Relation kind. The same rule is used by Toolbox
/// and contextual Quick Edit.
pub fn relation_kind_for_creation(kind: ModelingRelationKind) -> (&'static str, Option<&'static str>) {
    match kind {
        ModelingRelationKind::Aggregation => ("uml.association", Some("shared")),
        ModelingRelationKind::Composition => ("uml.association", Some("composite")),
        other => (other.model_kind(), None),
    }
}

/// Maps contextual create-new action to its directed graph edge. New node
/// creation uses the same relation semantics as Toolbox; only the UI gesture
/// differs. The clicked classifier is `subject`, and `child_is_source`
/// controls which endpoint of Relation receives the newly created node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuickRelationRule {
    pub kind: &'static str,
    pub child_is_source: bool,
    pub aggregation: Option<&'static str>,
}
pub fn quick_relation_rule(action: crate::UmlQuickCommand) -> Option<QuickRelationRule> {
    use crate::UmlQuickCommand as A;
    let rule = match action {
        A::AddNote => QuickRelationRule {kind:"uml.note_link",child_is_source:false,aggregation:None},
        A::AddConstraint => QuickRelationRule {kind:"uml.constraint_link",child_is_source:true,aggregation:None},
        A::AddSubtype => QuickRelationRule {kind:"uml.generalization",child_is_source:true,aggregation:None},
        A::AddSupertype => QuickRelationRule {kind:"uml.generalization",child_is_source:false,aggregation:None},
        A::AddRealizingClass => QuickRelationRule {kind:"uml.interface_realization",child_is_source:true,aggregation:None},
        A::AddProvidedInterface => QuickRelationRule {kind:"uml.interface_realization",child_is_source:false,aggregation:None},
        A::AddRequiredInterface => QuickRelationRule {kind:"uml.dependency",child_is_source:false,aggregation:None},
        A::AddAssociatedClass => QuickRelationRule {kind:"uml.association",child_is_source:false,aggregation:None},
        A::AddAggregatedClass => QuickRelationRule {kind:"uml.association",child_is_source:false,aggregation:Some("shared")},
        A::AddCompositedClass => QuickRelationRule {kind:"uml.association",child_is_source:false,aggregation:Some("composite")},
        _ => return None,
    };
    Some(rule)
}

#[cfg(test)]
mod graph_relationship_contract_tests {
    use super::*;
    #[test]
    fn toolbox_and_legacy_notation_have_one_direction_table() {
        use RelationshipMarker::*;
        assert_eq!(relationship_style_for_notation("uml.generalization").target_marker, Triangle);
        assert_eq!(relationship_style_for_notation("uml.interface_realization").target_marker, Triangle);
        assert_eq!(relationship_style_for_notation("uml.dependency").target_marker, OpenArrow);
        assert_eq!(relationship_style_for_notation("depends_on"), relationship_style_for_notation("uml.dependency"));
        assert_eq!(relationship_style_for_notation("uml.aggregation").source_marker, Diamond);
        assert_eq!(relationship_style_for_notation("uml.composition").source_marker, FilledDiamond);
        assert_eq!(relationship_style_for_notation("uml.association").target_marker, None);
    }
    #[test]
    fn moving_class_reanchors_existing_cubic_dependency_without_losing_its_shape() {
        let project = serde_json::from_str(
            include_str!("../../bellatrix_mde_model/fixtures/todo.json")
        ).unwrap();
        let mut section = ModelingSection::from_model(&project);
        let diagram_id = "diagram-tasks";
        let original_view = section.diagrams[0].views.iter()
            .find(|view| view.subject.as_deref() == Some("ref-task")).unwrap().geometry.clone();
        let original_start = match &original_view {
            Geometry::Path { start, .. } => *start,
            _ => panic!("fixture dependency must be a curve"),
        };
        let changed_node = section.diagrams[0].views.iter_mut()
            .find(|view| view.subject.as_deref() == Some("type-list")).unwrap();
        changed_node.geometry.translate(0.0, -80.0);
        let saved_before = section.clone();
        let projection = RelationshipGraph::new(&section, diagram_id).paths();
        assert_eq!(section, saved_before, "rendering is a pure graph projection");
        let Geometry::Path { start, segments } = &projection["ref-task"] else {
            panic!("legacy dependency must remain a Path");
        };
        assert_ne!(*start, original_start, "source endpoint must follow the moved Class");
        assert!(matches!(segments.first(), Some(PathSegment::Cubic { .. })),
            "manually shaped curve should retain its cubic bend");
        reroute_relationships(&mut section, diagram_id);
        assert_eq!(section.diagrams[0].views.iter()
            .find(|view| view.subject.as_deref() == Some("ref-task")).unwrap().geometry,
            projection["ref-task"],
            "persisted geometry follows the same graph contract as the renderer");
        crate::Graph::new(&section.to_model("project-todo","Todo")).validate().unwrap();
    }

    #[test]
    fn model_explorer_and_canvas_query_identical_visible_edges() {
        let project = serde_json::from_str(
            include_str!("../../bellatrix_mde_model/fixtures/todo.json")
        ).unwrap();
        let section = ModelingSection::from_model(&project);
        let graph = RelationshipGraph::new(&section, "diagram-tasks");
        let ids = graph.visible_relations().iter().map(|r|r.id.as_str()).collect::<Vec<_>>();
        let projected = graph.paths();
        assert_eq!(ids, vec!["ref-task"]);
        assert_eq!(projected.keys().map(String::as_str).collect::<Vec<_>>(), ids);
        let presentation = relationship_presentation(graph.visible_relations()[0]);
        assert_eq!(presentation.label, "dependency");
        assert_eq!(presentation.style.target_marker, RelationshipMarker::OpenArrow);
    }

    #[test]
    fn quick_edit_composes_same_endpoints_without_reversing_roles() {
        use crate::UmlQuickCommand as A;
        assert_eq!(quick_relation_rule(A::AddSubtype).unwrap(), QuickRelationRule {
            kind:"uml.generalization",child_is_source:true,aggregation:None
        });
        assert_eq!(quick_relation_rule(A::AddSupertype).unwrap().child_is_source, false);
        assert_eq!(quick_relation_rule(A::AddProvidedInterface).unwrap().child_is_source, false);
        assert_eq!(quick_relation_rule(A::AddRealizingClass).unwrap().child_is_source, true);
        assert_eq!(quick_relation_rule(A::AddAggregatedClass).unwrap().aggregation, Some("shared"));
        assert_eq!(quick_relation_rule(A::AddCompositedClass).unwrap().aggregation, Some("composite"));
    }
}
