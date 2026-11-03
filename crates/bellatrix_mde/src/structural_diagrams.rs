//! Initial compatibility-graph diagnostics, centralized in the MDE domain.
//! This is NOT a complete UML/UMLDI conformance validator. Diagram categories
//! are identified here, but specialized constraints are not all implemented.
//! Policy restrictions of the current editor must not be advertised as OMG rules.
use crate::{Geometry, Graph, Project, Relation, UmlType};
use std::collections::{HashMap, HashSet};

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
        Self::Class,
        Self::Object,
        Self::Package,
        Self::Component,
        Self::CompositeStructure,
        Self::Deployment,
        Self::Profile,
    ];
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
        Self::ALL
            .into_iter()
            .find(|kind| kind.uml_di_type() == value)
    }
}

/// UML 2.5.1 section 7.2; semantic inheritance is distinct from visual geometry.
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
        severity: StructuralSeverity::Error,
        code,
        subject_id: id.into(),
        message: message.into(),
    }
}

/// Compatibility classification retained during the crate migration. This
/// table still includes editor-specific links; it is not a UML metaclass registry.
pub fn relationship_root_kind(relation: &Relation) -> Option<RootMetaclass> {
    Some(match relation.kind.as_str() {
        "uml.association" => RootMetaclass::Relationship,
        "uml.generalization"
        | "uml.dependency"
        | "uml.usage"
        | "uml.directed_association"
        | "uml.interface_realization"
        | "uml.realization"
        | "uml.template_binding"
        | "uml.containment"
        | "uml.note_link"
        | "uml.constraint_link"
        | "depends_on" => RootMetaclass::DirectedRelationship,
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

/// Combines whole-model semantic diagnostics with the selected projection. The `kind`
/// identifies the requested diagram category; it does not imply full coverage
/// of that category's normative constraints. The required stored type must
/// match the requested kind. Context is distinct from Views: CompositeStructure
/// needs one qualifying context; the other six kinds prohibit context but allow Views.
/// Unknown future type names remain serializable, but cannot masquerade as a
/// supported structural kind when this explicit-kind validator is called.
pub fn validate_structural_diagram(
    project: &Project,
    diagram_id: &str,
    kind: StructuralDiagramKind,
) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    if let Err(error) = Graph::new(project).validate() {
        issues.push(problem("UML_ROOT_INVALID", &project.id, error.to_string()));
        return issues;
    }
    issues.extend(validate_structural_model(project));
    let Some(diagram) = project
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
    else {
        issues.push(problem(
            "UMLDI_DIAGRAM_MISSING",
            diagram_id,
            "diagram does not exist",
        ));
        return issues;
    };
    let elements = project
        .elements
        .iter()
        .map(|element| (element.id.as_str(), element))
        .collect::<HashMap<_, _>>();
    let stored_type = &diagram.uml_di_type;
    if StructuralDiagramKind::from_uml_di_type(stored_type).is_none() {
        issues.push(problem(
            "UMLDI_UNSUPPORTED_DIAGRAM_TYPE",
            &diagram.id,
            format!("unsupported stored UMLDI type {stored_type}"),
        ));
        return issues;
    }
    if stored_type != kind.uml_di_type() {
        issues.push(problem(
            "UMLDI_DIAGRAM_KIND_MISMATCH",
            &diagram.id,
            format!(
                "stored UMLDI type {stored_type} does not match requested {}",
                kind.uml_di_type()
            ),
        ));
    }
    // UMLDI.xmi: the six *-no_modelElement rules concern the diagram's own
    // context, never the subjects of its contained Views.
    if kind != StructuralDiagramKind::CompositeStructure {
        if !diagram.model_elements.is_empty() {
            issues.push(problem(
                "UMLDI_UNEXPECTED_CONTEXT",
                &diagram.id,
                "This UMLDI diagram kind must not have a modelElement context",
            ));
        }
    } else {
        // UMLCompositeStructureDiagram-modelElement_mult / modelElement_type.
        if diagram.model_elements.len() != 1 {
            issues.push(problem(
                "UMLDI_CONTEXT_MULTIPLICITY",
                &diagram.id,
                "A UMLCompositeStructureDiagram requires exactly one modelElement context",
            ));
        }
        for context_id in &diagram.model_elements {
            match elements.get(context_id.as_str()) {
                None => issues.push(problem(
                    "UMLDI_CONTEXT_MISSING",
                    context_id,
                    "The diagram context does not identify a model Element",
                )),
                Some(context) if context.kind == "uml.instance_specification" => {
                    // The active domain has no established instance-classifier
                    // adapter; do not infer conformance from unvalidated properties.
                    issues.push(StructuralDiagnostic { severity: StructuralSeverity::Warning,
                        code: "UMLDI_INSTANCE_CONTEXT_UNSUPPORTED", subject_id: context_id.clone(),
                        message: "InstanceSpecification context classifier conformance cannot yet be verified".into() });
                }
                Some(context)
                    if !UmlType::from_kind(&context.kind)
                        .is_some_and(UmlType::is_structured_classifier) =>
                {
                    issues.push(problem("UMLDI_CONTEXT_TYPE", context_id,
                        "Composite structure context must be a StructuredClassifier or a conforming InstanceSpecification"));
                }
                Some(_) => {}
            }
        }
    }
    let relations = project
        .relations
        .iter()
        .map(|relation| (relation.id.as_str(), relation))
        .collect::<HashMap<_, _>>();
    let subjects = diagram
        .views
        .iter()
        .filter_map(|view| view.subject.as_deref())
        .collect::<HashSet<_>>();
    for view in &diagram.views {
        let Some(subject) = view.subject.as_deref() else {
            continue;
        };
        if let Some(relation) = relations.get(subject) {
            match &view.geometry {
                Geometry::Path { segments, .. } if !segments.is_empty() => {}
                _ => issues.push(problem("UMLDI_EXPECTED_EDGE", &view.id,
                    format!("relationship {subject} needs a nonempty Edge Path in the current editor projection"))),
            }
            if !subjects.contains(relation.from.as_str())
                || !subjects.contains(relation.to.as_str())
            {
                issues.push(problem(
                    "UMLDI_ENDPOINT_NOT_VISIBLE",
                    &view.id,
                    format!(
                        "{}: the current editor requires both endpoint Shapes in the same diagram",
                        relation.id
                    ),
                ));
            }
        } else if let Some(element) = elements.get(subject)
            && matches!(&view.geometry, Geometry::Path { .. })
        {
            issues.push(problem(
                "UMLDI_EXPECTED_SHAPE",
                &view.id,
                format!(
                    "element {} needs Shape geometry in the current editor projection",
                    element.id
                ),
            ));
        }
    }
    issues
}

/// Checks the represented UML relationship semantics across the whole model,
/// independently of Views. This is not full UML conformance or graph integrity
/// validation; callers also use Graph::validate for IDs and references.
pub fn validate_structural_model(project: &Project) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    let elements = project
        .elements
        .iter()
        .map(|element| (element.id.as_str(), element))
        .collect::<HashMap<_, _>>();
    for relation in &project.relations {
        if matches!(
            relation.kind.as_str(),
            "uml.aggregation" | "uml.composition"
        ) {
            issues.push(problem(
                "UML_UNSUPPORTED_RELATIONSHIP_FORMAT", &relation.id,
                "Unsupported old relationship format: use canonical uml.association with end1Aggregation set to shared or composite",
            ));
            continue;
        }
        let Some(source) = elements.get(relation.from.as_str()) else {
            continue;
        };
        let Some(target) = elements.get(relation.to.as_str()) else {
            continue;
        };
        if relation.kind == "uml.generalization"
            && (!UmlType::from_kind(&source.kind).is_some_and(UmlType::is_classifier)
                || !UmlType::from_kind(&target.kind).is_some_and(UmlType::is_classifier))
        {
            issues.push(problem(
                "UML_GENERALIZATION_CLASSIFIERS",
                &relation.id,
                "Generalization must link a specific Classifier to a general Classifier",
            ));
        }
        if relation.kind == "uml.generalization" {
            let source_type = UmlType::from_kind(&source.kind);
            let target_type = UmlType::from_kind(&target.kind);
            // UML.xmi Stereotype-generalize applies in both directions.
            if (source_type == Some(UmlType::Stereotype))
                != (target_type == Some(UmlType::Stereotype))
            {
                issues.push(problem(
                    "UML_STEREOTYPE_GENERALIZATION",
                    &relation.id,
                    "Stereotype may only generalize or specialize another Stereotype",
                ));
            }
            // UML.xmi Classifier-specialize_type / Classifier-maySpecializeType-spec.
            if let (Some(specific), Some(general)) = (source_type, target_type)
                && specific.may_specialize_type(general) == Some(false)
            {
                issues.push(problem("UML_GENERALIZATION_TYPE", &relation.id,
                    "The specific Classifier metaclass must conform to the general Classifier metaclass"));
            }
            // UML.xmi Classifier-non_final_parents. Do not substitute isLeaf:
            // the editor represents isFinalSpecialization as its own Boolean.
            if target
                .properties
                .get("isFinalSpecialization")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
            {
                issues.push(problem(
                    "UML_GENERALIZATION_FINAL_PARENT",
                    &relation.id,
                    "A Classifier cannot specialize a final Classifier",
                ));
            }
        }
        if relation.kind == "uml.interface_realization"
            && !matches!(target.kind.as_str(), "uml.interface" | "rust.trait")
        {
            issues.push(problem(
                "UML_INTERFACE_REALIZATION_TARGET",
                &relation.id,
                "InterfaceRealization must target a UML Interface",
            ));
        }
        if let Some(value) = relation.properties.get("end1Aggregation")
            && (relation.kind != "uml.association"
                || !matches!(value.as_str(), Some("none" | "shared" | "composite")))
        {
            issues.push(problem(
                "UML_ASSOCIATION_AGGREGATION",
                &relation.id,
                "end1Aggregation belongs to Association and must be none, shared or composite",
            ));
        }
        if relationship_root_kind(relation).is_none() && relation.kind.starts_with("uml.") {
            issues.push(problem(
                "UML_UNKNOWN_RELATIONSHIP",
                &relation.id,
                format!("unknown UML relationship kind {}", relation.kind),
            ));
        }
    }
    // UML.xmi Classifier-no_cycles_in_generalization constrains the model,
    // even when one or all generalizations have no diagram representation.
    if !crate::GraphIndex::new(project)
        .cyclic_components("uml.generalization")
        .is_empty()
    {
        issues.push(problem(
            "UML_GENERALIZATION_CYCLE",
            &project.id,
            "Generalization cannot contain cycles of Classifiers",
        ));
    }
    issues.extend(crate::structural_rules::validate(project));
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PathSegment, Point, Style, View};
    fn fixture() -> Project {
        serde_json::from_str(include_str!("../fixtures/todo.json")).unwrap()
    }
    #[test]
    fn structural_diagrams_are_the_seven_uml_di_classes() {
        const XML: &str = include_str!("../../../docs/bellatrix/standards/uml/20161101/UMLDI.xmi");
        for kind in StructuralDiagramKind::ALL {
            assert!(XML.contains(&format!("xmi:id=\"{}\"", kind.uml_di_type())));
            assert_eq!(
                StructuralDiagramKind::from_uml_di_type(kind.uml_di_type()),
                Some(kind)
            );
        }
        assert_eq!(
            StructuralDiagramKind::from_uml_di_type("UMLBehaviorDiagram"),
            None
        );
    }
    #[test]
    fn root_specialization_matches_uml_251() {
        const XML: &str = include_str!("../../../docs/bellatrix/standards/uml/20161101/UML.xmi");
        for name in ["Element", "Relationship", "DirectedRelationship", "Comment"] {
            assert!(XML.contains(&format!("xmi:id=\"{name}\"")));
        }
        assert_eq!(
            RootMetaclass::DirectedRelationship.parent(),
            Some(RootMetaclass::Relationship)
        );
        assert_eq!(
            RootMetaclass::Comment.parent(),
            Some(RootMetaclass::Element)
        );
    }
    #[test]
    fn explicitly_typed_class_diagram_validates() {
        assert!(
            validate_structural_diagram(&fixture(), "diagram-tasks", StructuralDiagramKind::Class)
                .is_empty()
        );
    }
    #[test]
    fn edges_must_be_paths_not_class_rectangles() {
        let mut project = fixture();
        project.diagrams[0]
            .views
            .iter_mut()
            .find(|view| view.subject.as_deref() == Some("ref-task"))
            .unwrap()
            .geometry = Geometry::Rect {
            x: 1.0,
            y: 2.0,
            width: 90.0,
            height: 20.0,
        };
        assert!(
            validate_structural_diagram(&project, "diagram-tasks", StructuralDiagramKind::Class)
                .iter()
                .any(|failure| failure.code == "UMLDI_EXPECTED_EDGE")
        );
    }
    #[test]
    fn edge_endpoints_must_be_visible_on_the_same_diagram() {
        let mut project = fixture();
        project.diagrams[0]
            .views
            .retain(|view| view.subject.as_deref() != Some("type-task"));
        assert!(
            validate_structural_diagram(&project, "diagram-tasks", StructuralDiagramKind::Class)
                .iter()
                .any(|failure| failure.code == "UMLDI_ENDPOINT_NOT_VISIBLE")
        );
    }
    #[test]
    fn invalid_aggregation_property_is_diagnosed() {
        let mut project = fixture();
        let relation = project
            .relations
            .iter_mut()
            .find(|relation| relation.id == "ref-task")
            .unwrap();
        relation.kind = "uml.association".into();
        relation
            .properties
            .insert("end1Aggregation".into(), serde_json::json!("illegal"));
        assert!(
            validate_structural_diagram(&project, "diagram-tasks", StructuralDiagramKind::Class)
                .iter()
                .any(|failure| failure.code == "UML_ASSOCIATION_AGGREGATION")
        );
    }
    #[test]
    fn cyclic_generalization_is_detected_without_outlawing_dependencies() {
        let mut project = fixture();
        project
            .relations
            .iter_mut()
            .find(|relation| relation.id == "ref-task")
            .unwrap()
            .kind = "uml.generalization".into();
        project.relations.push(Relation {
            id: "inverse-generalization".into(),
            kind: "uml.generalization".into(),
            from: "type-task".into(),
            to: "type-list".into(),
            properties: Default::default(),
        });
        project.diagrams[0].views.push(View {
            id: "view-inverse".into(),
            subject: Some("inverse-generalization".into()),
            label: None,
            geometry: Geometry::Path {
                start: Point { x: 1.0, y: 1.0 },
                segments: vec![PathSegment::Line {
                    to: Point { x: 4.0, y: 4.0 },
                }],
            },
            style: Style::default(),
            z_index: 20,
        });
        assert!(
            validate_structural_diagram(&project, "diagram-tasks", StructuralDiagramKind::Class)
                .iter()
                .any(|failure| failure.code == "UML_GENERALIZATION_CYCLE")
        );
        for diagram in &mut project.diagrams {
            diagram
                .views
                .retain(|view| view.subject.as_deref() != Some("inverse-generalization"));
        }
        assert!(
            validate_structural_diagram(&project, "diagram-tasks", StructuralDiagramKind::Class)
                .iter()
                .any(|failure| failure.code == "UML_GENERALIZATION_CYCLE")
        );
    }

    fn generalization_project(specific: &str, general: &str) -> serde_json::Result<Project> {
        serde_json::from_value(serde_json::json!({
            "id": "model", "name": "Generalization rules",
            "elements": [
                {"id": "specific", "kind": specific, "name": "Specific"},
                {"id": "general", "kind": general, "name": "General"}
            ],
            "relations": [{"id": "specialization", "kind": "uml.generalization",
                "from": "specific", "to": "general"}],
            "diagrams": [{"id": "empty-diagram", "name": "No visible relationships", "uml_di_type": "UMLClassDiagram"}]
        }))
    }

    #[test]
    fn hidden_generalization_cycles_are_model_errors_for_every_diagram_kind()
    -> serde_json::Result<()> {
        let mut project = generalization_project("uml.class", "uml.class")?;
        project.relations.push(Relation {
            id: "hidden-return".into(),
            kind: "uml.generalization".into(),
            from: "general".into(),
            to: "specific".into(),
            properties: Default::default(),
        });
        for kind in StructuralDiagramKind::ALL {
            let issues = validate_structural_diagram(&project, "empty-diagram", kind);
            assert!(issues.iter().any(
                |issue| issue.code == "UML_GENERALIZATION_CYCLE" && issue.subject_id == "model"
            ));
        }
        project.diagrams.clear();
        assert!(
            validate_structural_model(&project)
                .iter()
                .any(|issue| issue.code == "UML_GENERALIZATION_CYCLE")
        );
        Ok(())
    }

    #[test]
    fn hidden_generalizations_require_classifier_endpoints() -> serde_json::Result<()> {
        let project = generalization_project("uml.package", "uml.class")?;
        assert!(
            validate_structural_model(&project)
                .iter()
                .any(|issue| issue.code == "UML_GENERALIZATION_CLASSIFIERS")
        );
        Ok(())
    }

    #[test]
    fn structural_classifier_specializations_use_semantics_not_view_kinds() -> serde_json::Result<()>
    {
        for kind in [
            "uml.component",
            "uml.node",
            "uml.device",
            "uml.execution_environment",
            "uml.artifact",
            "uml.stereotype",
            "uml.collaboration",
        ] {
            let project = generalization_project(kind, kind)?;
            assert!(validate_structural_model(&project).is_empty(), "{kind}");
        }
        Ok(())
    }

    #[test]
    fn stereotype_generalization_is_restricted_in_both_directions() -> serde_json::Result<()> {
        for (specific, general) in [
            ("uml.stereotype", "uml.class"),
            ("uml.class", "uml.stereotype"),
        ] {
            let project = generalization_project(specific, general)?;
            assert!(
                validate_structural_model(&project)
                    .iter()
                    .any(|issue| issue.code == "UML_STEREOTYPE_GENERALIZATION")
            );
        }
        Ok(())
    }

    #[test]
    fn specialization_requires_conforming_metaclasses() -> serde_json::Result<()> {
        for (specific, general) in [
            ("uml.component", "uml.class"),
            ("uml.device", "uml.node"),
            ("uml.enumeration", "uml.data_type"),
            ("rust.struct", "uml.class"),
        ] {
            assert!(
                validate_structural_model(&generalization_project(specific, general)?).is_empty()
            );
        }
        for (specific, general) in [
            ("uml.class", "uml.component"),
            ("uml.node", "uml.device"),
            ("uml.class", "uml.interface"),
            ("uml.primitive_type", "uml.enumeration"),
        ] {
            assert!(
                validate_structural_model(&generalization_project(specific, general)?)
                    .iter()
                    .any(|issue| issue.code == "UML_GENERALIZATION_TYPE")
            );
        }
        Ok(())
    }

    #[test]
    fn final_parent_is_not_confused_with_leaf_or_abstract() -> serde_json::Result<()> {
        let mut project = generalization_project("uml.class", "uml.class")?;
        for element in &mut project.elements {
            if element.id == "general" {
                element
                    .properties
                    .insert("isLeaf".into(), serde_json::json!(true));
                element
                    .properties
                    .insert("isAbstract".into(), serde_json::json!(true));
                element
                    .properties
                    .insert("isFinalSpecialization".into(), serde_json::json!(false));
            }
        }
        assert!(validate_structural_model(&project).is_empty());
        for element in &mut project.elements {
            if element.id == "general" {
                element
                    .properties
                    .insert("isFinalSpecialization".into(), serde_json::json!(true));
            }
        }
        assert!(
            validate_structural_model(&project)
                .iter()
                .any(|issue| issue.code == "UML_GENERALIZATION_FINAL_PARENT")
        );
        Ok(())
    }

    #[test]
    fn dependency_cycles_are_not_generalization_cycles() -> serde_json::Result<()> {
        let mut project = generalization_project("uml.class", "uml.class")?;
        for relation in &mut project.relations {
            relation.kind = "uml.dependency".into();
        }
        project.relations.push(Relation {
            id: "return-dependency".into(),
            kind: "uml.dependency".into(),
            from: "general".into(),
            to: "specific".into(),
            properties: Default::default(),
        });
        assert!(validate_structural_model(&project).is_empty());
        Ok(())
    }

    #[test]
    fn acyclic_diamond_is_valid_but_self_generalization_is_not() -> serde_json::Result<()> {
        let mut project = generalization_project("uml.class", "uml.class")?;
        project
            .elements
            .extend(serde_json::from_value::<Vec<crate::Element>>(
                serde_json::json!([
                    {"id": "left", "name": "Left", "kind": "uml.class"},
                    {"id": "right", "name": "Right", "kind": "uml.class"}
                ]),
            )?);
        for (identifier, specific, general) in [
            ("left-base", "left", "general"),
            ("right-base", "right", "general"),
            ("leaf-left", "specific", "left"),
            ("leaf-right", "specific", "right"),
        ] {
            project.relations.push(Relation {
                id: identifier.into(),
                kind: "uml.generalization".into(),
                from: specific.into(),
                to: general.into(),
                properties: Default::default(),
            });
        }
        assert!(validate_structural_model(&project).is_empty());
        project.relations.push(Relation {
            id: "self".into(),
            kind: "uml.generalization".into(),
            from: "specific".into(),
            to: "specific".into(),
            properties: Default::default(),
        });
        assert!(
            validate_structural_model(&project)
                .iter()
                .any(|issue| issue.code == "UML_GENERALIZATION_CYCLE")
        );
        Ok(())
    }

    fn context_project(
        kind: StructuralDiagramKind,
        context_kind: &str,
        contexts: &[&str],
    ) -> serde_json::Result<Project> {
        serde_json::from_value(serde_json::json!({
            "id": "context-model", "name": "Diagram context",
            "elements": [{"id": "context", "kind": context_kind, "name": "Context"}],
            "diagrams": [{"id": "context-diagram", "name": "Context",
                "uml_di_type": kind.uml_di_type(), "model_elements": contexts}]
        }))
    }

    #[test]
    fn six_diagram_kinds_forbid_context_not_contained_views() -> serde_json::Result<()> {
        for kind in StructuralDiagramKind::ALL
            .into_iter()
            .filter(|kind| *kind != StructuralDiagramKind::CompositeStructure)
        {
            let mut project = context_project(kind, "uml.class", &[])?;
            for diagram in &mut project.diagrams {
                diagram.views.push(View {
                    id: "visible-context".into(),
                    subject: Some("context".into()),
                    label: None,
                    geometry: Geometry::Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 20.0,
                        height: 20.0,
                    },
                    style: Style::default(),
                    z_index: 0,
                });
            }
            assert!(validate_structural_diagram(&project, "context-diagram", kind).is_empty());
            for diagram in &mut project.diagrams {
                diagram.model_elements.push("context".into());
            }
            assert!(
                validate_structural_diagram(&project, "context-diagram", kind)
                    .iter()
                    .any(|issue| issue.code == "UMLDI_UNEXPECTED_CONTEXT")
            );
        }
        Ok(())
    }

    #[test]
    fn composite_structure_context_uses_structured_classifier_ancestry() -> serde_json::Result<()> {
        let kind = StructuralDiagramKind::CompositeStructure;
        for context_kind in [
            "uml.class",
            "rust.struct",
            "uml.association_class",
            "uml.component",
            "uml.node",
            "uml.device",
            "uml.execution_environment",
            "uml.stereotype",
            "uml.collaboration",
        ] {
            let project = context_project(kind, context_kind, &["context"])?;
            assert!(
                validate_structural_diagram(&project, "context-diagram", kind).is_empty(),
                "{context_kind}"
            );
        }
        for context_kind in [
            "uml.package",
            "uml.interface",
            "uml.artifact",
            "uml.data_type",
            "uml.signal",
        ] {
            let project = context_project(kind, context_kind, &["context"])?;
            assert!(
                validate_structural_diagram(&project, "context-diagram", kind)
                    .iter()
                    .any(|issue| issue.code == "UMLDI_CONTEXT_TYPE"),
                "{context_kind}"
            );
        }
        Ok(())
    }

    #[test]
    fn composite_structure_requires_one_resolved_context() -> serde_json::Result<()> {
        let kind = StructuralDiagramKind::CompositeStructure;
        for contexts in [vec![], vec!["context", "context"]] {
            let project = context_project(kind, "uml.class", &contexts)?;
            assert!(
                validate_structural_diagram(&project, "context-diagram", kind)
                    .iter()
                    .any(|issue| issue.code == "UMLDI_CONTEXT_MULTIPLICITY")
            );
        }
        let project = context_project(kind, "uml.class", &["missing"])?;
        assert!(
            validate_structural_diagram(&project, "context-diagram", kind)
                .iter()
                .any(|issue| issue.code == "UMLDI_CONTEXT_MISSING")
        );
        let project = context_project(kind, "uml.instance_specification", &["context"])?;
        assert!(
            validate_structural_diagram(&project, "context-diagram", kind)
                .iter()
                .any(|issue| issue.code == "UMLDI_INSTANCE_CONTEXT_UNSUPPORTED")
        );
        Ok(())
    }

    #[test]
    fn stored_kind_must_match_requested_kind_and_unknown_is_not_reinterpreted()
    -> serde_json::Result<()> {
        let mut project = context_project(StructuralDiagramKind::Class, "uml.class", &[])?;
        assert!(
            validate_structural_diagram(
                &project,
                "context-diagram",
                StructuralDiagramKind::Package
            )
            .iter()
            .any(|issue| issue.code == "UMLDI_DIAGRAM_KIND_MISMATCH")
        );
        for diagram in &mut project.diagrams {
            diagram.uml_di_type = "UMLFutureDiagram".into();
        }
        assert!(
            validate_structural_diagram(
                &project,
                "context-diagram",
                StructuralDiagramKind::Package
            )
            .iter()
            .any(|issue| issue.code == "UMLDI_UNSUPPORTED_DIAGRAM_TYPE")
        );
        Ok(())
    }
    #[test]
    fn obsolete_aggregation_kinds_are_rejected_but_canonical_association_ends_are_valid()
    -> serde_json::Result<()> {
        for (obsolete, aggregation) in [
            ("uml.aggregation", "shared"),
            ("uml.composition", "composite"),
        ] {
            let mut project = generalization_project("uml.class", "uml.class")?;
            for relation in &mut project.relations {
                relation.kind = obsolete.into();
                assert_eq!(relationship_root_kind(relation), None);
            }
            assert!(
                validate_structural_model(&project)
                    .iter()
                    .any(
                        |diagnostic| diagnostic.code == "UML_UNSUPPORTED_RELATIONSHIP_FORMAT"
                            && diagnostic.message.contains("uml.association")
                            && diagnostic.message.contains("end1Aggregation")
                    )
            );
            for relation in &mut project.relations {
                relation.kind = "uml.association".into();
                relation
                    .properties
                    .insert("end1Aggregation".into(), serde_json::json!(aggregation));
                assert_eq!(
                    relationship_root_kind(relation),
                    Some(RootMetaclass::Relationship)
                );
            }
            assert!(validate_structural_model(&project).is_empty());
        }
        Ok(())
    }
}
