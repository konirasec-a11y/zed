use crate::uml::types::{MultiplicityError, RelationEnd, RelationEndMultiplicity};
use crate::{Element, GraphIndex, Project, StructuralDiagnostic, StructuralSeverity, UmlType};
use std::collections::HashMap;

fn metaclass(element: &Element) -> Option<&'static str> {
    match element.kind.as_str() {
        "uml.instance_specification" => Some("InstanceSpecification"),
        "uml.slot" => Some("Slot"),
        _ => UmlType::from_kind(&element.kind).and_then(UmlType::metaclass),
    }
}

fn conforms(element: &Element, expected: &str) -> bool {
    metaclass(element).is_some_and(|name| crate::uml::metamodel::conforms_to(name, expected))
}

fn diagnostic(
    severity: StructuralSeverity,
    code: &'static str,
    subject: &str,
    message: impl Into<String>,
) -> StructuralDiagnostic {
    StructuralDiagnostic {
        severity,
        code,
        subject_id: subject.into(),
        message: message.into(),
    }
}

pub(super) fn validate(project: &Project) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    let elements: HashMap<_, _> = project
        .elements
        .iter()
        .map(|element| (element.id.as_str(), element))
        .collect();
    for element in &project.elements {
        if element.kind.starts_with("uml.") && metaclass(element).is_none() {
            issues.push(diagnostic(StructuralSeverity::Warning,
                "MDE_UML_METACLASS_UNSUPPORTED", &element.id,
                format!("{} has no normative metaclass mapping in the current adapter; UML semantics are not verified", element.kind)));
        }
    }
    for relation in &project.relations {
        // UML.xmi InterfaceRealization-implementingClassifier; UML 2.5.1 §10.5.6.4.
        if relation.kind == "uml.interface_realization"
            && let Some(source) = elements.get(relation.from.as_str())
            && !conforms(source, "BehavioredClassifier")
        {
            if metaclass(source).is_some() {
                issues.push(diagnostic(
                    StructuralSeverity::Error,
                    "UML_INTERFACE_REALIZATION_SOURCE",
                    &relation.id,
                    "InterfaceRealization implementingClassifier must be a BehavioredClassifier",
                ));
            } else {
                issues.push(diagnostic(StructuralSeverity::Warning,
                    "MDE_RELATIONSHIP_ENDPOINT_UNSUPPORTED", &relation.id,
                    "InterfaceRealization source has no normative metaclass mapping; implementingClassifier type is not verified"));
            }
        }
        if matches!(
            relation.kind.as_str(),
            "uml.association" | "uml.directed_association"
        ) {
            // UML.xmi Association-ends_must_be_typed and TypedElement-type.
            // Relation endpoints represent the Types of the two member ends.
            for (end, identifier) in [("source", &relation.from), ("target", &relation.to)] {
                if let Some(element) = elements.get(identifier.as_str())
                    && !conforms(element, "Type")
                {
                    if metaclass(element).is_some() {
                        issues.push(diagnostic(
                            StructuralSeverity::Error,
                            "UML_ASSOCIATION_END_TYPE",
                            &relation.id,
                            format!("Association {end} end must be typed by a UML Type"),
                        ));
                    } else {
                        issues.push(diagnostic(StructuralSeverity::Warning,
                            "MDE_RELATIONSHIP_ENDPOINT_UNSUPPORTED", &relation.id,
                            format!("Association {end} endpoint has no normative metaclass mapping; end Type is not verified")));
                    }
                }
            }
            for end in [RelationEnd::Source, RelationEnd::Target] {
                if let Some(issue) = multiplicity(
                    &relation.id,
                    end.multiplicity_key(),
                    RelationEndMultiplicity::new(relation, end),
                ) {
                    issues.push(issue);
                }
            }
        }
    }
    let graph = GraphIndex::new(project);
    // UML.xmi Package-elements_public_or_private; UML 2.5.1 §12.4.5.8.
    // Only direct semantic ownership of PackageableElements maps to packagedElement.
    for package in project
        .elements
        .iter()
        .filter(|element| conforms(element, "Package"))
    {
        match graph.outgoing(&package.id, Some("owns")) {
            Ok(relations) => {
                for relation in relations {
                    if let Some(element) = elements.get(relation.to.as_str())
                        && conforms(element, "PackageableElement")
                        && let Some(visibility) = element.properties.get("visibility")
                        && !visibility.is_null()
                        && !matches!(visibility.as_str(), Some("public" | "private"))
                    {
                        let (code, message) = if visibility.is_string() {
                            (
                                "UML_PACKAGE_MEMBER_VISIBILITY",
                                "Package-owned packaged elements with visibility must be public or private",
                            )
                        } else {
                            (
                                "MDE_VISIBILITY_REPRESENTATION",
                                "visibility must be a string or null in the MDE representation",
                            )
                        };
                        issues.push(diagnostic(
                            StructuralSeverity::Error,
                            code,
                            &element.id,
                            message,
                        ));
                    }
                }
            }
            // GraphIndex is built from these same element IDs; preserve an unexpected
            // lookup failure rather than silently losing validation coverage.
            Err(error) => issues.push(diagnostic(
                StructuralSeverity::Error,
                "MDE_OWNERSHIP_LOOKUP",
                &package.id,
                error.to_string(),
            )),
        }
    }
    issues
}

fn multiplicity(
    subject: &str,
    key: &str,
    multiplicity: RelationEndMultiplicity<'_>,
) -> Option<StructuralDiagnostic> {
    let error = multiplicity.effective().err()?;
    let (severity, code) = match error {
        MultiplicityError::Representation => {
            (StructuralSeverity::Error, "MDE_MULTIPLICITY_REPRESENTATION")
        }
        MultiplicityError::UnsupportedValueSpecification => {
            (StructuralSeverity::Warning, "MDE_MULTIPLICITY_UNSUPPORTED")
        }
        MultiplicityError::InvalidBounds => (StructuralSeverity::Error, "UML_MULTIPLICITY_BOUNDS"),
    };
    Some(diagnostic(
        severity,
        code,
        subject,
        format!("{key}: {error}"),
    ))
}
