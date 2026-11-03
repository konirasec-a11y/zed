use super::{
    classifiers::{self, QueryError},
    feature_compatibility,
    property_contracts::{self, PropertyOutcome},
    references::ReferenceIndex,
    root::ElementRef,
    schema,
    types::{Integer, UnlimitedNatural},
    value_specifications::{self, EvaluationError},
};
use crate::{StructuralDiagnostic, StructuralSeverity};

fn query_value<T>(result: Result<T, EvaluationError>) -> Result<T, QueryError> {
    result.map_err(|error| match error {
        EvaluationError::Invalid { subject_id, message } => QueryError::invalid(&subject_id, message),
        EvaluationError::Unsupported { subject_id, message } => QueryError::unsupported(&subject_id, message),
    })
}

fn end_multiplicity(references: &ReferenceIndex<'_>, end: ElementRef<'_>) -> Result<bool, QueryError> {
    let interval = query_value(value_specifications::multiplicity(references, end));
    let valid = match interval {
        Err(error @ QueryError::Invalid { .. }) => Err(error),
        _ => Ok(true),
    };
    // The known bound can disprove the conjunction even when its opposite is
    // opaque. The interval guard still retains malformed bound values.
    feature_compatibility::require_all([
        valid,
        query_value(value_specifications::lower_bound(references, end))
            .map(|lower| lower == Integer::from(0) || lower == Integer::from(1)),
        query_value(value_specifications::upper_bound(references, end))
            .map(|upper| upper == UnlimitedNatural::from(1)),
    ])
}

fn end_aggregation(end: ElementRef<'_>) -> Result<bool, QueryError> {
    schema::read_effective_scalar(end, "aggregation")
        .map(|value| value.as_str() == Some("composite"))
        .map_err(|error| QueryError::invalid(end.id(), error))
}

fn property_check(references: &ReferenceIndex<'_>, element: ElementRef<'_>, name: &str) -> Result<bool, QueryError> {
    let descriptor = schema::uml_property(element.metaclass().ok_or_else(|| QueryError::unsupported(element.id(), "unresolved metaclass"))?, name)
        .map_err(|error| QueryError::unsupported(element.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(element.id(), format!("missing {name} contract")))?;
    let value = element.properties().get(name)
        .ok_or_else(|| QueryError::unsupported(element.id(), format!("{name} is not materialized")))?;
    match property_contracts::check_value(references, element, descriptor, value) {
        PropertyOutcome::Satisfied => Ok(true),
        PropertyOutcome::Violation(message) => Err(QueryError::invalid(element.id(), message)),
        PropertyOutcome::Unverified(message) => Err(QueryError::unsupported(element.id(), message)),
    }
}

fn binary(references: &ReferenceIndex<'_>, extension: ElementRef<'_>) -> Result<bool, QueryError> {
    let contract = property_check(references, extension, "memberEnd");
    if let Err(error @ QueryError::Invalid { .. }) = contract { return Err(error); }
    match extension.properties().get("memberEnd").and_then(serde_json::Value::as_array) {
        Some(ends) => Ok(ends.len() == 2),
        None => contract,
    }
}

fn owned_end_projection<'a>(references: &ReferenceIndex<'a>, extension: ElementRef<'a>) -> (Option<ElementRef<'a>>, Result<bool, QueryError>) {
    let contract = property_check(references, extension, "ownedEnd");
    let mut checks = vec![contract.clone()];
    let children = references.explicit_owned_elements(extension.id());
    match children {
        Ok(children) => {
            let known = children.iter().filter(|child| child.conforms_to("ExtensionEnd")).count();
            if known > 1 {
                checks.push(Err(QueryError::invalid(extension.id(), "Extension::ownedEnd permits exactly one canonical ExtensionEnd")));
            } else if children.iter().any(|child| child.metaclass().is_none()) {
                checks.push(Err(QueryError::unsupported(extension.id(), "unknown owned metaclass may contribute to Extension::ownedEnd")));
            }
        }
        Err(error) => checks.push(Err(QueryError::invalid(extension.id(), error))),
    }
    let element = if matches!(contract, Ok(true)) {
        let resolved = (|| {
            let value = extension.properties().get("ownedEnd")
                .ok_or_else(|| QueryError::unsupported(extension.id(), "ownedEnd is not materialized"))?;
            let id = super::references::reference_id(value)
                .map_err(|error| QueryError::invalid(extension.id(), error))?
                .ok_or_else(|| QueryError::invalid(extension.id(), "ownedEnd is required"))?;
            references.element(id).map_err(|error| QueryError::invalid(extension.id(), error))
        })();
        match resolved {
            Ok(element) => Some(element),
            Err(error) => { checks.push(Err(error)); None }
        }
    } else { None };
    (element, feature_compatibility::require_all(checks))
}

