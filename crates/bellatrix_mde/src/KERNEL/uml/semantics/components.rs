use super::{
    classifiers::{self, QueryError},
    element_graph_queries::{self, ElementGraphError},
    references::ReferenceIndex,
    root::ElementRef,
};
use crate::{StructuralDiagnostic, StructuralSeverity};

fn canonical(references: &ReferenceIndex<'_>, element: ElementRef<'_>) -> Result<(), QueryError> {
    element_graph_queries::ensure_canonical_ownership(references, element).map_err(|error| {
        match error {
            ElementGraphError::Invalid {
                subject_id,
                message,
            } => QueryError::invalid(&subject_id, message),
            ElementGraphError::Unsupported {
                subject_id,
                message,
            } => QueryError::unsupported(&subject_id, message),
        }
    })
}

fn has_owned_role(
    references: &ReferenceIndex<'_>,
    element: ElementRef<'_>,
    role: &'static str,
) -> Result<bool, QueryError> {
    let mut unresolved = match canonical(references, element) {
        Ok(()) => None,
        Err(error @ QueryError::Unsupported { .. }) => Some(error),
        Err(error) => return Err(error),
    };
    let children = references
        .explicit_owned_elements(element.id())
        .map_err(|error| QueryError::invalid(element.id(), error))?;
    let mut present = false;
    for child in children {
        match classifiers::contributes_to_owned_union(element, child, role) {
            Ok(true) => present = true,
            Ok(false) => {}
            Err(error @ QueryError::Invalid { .. }) => return Err(error),
            Err(error) => unresolved = Some(error),
        }
    }
    if present {
        Ok(true)
    } else {
        unresolved.map_or(Ok(false), Err)
    }
}

fn has_nesting_class(
    references: &ReferenceIndex<'_>,
    component: ElementRef<'_>,
) -> Result<bool, QueryError> {
    if component
        .properties()
        .get("nestingClass")
        .is_some_and(|value| !value.is_null())
    {
        return Err(QueryError::unsupported(
            component.id(),
            "nestingClass must be derived from canonical Class::nestedClassifier ownership",
        ));
    }
    let Some(owner) = references
        .model()
        .owner(component)
        .map_err(|error| QueryError::invalid(component.id(), error))?
    else {
        return Ok(false);
    };
    if owner.metaclass().is_none() {
        return Err(QueryError::unsupported(
            component.id(),
            "nesting owner metaclass is unresolved",
        ));
    }
    if !owner.conforms_to("Class") {
        return Ok(false);
    }
    match canonical(references, owner) {
        Ok(()) | Err(QueryError::Unsupported { .. }) => {}
        Err(error) => return Err(error),
    }
    // A Component is a Class too, but its packagedElement ownership does not
    // establish the nestingClass opposite of Class::nestedClassifier.
    classifiers::contributes_to_owned_union(owner, component, "Class-nestedClassifier")
}

fn has_forbidden_packaged_elements(
    references: &ReferenceIndex<'_>,
    component: ElementRef<'_>,
) -> Result<bool, QueryError> {
    match (
        has_nesting_class(references, component),
        has_owned_role(references, component, "Component-packagedElement"),
    ) {
        (Err(error @ QueryError::Invalid { .. }), _)
        | (_, Err(error @ QueryError::Invalid { .. })) => Err(error),
        (Ok(false), _) | (_, Ok(false)) => Ok(false),
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(true), Ok(true)) => Ok(true),
    }
}

pub(crate) fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    for component in references
        .model()
        .elements()
        .filter(|element| element.conforms_to("Component"))
    {
        for (rule, code, result) in [
            (
                "Component-no_nested_classifiers",
                "UML_COMPONENT_NO_NESTED_CLASSIFIERS",
                has_owned_role(references, component, "Class-nestedClassifier"),
            ),
            (
                "Component-no_packaged_elements",
                "UML_COMPONENT_NO_PACKAGED_ELEMENTS",
                has_forbidden_packaged_elements(references, component),
            ),
        ] {
            let (severity, code, message) = match result {
                Ok(false) => continue,
                Ok(true) => (
                    StructuralSeverity::Error,
                    code,
                    format!("{rule}: forbidden composite membership"),
                ),
                Err(QueryError::Invalid { message, .. }) => (
                    StructuralSeverity::Error,
                    "MDE_COMPONENT_OWNERSHIP",
                    format!("{rule}: {message}"),
                ),
                Err(QueryError::Unsupported { message, .. }) => (
                    StructuralSeverity::Warning,
                    "MDE_COMPONENT_CONSTRAINT_UNVERIFIED",
                    format!("{rule}: {message}"),
                ),
            };
            issues.push(StructuralDiagnostic {
                severity,
                code,
                subject_id: component.id().into(),
                message,
            });
        }
    }
    issues
}
