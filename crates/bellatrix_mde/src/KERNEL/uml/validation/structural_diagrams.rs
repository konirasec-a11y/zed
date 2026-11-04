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
    UseCase,
    Activity,
    StateMachine,
    Sequence,
    Communication,
    InteractionOverview,
    Timing,
}
impl StructuralDiagramKind {
    pub const ALL_DIAGRAMS: [Self; 14] = [
        Self::Class,
        Self::Object,
        Self::Package,
        Self::Component,
        Self::CompositeStructure,
        Self::Deployment,
        Self::Profile,
        Self::UseCase,
        Self::Activity,
        Self::StateMachine,
        Self::Sequence,
        Self::Communication,
        Self::InteractionOverview,
        Self::Timing,
    ];
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
            Self::UseCase => "UMLUseCaseDiagram",
            Self::Activity => "UMLActivityDiagram",
            Self::StateMachine => "UMLStateMachineDiagram",
            Self::Sequence | Self::Communication | Self::InteractionOverview | Self::Timing => {
                "UMLInteractionDiagram"
            }
        }
    }
    pub const fn interaction_kind(self) -> Option<&'static str> {
        match self {
            Self::Sequence => Some("sequence"),
            Self::Communication => Some("communication"),
            Self::InteractionOverview => Some("overview"),
            Self::Timing => Some("timing"),
            _ => None,
        }
    }
    pub const fn context_metaclass(self) -> Option<&'static str> {
        match self {
            Self::Activity => Some("Activity"),
            Self::StateMachine => Some("StateMachine"),
            Self::Sequence | Self::Communication | Self::InteractionOverview | Self::Timing => {
                Some("Interaction")
            }
            _ => None,
        }
    }
    pub fn from_diagram(diagram: &crate::Diagram) -> Option<Self> {
        if diagram.uml_di_type == "UMLInteractionDiagram" {
            return match diagram.interaction_kind.as_deref().unwrap_or("sequence") {
                "sequence" => Some(Self::Sequence),
                "communication" => Some(Self::Communication),
                "overview" => Some(Self::InteractionOverview),
                "timing" => Some(Self::Timing),
                _ => None,
            };
        }
        Self::from_uml_di_type(&diagram.uml_di_type)
    }
    pub fn from_uml_di_type(value: &str) -> Option<Self> {
        Self::ALL_DIAGRAMS
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

fn context_type_diagnostic(
    context: &crate::Element,
    expected: &str,
) -> Option<StructuralDiagnostic> {
    match crate::uml::root::object_metaclass(&context.kind) {
        Some(metaclass) if crate::uml::metamodel::conforms_to(metaclass, expected) => None,
        Some(_) => Some(problem(
            "UMLDI_CONTEXT_TYPE",
            &context.id,
            format!("This diagram requires a {expected} context"),
        )),
        None => Some(StructuralDiagnostic {
            severity: StructuralSeverity::Warning,
            code: "UMLDI_CONTEXT_TYPE_UNVERIFIED",
            subject_id: context.id.clone(),
            message: format!(
                "Context metaclass is unresolved; {expected} conformance cannot be verified"
            ),
        }),
    }
}

fn root_kind(metaclass: &str) -> RootMetaclass {
    if crate::uml::metamodel::conforms_to(metaclass, "Comment") {
        RootMetaclass::Comment
    } else if crate::uml::metamodel::conforms_to(metaclass, "DirectedRelationship") {
        RootMetaclass::DirectedRelationship
    } else if crate::uml::metamodel::conforms_to(metaclass, "Relationship") {
        RootMetaclass::Relationship
    } else {
        RootMetaclass::Element
    }
}
pub fn relationship_root_kind(relation: &Relation) -> Option<RootMetaclass> {
    crate::uml::root::relationship_metaclass(&relation.kind).map(root_kind)
}
pub fn element_root_kind(kind: &str) -> Option<RootMetaclass> {
    crate::uml::root::object_metaclass(kind).map(root_kind)
}

/// Combines whole-model semantic diagnostics with the selected projection. The `kind`
/// identifies the requested diagram category; it does not imply full coverage
/// of that category's normative constraints. The required stored type must
/// match the requested kind. Context is distinct from Views: CompositeStructure
/// needs one qualifying context; the other six kinds prohibit context but allow Views.
/// Unknown future type names remain serializable, but cannot masquerade as a
/// supported structural kind when this explicit-kind validator is called.
/// Diagnostics are ordered by code, subject identity, severity and message,
/// matching the function-only boundary for legacy frontend consumers as well.
/// Input member order is not changed; only the resulting report is sorted.
pub fn validate_structural_diagram(
    project: &Project,
    diagram_id: &str,
    kind: StructuralDiagramKind,
) -> Vec<StructuralDiagnostic> {
    let mut issues = collect_structural_diagram_diagnostics(project, diagram_id, kind);
    sort_diagnostics(&mut issues);
    issues
}

fn collect_structural_diagram_diagnostics(
    project: &Project,
    diagram_id: &str,
    kind: StructuralDiagramKind,
) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    if let Some(diagram) = project
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
    {
        for view in &diagram.views {
            if let Some(draft) = &view.draft {
                issues.push(StructuralDiagnostic {
                    severity: StructuralSeverity::Warning,
                    code: "VISUAL_DRAFT_UNCHECKED",
                    subject_id: view.id.clone(),
                    message: format!("Visual draft {} has not been validated as a semantic UML element or relationship", draft.notation_kind),
                });
                for identifier in draft
                    .parent_view_id
                    .iter()
                    .chain(view.endpoints.iter().flat_map(|endpoints| {
                        endpoints
                            .source_view_id
                            .iter()
                            .chain(endpoints.target_view_id.iter())
                    }))
                {
                    if !diagram
                        .views
                        .iter()
                        .any(|candidate| &candidate.id == identifier)
                    {
                        issues.push(StructuralDiagnostic {
                            severity: StructuralSeverity::Warning,
                            code: "VISUAL_DRAFT_ORPHANED_LINK",
                            subject_id: view.id.clone(),
                            message: format!("Visual draft references removed view {identifier}"),
                        });
                    }
                }
            }
        }
    }
    if let Err(error) = Graph::new(project).validate() {
        issues.push(problem("UML_ROOT_INVALID", &project.id, error.to_string()));
        return issues;
    }
    issues.extend(collect_structural_model_diagnostics(project));
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
    if StructuralDiagramKind::from_diagram(diagram).is_none() {
        issues.push(problem(
            "UMLDI_UNSUPPORTED_DIAGRAM_TYPE",
            &diagram.id,
            format!("unsupported stored UMLDI type {stored_type}"),
        ));
        return issues;
    }
    if StructuralDiagramKind::from_diagram(diagram) != Some(kind) {
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
    if let Some(expected) = kind.context_metaclass() {
        if diagram.model_elements.len() != 1 {
            issues.push(problem(
                "UMLDI_CONTEXT_MULTIPLICITY",
                &diagram.id,
                format!("This diagram requires exactly one {expected} context"),
            ));
        }
        for context_id in &diagram.model_elements {
            match elements.get(context_id.as_str()) {
                None => issues.push(problem(
                    "UMLDI_CONTEXT_MISSING",
                    context_id,
                    "The diagram context does not identify a model Element",
                )),
                Some(context) => {
                    if let Some(issue) = context_type_diagnostic(context, expected) {
                        issues.push(issue);
                    }
                }
            }
        }
    } else if kind != StructuralDiagramKind::CompositeStructure {
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
                    use crate::uml::{
                        classifiers::QueryError,
                        root::{ElementRef, Model},
                    };
                    let conformance = ElementRef::Object(context)
                        .instance_specification()
                        .ok_or_else(|| {
                            QueryError::unsupported(
                                context_id,
                                "InstanceSpecification context adapter is unavailable",
                            )
                        })
                        .and_then(|instance| {
                            instance.has_structured_classifier(Model::from_project(project))
                        });
                    match conformance {
                        Ok(true) => {}
                        Ok(false) => issues.push(problem("UMLDI_CONTEXT_TYPE", context_id,
                            "Composite structure InstanceSpecification context requires at least one StructuredClassifier classifier")),
                        Err(QueryError::Invalid { message, .. }) =>
                            issues.push(problem("UMLDI_CONTEXT_TYPE", context_id, message)),
                        Err(QueryError::Unsupported { message, .. }) => issues.push(StructuralDiagnostic {
                            severity: StructuralSeverity::Warning,
                            code: "UMLDI_INSTANCE_CONTEXT_UNSUPPORTED",
                            subject_id: context_id.clone(),
                            message,
                        }),
                    }
                }
                Some(context) => {
                    if let Some(issue) = context_type_diagnostic(context, "StructuredClassifier") {
                        issues.push(issue);
                    }
                }
            }
        }
    }
    let relations = project
        .relations
        .iter()
        .map(|relation| (relation.id.as_str(), relation))
        .collect::<HashMap<_, _>>();
    let views = diagram
        .views
        .iter()
        .map(|view| (view.id.as_str(), view))
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
            // Attachment identities belong to the editor projection, not UML semantics.
            // Legacy views without explicit attachments still resolve by subject.
            if view.draft.is_none()
                && let Some(endpoints) = &view.endpoints
            {
                for (side, identifier, expected) in [
                    (
                        "source",
                        endpoints.source_view_id.as_deref(),
                        relation.from.as_str(),
                    ),
                    (
                        "target",
                        endpoints.target_view_id.as_deref(),
                        relation.to.as_str(),
                    ),
                ] {
                    match identifier {
                        None => issues.push(problem("UMLDI_ENDPOINT_UNATTACHED", &view.id,
                            format!("The current editor requires an explicit {side} View attachment when relationship endpoints are stored"))),
                        Some(identifier) => {
                            if let Some(endpoint) = views.get(identifier)
                                && endpoint.subject.as_deref() != Some(expected)
                            {
                                issues.push(problem("UMLDI_ENDPOINT_SUBJECT_MISMATCH", &view.id,
                                    format!("The current editor requires {side} View {identifier} to represent relationship endpoint {expected}")));
                            }
                        }
                    }
                }
            }
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
            && let Geometry::Path { segments, .. } = &view.geometry
        {
            // UML Message is a NamedElement, not a Relationship, but interaction
            // diagrams present it as an Edge rather than a classifier Shape.
            if element.kind == "uml.message" && kind.interaction_kind().is_some() {
                if segments.is_empty() {
                    issues.push(problem("UMLDI_EXPECTED_EDGE", &view.id,
                        format!("message {subject} needs a nonempty Edge Path in the current editor projection")));
                }
            } else {
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
    }
    issues
}

