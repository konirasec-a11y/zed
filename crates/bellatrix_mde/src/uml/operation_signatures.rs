//! UML 2.5.1 §§9.6.3.1 and 9.9.11.6 over canonical ordered Parameters.
//! This comparison is new. Existing Operation projections and Classifier
//! conformance retain their uml4net provenance in references/uml4net/README.md.

use super::{
    classifiers::QueryError,
    feature_compatibility::{
        accept_any, compare_multiplicity_containment, compare_type_conformance, require_all,
    },
    operations,
    references::{ReferenceIndex, reference_id},
    root::ElementRef,
};
use serde_json::Value;

fn populated(value: &Value) -> bool {
    !value.is_null() && !value.as_array().is_some_and(Vec::is_empty)
}

fn compare_parameter_signature(
    references: &ReferenceIndex<'_>,
    redefining: &crate::Element,
    redefined: &crate::Element,
) -> Result<bool, QueryError> {
    let specific = ElementRef::Object(redefining);
    let general = ElementRef::Object(redefined);
    let specific_direction = operations::direction(redefining)
        .map_err(|error| QueryError::invalid(specific.id(), error))?;
    let general_direction = operations::direction(redefined)
        .map_err(|error| QueryError::invalid(general.id(), error))?;
    let mut checks = vec![Ok(specific_direction == general_direction)];
    for key in ["isOrdered", "isUnique"] {
        let specific_value = operations::effective_boolean(specific, key)
            .map_err(|error| QueryError::invalid(specific.id(), error))?;
        let general_value = operations::effective_boolean(general, key)
            .map_err(|error| QueryError::invalid(general.id(), error))?;
        checks.push(Ok(specific_value == general_value));
    }
    // UML deliberately does not choose a language's covariance policy. Its
    // isConsistentWith body accepts Type conformance in either direction.
    checks.push(accept_any([
        compare_type_conformance(references, specific, general),
        compare_type_conformance(references, general, specific),
    ]));
    checks.push(match specific_direction.as_str() {
        "in" => compare_multiplicity_containment(references, general, specific),
        "out" | "return" => compare_multiplicity_containment(references, specific, general),
        "inout" => require_all([
            compare_multiplicity_containment(references, specific, general),
            compare_multiplicity_containment(references, general, specific),
        ]),
        _ => {
            return Err(QueryError::invalid(
                specific.id(),
                "unknown Parameter direction",
            ));
        }
    });
    require_all(checks)
}

/// Compares only the formal signature. The caller establishes the redefinition
/// context; names and visibility are not additional signature requirements.
pub fn compare_operation_signature(
    references: &ReferenceIndex<'_>,
    redefining: ElementRef<'_>,
    redefined: ElementRef<'_>,
) -> Result<bool, QueryError> {
    if !redefining.conforms_to("Operation") || !redefined.conforms_to("Operation") {
        return Ok(false);
    }
    for operation in [redefining, redefined] {
        if operation.properties().contains_key("ownedParameter") {
            return Err(QueryError::unsupported(
                operation.id(),
                "Operation signature requires ordered canonical Parameter ownership, not a stored ownedParameter list",
            ));
        }
        operations::return_parameter(references, operation.id())
            .map_err(|error| QueryError::invalid(operation.id(), error))?;
    }
    let specific = operations::owned_parameters(references, redefining.id())
        .map_err(|error| QueryError::invalid(redefining.id(), error))?;
    let general = operations::owned_parameters(references, redefined.id())
        .map_err(|error| QueryError::invalid(redefined.id(), error))?;
    if specific.len() != general.len() {
        return Ok(false);
    }
    require_all(
        specific
            .into_iter()
            .zip(general)
            .map(|(specific, general)| compare_parameter_signature(references, specific, general)),
    )
}

/// A compatible signature does not establish behavioral/template contract
/// consistency. Preserve that boundary without interpreting opaque expressions.
pub fn compare_operation_consistency(
    references: &ReferenceIndex<'_>,
    redefining: ElementRef<'_>,
    redefined: ElementRef<'_>,
) -> Result<bool, QueryError> {
    if !compare_operation_signature(references, redefining, redefined)? {
        return Ok(false);
    }
    for operation in [redefining, redefined] {
        for key in [
            "precondition",
            "postcondition",
            "bodyCondition",
            "method",
            "raisedException",
            "ownedTemplateSignature",
            "templateBinding",
            "templateParameter",
        ] {
            if operation.properties().get(key).is_some_and(populated) {
                return Err(QueryError::unsupported(
                    operation.id(),
                    format!(
                        "Operation signature is compatible; {key} contract compatibility is not evaluated"
                    ),
                ));
            }
        }
        let owned = references
            .model()
            .owned_elements(operation)
            .map_err(|error| QueryError::invalid(operation.id(), error))?;
        if owned.iter().any(|child| {
            child.conforms_to("Constraint")
                || child.conforms_to("TemplateSignature")
                || child.conforms_to("Behavior")
        }) || references.model().relationship_records().any(|relation| {
            relation.from == operation.id() && relation.kind == "uml.template_binding"
        }) || references.model().elements().any(|candidate| {
            candidate.conforms_to("Behavior")
                && reference_id(
                    candidate
                        .properties()
                        .get("specification")
                        .unwrap_or(&Value::Null),
                )
                .ok()
                .flatten()
                    == Some(operation.id())
        }) {
            return Err(QueryError::unsupported(
                operation.id(),
                "Operation signature is compatible; behavioral/template contracts are not evaluated",
            ));
        }
    }
    Ok(true)
}
