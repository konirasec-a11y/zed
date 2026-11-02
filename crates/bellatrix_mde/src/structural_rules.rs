use crate::{Element, GraphIndex, Project, StructuralDiagnostic, StructuralSeverity, UmlType};
use serde_json::Value;
use std::collections::HashMap;

fn metaclass(element: &Element) -> Option<&'static str> {
    UmlType::from_kind(&element.kind).and_then(UmlType::metaclass)
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
            for key in ["sourceMultiplicity", "targetMultiplicity"] {
                if let Some(value) = relation.properties.get(key)
                    && let Some(issue) = multiplicity(&relation.id, key, value)
                {
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

// Decimal comparison avoids imposing a machine-integer maximum on UML literals.
fn integer(text: &str) -> Option<(bool, &str)> {
    let text = text.trim();
    let negative = text.starts_with('-');
    let digits = text.strip_prefix(['-', '+']).unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let digits = digits.trim_start_matches('0');
    Some((negative && !digits.is_empty(), digits))
}

fn multiplicity(subject: &str, key: &str, value: &Value) -> Option<StructuralDiagnostic> {
    let Some(text) = value.as_str() else {
        return Some(diagnostic(
            StructuralSeverity::Error,
            "MDE_MULTIPLICITY_REPRESENTATION",
            subject,
            format!("{key} must be a string in the MDE representation"),
        ));
    };
    let text = text.trim();
    let (lower, upper) = if text == "*" {
        ("0", "*")
    } else {
        text.split_once("..").unwrap_or((text, text))
    };
    let parsed_lower = integer(lower);
    let parsed_upper = if upper.trim() == "*" {
        None
    } else {
        integer(upper)
    };
    let unsupported = || {
        diagnostic(
            StructuralSeverity::Warning,
            "MDE_MULTIPLICITY_UNSUPPORTED",
            subject,
            format!(
                "{key} cannot be evaluated by the literal-bound validator; constant UML ValueSpecifications are not evaluated"
            ),
        )
    };
    let Some((negative_lower, lower_digits)) = parsed_lower else {
        return Some(unsupported());
    };
    if upper.trim() != "*" && parsed_upper.is_none() {
        return Some(unsupported());
    }
    // UML.xmi MultiplicityElement-lower_ge_0, -upper_is_unlimitedNatural,
    // and -upper_ge_lower; UML 2.5.1 §7.8.8.8. Nonliteral values remain unsupported,
    // not invalid UML (value_specification_constant permits constant expressions).
    let invalid = negative_lower
        || parsed_upper.is_some_and(|(negative_upper, upper_digits)| {
            negative_upper
                || (upper_digits.len(), upper_digits) < (lower_digits.len(), lower_digits)
        });
    invalid.then(|| diagnostic(StructuralSeverity::Error, "UML_MULTIPLICITY_BOUNDS", subject,
        format!("{key} must have a nonnegative integer lower bound and an unlimited-natural upper bound at least as large")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate_structural_model;
    use serde_json::json;

    fn fixture(
        source: &str,
        target: &str,
        kind: &str,
        properties: Value,
    ) -> serde_json::Result<Project> {
        serde_json::from_value(json!({"id":"model", "name":"Rules",
            "elements":[{"id":"a","name":"A","kind":source}, {"id":"b","name":"B","kind":target}],
            "relations":[{"id":"r","from":"a","to":"b","kind":kind,"properties":properties}],
            "diagrams":[{"id":"d","name":"Classes","uml_di_type":"UMLClassDiagram","views":[]}]}))
    }

    #[test]
    fn unsupported_uml_adapters_are_warnings_not_normative_errors() -> serde_json::Result<()> {
        for kind in [
            "uml.template",
            "uml.subsystem",
            "uml.frame",
            "uml.instance_specification",
            "uml.connector",
            "uml.unknown",
        ] {
            let project = fixture(kind, "uml.class", "uml.association", json!({}))?;
            let issues = validate(&project);
            assert!(
                issues
                    .iter()
                    .any(|issue| issue.code == "MDE_UML_METACLASS_UNSUPPORTED"),
                "{kind}"
            );
            assert!(
                issues
                    .iter()
                    .all(|issue| issue.severity == StructuralSeverity::Warning),
                "{kind}"
            );
        }
        let mut project = fixture("fs.file", "rust.unmapped", "depends_on", json!({}))?;
        assert!(validate(&project).is_empty());
        project.relations[0].kind = "uml.association".into();
        assert!(
            validate(&project)
                .iter()
                .all(|issue| issue.severity == StructuralSeverity::Warning)
        );
        Ok(())
    }

    #[test]
    fn interface_realization_requires_behaviored_classifier_not_just_classifier()
    -> serde_json::Result<()> {
        for source in [
            "uml.interface",
            "uml.signal",
            "uml.data_type",
            "uml.enumeration",
            "uml.artifact",
            "uml.package",
        ] {
            let project = fixture(
                source,
                "uml.interface",
                "uml.interface_realization",
                json!({}),
            )?;
            assert!(
                validate_structural_model(&project)
                    .iter()
                    .any(|issue| issue.code == "UML_INTERFACE_REALIZATION_SOURCE"),
                "{source}"
            );
        }
        for source in [
            "uml.class",
            "uml.component",
            "uml.node",
            "uml.device",
            "uml.execution_environment",
            "uml.stereotype",
            "uml.collaboration",
            "uml.association_class",
        ] {
            let project = fixture(
                source,
                "uml.interface",
                "uml.interface_realization",
                json!({}),
            )?;
            assert!(validate_structural_model(&project).is_empty(), "{source}");
        }
        Ok(())
    }

    #[test]
    fn association_end_types_are_not_limited_to_classes() -> serde_json::Result<()> {
        for kind in ["uml.association", "uml.directed_association"] {
            for invalid in ["uml.package", "uml.attribute", "uml.comment"] {
                let project = fixture(invalid, "uml.class", kind, json!({}))?;
                assert!(
                    validate(&project)
                        .iter()
                        .any(|issue| issue.code == "UML_ASSOCIATION_END_TYPE")
                );
            }
            let project = fixture("uml.data_type", "uml.interface", kind, json!({}))?;
            assert!(validate(&project).is_empty());
        }
        Ok(())
    }

    #[test]
    fn package_visibility_applies_to_direct_packaged_elements_only() -> serde_json::Result<()> {
        for owner in ["uml.package", "uml.model", "uml.profile"] {
            for visibility in ["protected", "package", "public", "private"] {
                let mut project = fixture(owner, "uml.class", "owns", json!({}))?;
                project.elements[1]
                    .properties
                    .insert("visibility".into(), json!(visibility));
                assert_eq!(
                    validate(&project).is_empty(),
                    matches!(visibility, "public" | "private")
                );
            }
        }
        let mut project = fixture("uml.class", "uml.attribute", "owns", json!({}))?;
        project.elements[1]
            .properties
            .insert("visibility".into(), json!("protected"));
        assert!(validate(&project).is_empty());
        Ok(())
    }

    #[test]
    fn package_visibility_absent_or_null_is_not_rejected() -> serde_json::Result<()> {
        let mut project = fixture("uml.package", "uml.class", "owns", json!({}))?;
        assert!(validate(&project).is_empty());
        project.elements[1]
            .properties
            .insert("visibility".into(), Value::Null);
        assert!(validate(&project).is_empty());
        project.elements[1]
            .properties
            .insert("visibility".into(), json!(42));
        assert_eq!(validate(&project)[0].code, "MDE_VISIBILITY_REPRESENTATION");
        Ok(())
    }

    #[test]
    fn literal_multiplicities_validate_both_ends_without_integer_size_limit()
    -> serde_json::Result<()> {
        for key in ["sourceMultiplicity", "targetMultiplicity"] {
            for text in ["3..2", "-1..2", "0..-1", "-1"] {
                let project = fixture(
                    "uml.class",
                    "uml.class",
                    "uml.association",
                    json!({key:text}),
                )?;
                let issues = validate(&project);
                assert_eq!(issues.len(), 1, "{text}");
                assert_eq!(issues[0].severity, StructuralSeverity::Error);
                assert_eq!(issues[0].code, "UML_MULTIPLICITY_BOUNDS");
            }
            for text in [
                "2..3",
                "0..2",
                "*",
                "0",
                "1",
                "0..*",
                " 01 .. 002 ",
                "999999999999999999999999999999999999999999999999..*",
            ] {
                let project = fixture(
                    "uml.class",
                    "uml.class",
                    "uml.association",
                    json!({key:text}),
                )?;
                assert!(validate(&project).is_empty(), "{text}");
            }
        }
        Ok(())
    }

    #[test]
    fn nonliteral_multiplicity_is_unsupported_not_invalid_uml() {
        for text in ["", "limit", "0..limit", "1..2..3", "*..3", "1.5"] {
            let issue = multiplicity("r", "sourceMultiplicity", &json!(text))
                .expect("unsupported literal syntax");
            assert_eq!(issue.severity, StructuralSeverity::Warning);
            assert_eq!(issue.code, "MDE_MULTIPLICITY_UNSUPPORTED");
        }
        for value in [json!(2), json!(null), json!({"lower":0}), json!(true)] {
            assert_eq!(
                multiplicity("r", "sourceMultiplicity", &value)
                    .expect("wrong representation")
                    .code,
                "MDE_MULTIPLICITY_REPRESENTATION"
            );
        }
    }

    #[test]
    fn absence_and_unrelated_relationship_properties_are_skipped() -> serde_json::Result<()> {
        let project = fixture(
            "uml.class",
            "uml.class",
            "uml.dependency",
            json!({"sourceMultiplicity":"3..2"}),
        )?;
        assert!(validate(&project).is_empty());
        let project = fixture("uml.class", "uml.class", "uml.association", json!({}))?;
        assert!(validate(&project).is_empty());
        Ok(())
    }

    #[test]
    fn reflexive_association_is_valid_and_missing_endpoints_belong_to_graph_validation()
    -> serde_json::Result<()> {
        let mut project = fixture("uml.class", "uml.class", "uml.association", json!({}))?;
        project.relations[0].to = "a".into();
        assert!(validate_structural_model(&project).is_empty());
        project.relations[0].to = "missing".into();
        assert!(validate(&project).is_empty());
        Ok(())
    }
}