/// UML Extension::isRequired, independent of stored isRequired/metaclass caches.
pub fn is_required(references: &ReferenceIndex<'_>, extension: ElementRef<'_>) -> Result<bool, QueryError> {
    if extension.metaclass().is_none() {
        return Err(QueryError::unsupported(extension.id(), "Extension metaclass is unresolved"));
    }
    if !extension.conforms_to("Extension") {
        return Err(QueryError::invalid(extension.id(), "isRequired requires an Extension"));
    }
    let (owned, contract) = owned_end_projection(references, extension);
    let lower = match owned {
        Some(owned) => query_value(value_specifications::lower_bound(references, owned))
            .map(|lower| lower == Integer::from(1)),
        None => Err(QueryError::unsupported(extension.id(), "ownedEnd identity is unresolved")),
    };
    // A false derived Boolean is not permission to ignore an unresolved required
    // reference contract, unlike a false operand of a normative conjunction.
    feature_compatibility::require_all([
        contract, lower.as_ref().map(|_| true).map_err(Clone::clone),
    ])?;
    lower
}

fn attribute_associations_binary(references: &ReferenceIndex<'_>, attribute: ElementRef<'_>) -> Result<bool, QueryError> {
    let associations = references.association_membership_candidates(attribute.id())
        .map_err(|error| QueryError::invalid(attribute.id(), error))?;
    let mut checks = Vec::new();
    if associations.len() > 1 {
        checks.push(Err(QueryError::invalid(attribute.id(), "Property::association permits at most one Association")));
    }
    for association in &associations {
        checks.push(binary(references, *association));
    }
    // The reverse index contains only explicit local identities. Missing or
    // external member lists cannot establish that no further Association exists.
    for association in references.model().elements().filter(|element| element.conforms_to("Association")) {
        if associations.iter().any(|known| known.id() == association.id()) { continue; }
        let members = association.properties().get("memberEnd");
        if members.is_some_and(|value| super::references::reference_id(value).ok().flatten() == Some(attribute.id())) {
            checks.push(property_check(references, association, "memberEnd"));
        } else if members.and_then(serde_json::Value::as_array).is_none_or(|members|
            members.iter().any(|value| super::references::reference_id(value).is_err()))
        {
            checks.push(Err(QueryError::unsupported(attribute.id(), "unresolved Association member lists prevent complete association membership")));
        }
    }
    if let Some(stored) = attribute.properties().get("association") {
        checks.push(property_check(references, attribute, "association"));
        if let Ok(id) = super::references::reference_id(stored) {
            if let Some(known) = associations.first() {
                if id != Some(known.id()) {
                    checks.push(Err(QueryError::invalid(attribute.id(), "association inverse contradicts canonical memberEnd membership")));
                }
            } else if let Some(id) = id {
                match references.element(id) {
                    Ok(association) if association.conforms_to("Association") => {
                        let members = association.properties().get("memberEnd");
                        checks.push(match members {
                            Some(_) => property_check(references, association, "memberEnd"),
                            None => Err(QueryError::unsupported(attribute.id(), "Association::memberEnd is not materialized")),
                        });
                        if members.and_then(serde_json::Value::as_array).is_some_and(|members|
                            members.iter().all(|value| super::references::reference_id(value).is_ok()))
                        {
                            checks.push(Err(QueryError::invalid(attribute.id(), "association inverse has no corresponding memberEnd")));
                        } else {
                            checks.push(Err(QueryError::unsupported(attribute.id(), "association inverse cannot establish canonical membership")));
                        }
                    }
                    Ok(_) => {}
                    Err(error) => checks.push(Err(QueryError::invalid(attribute.id(), error))),
                }
            }
        }
    }
    feature_compatibility::require_all(checks)
}