/// Checks the represented UML relationship semantics across the whole model,
/// independently of Views. This is not full UML conformance or graph integrity
/// validation; callers also use Graph::validate for IDs and references.
/// Diagnostics use the same canonical order as diagram validation. No input
/// member ordering or diagnostic multiplicity is changed.
pub fn validate_structural_model(project: &Project) -> Vec<StructuralDiagnostic> {
    let mut issues = collect_structural_model_diagnostics(project);
    sort_diagnostics(&mut issues);
    issues
}

fn sort_diagnostics(issues: &mut [StructuralDiagnostic]) {
    let severity_order = |severity| match severity {
        StructuralSeverity::Error => 0,
        StructuralSeverity::Warning => 1,
    };
    issues.sort_by(|left, right| {
        (
            left.code,
            &left.subject_id,
            severity_order(left.severity),
            &left.message,
        )
            .cmp(&(
                right.code,
                &right.subject_id,
                severity_order(right.severity),
                &right.message,
            ))
    });
}

fn collect_structural_model_diagnostics(project: &Project) -> Vec<StructuralDiagnostic> {
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
            && (UmlType::from_kind(&source.kind).is_some_and(|kind| !kind.is_classifier())
                || UmlType::from_kind(&target.kind).is_some_and(|kind| !kind.is_classifier()))
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
            // UML.xmi Classifier-specialize_type / Classifier-maySpecializeType-spec.
            if let (Some(specific), Some(general)) = (source_type, target_type)
                && specific.may_specialize_type(general) == Some(false)
            {
                issues.push(problem(
                    if general == UmlType::Stereotype { "UML_STEREOTYPE_GENERALIZATION" } else { "UML_GENERALIZATION_TYPE" }, &relation.id,
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
        // Property-reference and editor links remain supported without becoming UML Relationships.
        if relationship_root_kind(relation).is_none()
            && relation.kind.starts_with("uml.")
            && !matches!(
                relation.kind.as_str(),
                "uml.parametered_element"
                    | "uml.constraining_classifier"
                    | "uml.note_link"
                    | "uml.constraint_link"
                    | "uml.containment"
            )
        {
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
    let model = crate::uml::root::Model::from_project(project);
    if project.root_model.is_some() {
        issues.extend(model.validate());
    } else {
        issues.extend(model.validate_contracts());
    }
    for issue in crate::structural_rules::validate(project) {
        // Rooted namespace validation and the rootless-draft check can report
        // the same package visibility violation. Keep both checks, including
        // their different coverage and representation diagnostics, but report
        // that shared violation only once.
        if issue.code == "UML_PACKAGE_MEMBER_VISIBILITY"
            && issues.iter().any(|existing| {
                existing.code == issue.code
                    && existing.subject_id == issue.subject_id
                    && existing.severity == issue.severity
            })
        {
            continue;
        }
        issues.push(issue);
    }
    issues.extend(crate::templates::validate(project));
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PathSegment, Point, Style, View};

    fn assert_legacy_association_warnings(
        issues: Vec<StructuralDiagnostic>,
        subject: &str,
        aggregation: bool,
        association_class: bool,
    ) {
        let distinguishability = format!(
            "Unsupported UML namespace capability on {subject}: Association memberEnd identities are not materialized; migrate the legacy binary notation first"
        );
        let endpoints = format!(
            "Unsupported UML graph representation at {subject}: Association memberEnd identities are not materialized; related Elements are unverified"
        );
        let mut expected = vec![
            (
                "UML_NAMESPACE_DISTINGUISHABILITY_UNSUPPORTED",
                distinguishability.as_str(),
            ),
            (
                "MDE_ASSOCIATION_ENDS_UNMATERIALIZED",
                "Association memberEnd identities are not materialized; legacy endpoint notation is not a complete Association kernel model",
            ),
            (
                "UML_NAMESPACE_QUERY_UNSUPPORTED",
                "Association memberEnd identities are not materialized; migrate the legacy binary notation first",
            ),
            (
                "MDE_PROPERTY_CONTRACT_UNVERIFIED",
                "Association-memberEnd: required reference/collection needs its canonical role projection",
            ),
        ];
        if association_class {
            expected.extend([
                (
                    "MDE_ASSOCIATION_CLASS_UNVERIFIED",
                    "AssociationClass-cannot_be_defined: memberEnd identities are not materialized",
                ),
                ("MDE_RELATIONSHIP_ENDPOINTS_UNVERIFIED", endpoints.as_str()),
            ]);
        }
        if aggregation {
            expected.push((
                "MDE_PROPERTY_CONTRACT_UNVERIFIED",
                "end1Aggregation: no normative property or registered extension contract",
            ));
        }
        assert!(
            issues
                .iter()
                .all(|issue| issue.severity == StructuralSeverity::Warning
                    && issue.subject_id == subject),
            "{issues:#?}"
        );
        let mut actual = issues
            .iter()
            .map(|issue| (issue.code, issue.message.as_str()))
            .collect::<Vec<_>>();
        actual.sort_unstable();
        expected.sort_unstable();
        assert_eq!(actual, expected);
    }

    fn fixture() -> Project {
        serde_json::from_str(include_str!("../../../../fixtures/todo.json")).unwrap()
    }
    #[test]
    fn structural_diagrams_are_the_seven_uml_di_classes() {
        const XML: &str =
            include_str!("../../../../../../docs/bellatrix/standards/uml/20161101/UMLDI.xmi");
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
        const XML: &str =
            include_str!("../../../../../../docs/bellatrix/standards/uml/20161101/UML.xmi");
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
        let issues =
            validate_structural_diagram(&fixture(), "diagram-tasks", StructuralDiagramKind::Class);
        assert!(
            issues
                .iter()
                .all(|issue| issue.severity == StructuralSeverity::Warning
                    && issue.code == "MDE_PROPERTY_CONTRACT_UNVERIFIED"),
            "{issues:#?}"
        );
        let mut actual = issues
            .iter()
            .map(|issue| (issue.subject_id.as_str(), issue.message.as_str()))
            .collect::<Vec<_>>();
        actual.sort_unstable();
        let declaration = "declaration: no normative property or registered extension contract";
        let signature = "signature: no normative property or registered extension contract";
        assert_eq!(
            actual,
            vec![
                ("field-completed", declaration),
                ("field-tasks", declaration),
                ("field-title", declaration),
                ("fn-add", signature),
                ("fn-complete", signature)
            ]
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
            endpoints: None,
            timing: None,
            draft: None,
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
                    endpoints: None,
                    timing: None,
                    draft: None,
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
            let issues = validate_structural_diagram(&project, "context-diagram", kind);
            if context_kind == "uml.association_class" {
                assert_legacy_association_warnings(issues, "context", false, true);
            } else {
                assert!(issues.is_empty(), "{context_kind}: {issues:#?}");
            }
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
    fn obsolete_aggregation_kinds_are_rejected_but_legacy_association_notation_is_retained()
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
            assert_legacy_association_warnings(
                validate_structural_model(&project),
                "specialization",
                true,
                false,
            );
        }
        Ok(())
    }
}
