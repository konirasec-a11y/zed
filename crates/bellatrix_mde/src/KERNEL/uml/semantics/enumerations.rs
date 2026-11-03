use super::{
    classifiers::{self, QueryError},
    property_contracts::{self, PropertyOutcome},
    references::ReferenceIndex,
    root::ElementRef,
    schema,
};
use crate::{StructuralDiagnostic, StructuralSeverity};

fn immutable(
    references: &ReferenceIndex<'_>,
    enumeration: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let mut unknown = None;
    for property in classifiers::properties(enumeration)? {
        if !classifiers::contributes(property, "DataType-ownedAttribute")
            .map_err(|error| QueryError::unsupported(enumeration.id(), error))?
        {
            continue;
        }
        if let Some(value) = enumeration.properties().get(property.name) {
            match property_contracts::check_value(references, enumeration, property, value) {
                PropertyOutcome::Satisfied => {}
                PropertyOutcome::Violation(message) => {
                    return Err(QueryError::invalid(enumeration.id(), message));
                }
                PropertyOutcome::Unverified(message) => {
                    unknown = Some(QueryError::unsupported(enumeration.id(), message))
                }
            }
        }
    }
    let attributes = references
        .explicit_owned_elements(enumeration.id())
        .map_err(|error| QueryError::invalid(enumeration.id(), error))?;
    let mut mutable = false;
    for attribute in attributes {
        if attribute.metaclass().is_some() && !attribute.conforms_to("Property") {
            continue;
        }
        match classifiers::contributes_to_owned_union(
            enumeration,
            attribute,
            "DataType-ownedAttribute",
        ) {
            Ok(false) => continue,
            Err(error @ QueryError::Invalid { .. }) => return Err(error),
            Err(error) => {
                unknown = Some(error);
                continue;
            }
            Ok(true) => {}
        }
        mutable |= !schema::read_effective_boolean(attribute, "isReadOnly")
            .map_err(|error| QueryError::invalid(attribute.id(), error))?;
    }
    // A local mutable attribute disproves forAll even with unloaded neighbors;
    // it cannot erase malformed values or identities in the inspected collection.
    if mutable {
        Ok(false)
    } else {
        unknown.map_or(Ok(true), Err)
    }
}

pub(crate) fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    for enumeration in references
        .model()
        .elements()
        .filter(|element| element.conforms_to("Enumeration"))
    {
        let (severity, code, message) = match immutable(references, enumeration) {
            Ok(true) => continue,
            Ok(false) => (
                StructuralSeverity::Error,
                "UML_ENUMERATION_IMMUTABLE",
                "every directly owned attribute must be read-only".into(),
            ),
            Err(QueryError::Invalid { message, .. }) => (
                StructuralSeverity::Error,
                "MDE_ENUMERATION_IMMUTABILITY",
                message,
            ),
            Err(QueryError::Unsupported { message, .. }) => (
                StructuralSeverity::Warning,
                "MDE_ENUMERATION_IMMUTABILITY_UNVERIFIED",
                message,
            ),
        };
        issues.push(StructuralDiagnostic {
            severity,
            code,
            subject_id: enumeration.id().into(),
            message: format!("Enumeration-immutable: {message}"),
        });
    }
    issues
}
