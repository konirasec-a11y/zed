//! Structural UML diagram validation against OMG UML 2.5.1 + UMLDI (20161101).
//! A domain Element/Relation is distinct from its UMLDI Shape/Edge View.
//! This is an opt-in, non-mutating conformance pass; existing Zed/Cargo models
//! remain valid while Bellatrix evolves to fully typed UML ownership/ends.
use crate::{Graph, UmlType};
use bellatrix_mde_model::{Geometry, Project, Relation};
use std::collections::{HashMap, HashSet};

/// Seven structural diagram metaclasses in OMG UMLDI (20161101).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum StructuralDiagramKind {
    Class,
    Object,
    Package,
    Component,
    CompositeStructure,
    Deployment,
    Profile,
}

impl StructuralDiagramKind {
    pub const ALL: [Self; 7] = [
        Self::Class, Self::Object, Self::Package, Self::Component,
        Self::CompositeStructure, Self::Deployment, Self::Profile,
    ];

    /// Names match xmi:id in the vendored OMG UMLDI.xmi.
    pub const fn uml_di_type(self) -> &'static str {
        match self {
            Self::Class => "UMLClassDiagram",
            Self::Object => "UMLObjectDiagram",
            Self::Package => "UMLPackageDiagram",
            Self::Component => "UMLComponentDiagram",
            Self::CompositeStructure => "UMLCompositeStructureDiagram",
            Self::Deployment => "UMLDeploymentDiagram",
            Self::Profile => "UMLProfileDiagram",
        }
    }
    pub fn from_uml_di_type(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.uml_di_type() == value)
    }
}