fn stereotype_binary_associations(references: &ReferenceIndex<'_>, stereotype: ElementRef<'_>) -> Result<bool, QueryError> {
    let mut checks = Vec::new();
    // Inspect only direct attribute roles; inherited namespace members are not
    // Stereotype::ownedAttribute and must not create this obligation.
    for descriptor in classifiers::properties(stereotype)? {
        if descriptor.aggregation != schema::Aggregation::Composite || descriptor.is_derived {
            continue;
        }
        if classifiers::contributes(descriptor, "Classifier-attribute")
            .map_err(|error| QueryError::unsupported(stereotype.id(), error))?
            && stereotype.properties().contains_key(descriptor.name)
        {
            checks.push(property_check(references, stereotype, descriptor.name));
        }
    }
    for child in references.explicit_owned_elements(stereotype.id())
        .map_err(|error| QueryError::invalid(stereotype.id(), error))?
    {
        if child.metaclass().is_none() {
            checks.push(Err(QueryError::unsupported(child.id(), "unknown owned metaclass may be an attribute")));
        } else if child.conforms_to("Property") {
            match classifiers::is_owned_attribute(stereotype, child) {
                Ok(true) => checks.push(attribute_associations_binary(references, child)),
                Ok(false) => {}
                Err(error) => checks.push(Err(error)),
            }
        }
    }
    feature_compatibility::require_all(checks)
}

pub(crate) fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    for end in references.model().elements().filter(|element| element.conforms_to("ExtensionEnd")) {
        for (rule, error_code, unknown_code, result) in [
            ("ExtensionEnd-multiplicity", "UML_EXTENSION_END_MULTIPLICITY", "MDE_EXTENSION_END_MULTIPLICITY_UNVERIFIED", end_multiplicity(references, end)),
            ("ExtensionEnd-aggregation", "UML_EXTENSION_END_AGGREGATION", "MDE_EXTENSION_END_AGGREGATION_UNVERIFIED", end_aggregation(end)),
        ] {
            let (severity, code, message) = match result {
                Ok(true) => continue,
                Ok(false) => (StructuralSeverity::Error, error_code, "normative constraint is violated".to_owned()),
                Err(QueryError::Invalid { message, .. }) => (StructuralSeverity::Error, error_code, message),
                Err(QueryError::Unsupported { message, .. }) => (StructuralSeverity::Warning, unknown_code, message),
            };
            issues.push(StructuralDiagnostic { severity, code, subject_id:end.id().into(), message:format!("{rule}: {message}") });
        }
    }
    for extension in references.model().elements().filter(|element| element.conforms_to("Extension")) {
        for (rule, error_code, unknown_code, result) in [
            ("Extension-is_binary", "UML_EXTENSION_IS_BINARY", "MDE_EXTENSION_IS_BINARY_UNVERIFIED", binary(references, extension)),
            ("Extension-ownedEnd", "UML_EXTENSION_OWNED_END", "MDE_EXTENSION_OWNED_END_UNVERIFIED", owned_end_projection(references, extension).1),
            ("Extension-isRequired.1-spec", "UML_EXTENSION_IS_REQUIRED", "MDE_EXTENSION_IS_REQUIRED_UNVERIFIED", is_required(references, extension).map(|_| true)),
        ] {
            let (severity, code, message) = match result {
                Ok(true) => continue,
                Ok(false) => (StructuralSeverity::Error, error_code, "normative constraint is violated".to_owned()),
                Err(QueryError::Invalid { message, .. }) => (StructuralSeverity::Error, error_code, message),
                Err(QueryError::Unsupported { message, .. }) => (StructuralSeverity::Warning, unknown_code, message),
            };
            issues.push(StructuralDiagnostic { severity, code, subject_id:extension.id().into(), message:format!("{rule}: {message}") });
        }
    }
    for stereotype in references.model().elements().filter(|element| element.conforms_to("Stereotype")) {
        let outcome = stereotype_binary_associations(references, stereotype);
        let (severity, code, message) = match outcome {
            Ok(true) => continue,
            Ok(false) => (StructuralSeverity::Error, "UML_STEREOTYPE_BINARY_ASSOCIATIONS", "owned attributes may participate only in binary Associations".to_owned()),
            Err(QueryError::Invalid { message, .. }) => (StructuralSeverity::Error, "UML_STEREOTYPE_BINARY_ASSOCIATIONS", message),
            Err(QueryError::Unsupported { message, .. }) => (StructuralSeverity::Warning, "MDE_STEREOTYPE_BINARY_ASSOCIATIONS_UNVERIFIED", message),
        };
        issues.push(StructuralDiagnostic { severity, code, subject_id:stereotype.id().into(), message:format!("Stereotype-binaryAssociationsOnly: {message}") });
    }
    issues
}
