//! UML 2.5.1 §§9.6.3.1 and 9.9.11.6 over canonical ordered Parameters.
//! This comparison is new. Existing Operation projections and Classifier
//! conformance retain their uml4net provenance in references/uml4net/README.md.

use super::{
    classifiers::QueryError,
    feature_compatibility::{
        accept_any, compare_multiplicity_containment, compare_type_conformance, require_all,
    },
    operations,
    references::ReferenceIndex,
    root::ElementRef,
    schema,
};

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
        let specific_value = schema::read_effective_boolean(specific, key)
            .map_err(|error| QueryError::invalid(specific.id(), error))?;
        let general_value = schema::read_effective_boolean(general, key)
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
        operations::checked_return_parameter(references, operation.id())?;
    }
    let specific = operations::checked_owned_parameters(references, redefining.id())?;
    let general = operations::checked_owned_parameters(references, redefined.id())?;
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

/// UML Operation-isConsistentWith-spec compares formal signatures only; the
/// caller establishes its redefinition-context precondition. Conditions,
/// methods, exceptions and templates retain their independent validators and
/// do not add requirements to this normative query.
pub fn compare_operation_consistency(
    references: &ReferenceIndex<'_>,
    redefining: ElementRef<'_>,
    redefined: ElementRef<'_>,
) -> Result<bool, QueryError> {
    compare_operation_signature(references, redefining, redefined)
}