/// The UML 2.5.1 §7.2 Root hierarchy is not a visual shape hierarchy.
/// Relationship <: Element, DirectedRelationship <: Relationship, Comment <: Element.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootMetaclass {
    Element,
    Relationship,
    DirectedRelationship,
    Comment,
}
impl RootMetaclass {
    pub const fn parent(self) -> Option<Self> {
        match self {
            Self::Element => None,
            Self::Relationship | Self::Comment => Some(Self::Element),
            Self::DirectedRelationship => Some(Self::Relationship),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralSeverity {
    Error,
    Warning,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralDiagnostic {
    pub severity: StructuralSeverity,
    pub code: &'static str,
    pub subject_id: String,
    pub message: String,
}
fn problem(code: &'static str, id: &str, message: impl Into<String>) -> StructuralDiagnostic {
    StructuralDiagnostic {
        severity: StructuralSeverity::Error, code,
        subject_id: id.into(), message: message.into(),
    }
}

/// Compatibility aliases for the existing MDE graph. These are *not* claims
/// that `depends_on` or `uml.note_link` are OMG UML metaclass names.
pub fn relationship_root_kind(relation: &Relation) -> Option<RootMetaclass> {
    Some(match relation.kind.as_str() {
        "uml.association" | "uml.aggregation" | "uml.composition" =>
            RootMetaclass::Relationship,
        "uml.generalization" | "uml.dependency" | "uml.directed_association" |
        "uml.interface_realization" | "uml.realization" | "uml.template_binding" |
        "uml.containment" | "uml.note_link" | "uml.constraint_link" | "depends_on" =>
            RootMetaclass::DirectedRelationship,
        _ => return None,
    })
}
pub fn element_root_kind(kind: &str) -> RootMetaclass {
    if matches!(kind, "uml.comment" | "uml.note") {
        RootMetaclass::Comment
    } else {
        RootMetaclass::Element
    }
}

/// Verify the compatibility graph and the UMLDI shape/edge projection for
/// one structural diagram. It does not change persisted ProjectDocument data.
pub fn validate_structural_diagram(
    project: &Project, diagram_id: &str, kind: StructuralDiagramKind,
) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    if let Err(error) = Graph::new(project).validate() {
        issues.push(problem("UML_ROOT_INVALID", &project.id, error.to_string()));
        return issues;
    }
    let Some(diagram) = project.diagrams.iter().find(|d| d.id == diagram_id) else {
        issues.push(problem("UMLDI_DIAGRAM_MISSING", diagram_id, "diagram does not exist"));
        return issues;
    };
    let elements = project.elements.iter().map(|e| (e.id.as_str(), e)).collect::<HashMap<_, _>>();
    let relations = project.relations.iter().map(|r| (r.id.as_str(), r)).collect::<HashMap<_, _>>();
    let subjects = diagram.views.iter().filter_map(|v| v.subject.as_deref())
        .collect::<HashSet<_>>();

    // UMLDI UMLShape and UMLEdge are different diagram representations.
    // A relation must never be silently represented by a classifier rectangle.
    for view in &diagram.views {
        let Some(subject) = view.subject.as_deref() else { continue };
        if let Some(relation) = relations.get(subject) {
            match &view.geometry {
                Geometry::Path { segments, .. } if !segments.is_empty() => {}
                _ => issues.push(problem(
                    "UMLDI_EXPECTED_EDGE", &view.id,
                    format!("relationship {subject} needs a nonempty UMLDI Edge Path"),
                )),
            }
            if !subjects.contains(relation.from.as_str()) ||
                !subjects.contains(relation.to.as_str()) {
                issues.push(problem(
                    "UMLDI_ENDPOINT_NOT_VISIBLE", &view.id,
                    format!("{}: both endpoints must have Shapes in the same diagram", relation.id),
                ));
            }
        } else if let Some(element) = elements.get(subject) {
            if matches!(view.geometry, Geometry::Path { .. }) {
                issues.push(problem(
                    "UMLDI_EXPECTED_SHAPE", &view.id,
                    format!("element {} must have a UMLDI Shape geometry", element.id),
                ));
            }
            // Only the seven categories are structural; no generic claim
            // that a random project node is an OMG UML classifier.
            if element.kind.starts_with("uml.") &&
                UmlType::from_kind(&element.kind).is_none() &&
                !matches!(element.kind.as_str(),
                    "uml.component" | "uml.node" | "uml.device" |
                    "uml.execution_environment" | "uml.artifact" |
                    "uml.instance_specification" | "uml.profile" |
                    "uml.stereotype" | "uml.connector" | "uml.collaboration") {
                issues.push(problem("UML_UNMAPPED_METACLASS", &element.id,
                    format!("{} has no registered UML semantic interpretation for {:?}",element.kind,kind)));
            }
        }
    }

    for relation in &project.relations {
        // Relationships absent from this diagram need not have a View.
        if !subjects.contains(relation.id.as_str()) { continue; }
        let Some(source) = elements.get(relation.from.as_str()) else { continue };
        let Some(target) = elements.get(relation.to.as_str()) else { continue };
        if relation.kind == "uml.generalization" &&
           (!UmlType::from_kind(&source.kind).is_some_and(UmlType::is_classifier) ||
            !UmlType::from_kind(&target.kind).is_some_and(UmlType::is_classifier)) {
            issues.push(problem("UML_GENERALIZATION_CLASSIFIERS", &relation.id,
                "Generalization must link a specific Classifier to a general Classifier"));
        }
        if relation.kind == "uml.interface_realization" &&
            !matches!(target.kind.as_str(), "uml.interface" | "rust.trait") {
            issues.push(problem("UML_INTERFACE_REALIZATION_TARGET", &relation.id,
                "InterfaceRealization must target a UML Interface"));
        }
        if let Some(value) = relation.properties.get("end1Aggregation") {
            if relation.kind != "uml.association" ||
               !matches!(value.as_str(), Some("none" | "shared" | "composite")) {
                issues.push(problem("UML_ASSOCIATION_AGGREGATION", &relation.id,
                    "end1Aggregation belongs to Association and must be none, shared or composite"));
            }
        }
        if relationship_root_kind(relation).is_none() && relation.kind.starts_with("uml.") {
            issues.push(problem("UML_UNKNOWN_RELATIONSHIP", &relation.id,
                format!("unknown UML relationship kind {}", relation.kind)));
        }
    }
    // Generalization cycles are invalid for structural classifiers, but
    // ordinary dependency cycles are NOT globally forbidden by UML.
    let mut links: HashMap<&str, Vec<&str>> = HashMap::new();
    for relation in project.relations.iter().filter(|r|
        r.kind == "uml.generalization" && subjects.contains(r.id.as_str())) {
        links.entry(&relation.from).or_default().push(&relation.to);
    }
    fn cycle<'a>(
        at: &'a str, graph: &HashMap<&'a str, Vec<&'a str>>,
        active: &mut HashSet<&'a str>, visited: &mut HashSet<&'a str>,
    ) -> bool {
        if active.contains(at) { return true; }
        if !visited.insert(at) { return false; }
        active.insert(at);
        let found = graph.get(at).is_some_and(|outgoing|
            outgoing.iter().any(|next| cycle(next, graph, active, visited)));
        active.remove(at);
        found
    }
    let mut visited=HashSet::new();
    if links.keys().any(|node| cycle(node, &links, &mut HashSet::new(), &mut visited)) {
        issues.push(problem("UML_GENERALIZATION_CYCLE", diagram_id,
            "Generalization cannot contain cycles of Classifiers"));
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use bellatrix_mde_model::{Geometry, PathSegment, Point, Relation, View, Style};
    fn fixture() -> Project {
        serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap()
    }
    #[test]
    fn structural_diagrams_are_the_seven_uml_di_classes() {
        const XML: &str = include_str!("../../../docs/bellatrix/standards/uml/20161101/UMLDI.xmi");
        for kind in StructuralDiagramKind::ALL {
            assert!(XML.contains(&format!("xmi:id=\"{}\"",kind.uml_di_type())));
            assert_eq!(StructuralDiagramKind::from_uml_di_type(kind.uml_di_type()),Some(kind));
        }
        assert_eq!(StructuralDiagramKind::from_uml_di_type("UMLBehaviorDiagram"),None);
    }
    #[test]
    fn root_specialization_matches_uml_251() {
        const XML: &str = include_str!("../../../docs/bellatrix/standards/uml/20161101/UML.xmi");
        for name in ["Element","Relationship","DirectedRelationship","Comment"] {
            assert!(XML.contains(&format!("xmi:id=\"{name}\"")));
        }
        assert_eq!(RootMetaclass::DirectedRelationship.parent(),Some(RootMetaclass::Relationship));
        assert_eq!(RootMetaclass::Comment.parent(),Some(RootMetaclass::Element));
    }
    #[test]
    fn legacy_class_diagram_validates_without_migrating_data() {
        let project=fixture();
        assert!(validate_structural_diagram(&project,"diagram-tasks",StructuralDiagramKind::Class).is_empty());
    }
    #[test]
    fn edges_must_be_paths_not_class_rectangles() {
        let mut project=fixture();
        let view=project.diagrams[0].views.iter_mut().find(|v|v.subject.as_deref()==Some("ref-task")).unwrap();
        view.geometry=Geometry::Rect{x:1.0,y:2.0,width:90.0,height:20.0};
        let failures=validate_structural_diagram(&project,"diagram-tasks",StructuralDiagramKind::Class);
        assert!(failures.iter().any(|f|f.code=="UMLDI_EXPECTED_EDGE"));
    }
    #[test]
    fn edge_endpoints_must_be_visible_on_the_same_diagram() {
        let mut project=fixture();
        project.diagrams[0].views.retain(|v|v.subject.as_deref()!=Some("type-task"));
        let failures=validate_structural_diagram(&project,"diagram-tasks",StructuralDiagramKind::Class);
        assert!(failures.iter().any(|f|f.code=="UMLDI_ENDPOINT_NOT_VISIBLE"));
    }
    #[test]
    fn invalid_aggregation_property_is_diagnosed() {
        let mut project=fixture();
        let relation=project.relations.iter_mut().find(|r|r.id=="ref-task").unwrap();
        relation.kind="uml.association".into();
        relation.properties.insert("end1Aggregation".into(),serde_json::json!("illegal"));
        let failures=validate_structural_diagram(&project,"diagram-tasks",StructuralDiagramKind::Class);
        assert!(failures.iter().any(|f|f.code=="UML_ASSOCIATION_AGGREGATION"));
    }
    #[test]
    fn cyclic_generalization_is_detected_without_outlawing_dependencies() {
        let mut project=fixture();
        let relation=project.relations.iter_mut().find(|r|r.id=="ref-task").unwrap();
        relation.kind="uml.generalization".into();
        project.relations.push(Relation {
            id:"inverse-generalization".into(),kind:"uml.generalization".into(),
            from:"type-task".into(),to:"type-list".into(),properties:Default::default(),
        });
        project.diagrams[0].views.push(View {
            id:"view-inverse".into(),subject:Some("inverse-generalization".into()),
            label:None,geometry:Geometry::Path{
                start:Point{x:1.0,y:1.0},segments:vec![PathSegment::Line{to:Point{x:4.0,y:4.0}}],
            },style:Style::default(),z_index:20,
        });
        assert!(validate_structural_diagram(&project,"diagram-tasks",StructuralDiagramKind::Class)
            .iter().any(|d|d.code=="UML_GENERALIZATION_CYCLE"));
    }
}
