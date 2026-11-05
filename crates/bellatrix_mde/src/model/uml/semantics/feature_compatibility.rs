//! Executable UML 2.5.1 Property::isConsistentWith and reusable feature comparisons.
//! Original XMI/PDF contracts are authoritative. This composition is new; the
//! reused Classifier/Property query adaptations are credited in references/uml4net.

use super::{
    classifier_hierarchy,
    classifiers::QueryError,
    references::{ReferenceIndex, external_reference_href},
    root::ElementRef,
    schema,
    types::{MultiplicityRange, UnlimitedNatural},
    value_specifications::{self, EvaluationError},
};
use serde_json::Value;

pub fn resolve_feature_type<'a>(
    references: &ReferenceIndex<'a>,
    element: ElementRef<'a>,
) -> Result<ElementRef<'a>, QueryError> {
    if element.metaclass().is_none() {
        return Err(QueryError::unsupported(
            element.id(),
            "TypedElement metaclass is unresolved",
        ));
    }
    if !element.conforms_to("TypedElement") {
        return Err(QueryError::invalid(
            element.id(),
            "type comparison requires TypedElements",
        ));
    }
    let descriptor = element
        .metaclass()
        .and_then(|metaclass| schema::uml_property(metaclass, "type").ok().flatten())
        .ok_or_else(|| {
            QueryError::unsupported(element.id(), "TypedElement type contract unavailable")
        })?;
    if descriptor.is_derived {
        return Err(QueryError::unsupported(
            element.id(),
            "derived Type requires its semantic projection",
        ));
    }
    let value = element.properties().get("type").unwrap_or(&Value::Null);
    if value.is_string()
        || external_reference_href(value)
            .map_err(|error| QueryError::invalid(element.id(), error))?
            .is_some()
    {
        return Err(QueryError::unsupported(
            element.id(),
            "authored Type text and external Types cannot establish local Type conformance",
        ));
    }
    match super::property_contracts::check_value(references, element, descriptor, value) {
        super::property_contracts::PropertyOutcome::Satisfied => {}
        super::property_contracts::PropertyOutcome::Violation(message) => {
            return Err(QueryError::invalid(element.id(), message));
        }
        super::property_contracts::PropertyOutcome::Unverified(message) => {
            return Err(QueryError::unsupported(element.id(), message));
        }
    }
    references
        .validate(descriptor, value)
        .map_err(|error| QueryError::invalid(element.id(), error))?
        .ok_or_else(|| {
            QueryError::unsupported(
                element.id(),
                "absent Type cannot establish redefinition consistency",
            )
        })
}

pub fn compare_type_conformance(
    references: &ReferenceIndex<'_>,
    specific: ElementRef<'_>,
    general: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let specific = resolve_feature_type(references, specific);
    let general = resolve_feature_type(references, general);
    for result in [&specific, &general] {
        if let Err(error @ QueryError::Invalid { .. }) = result {
            return Err(error.clone());
        }
    }
    compare_resolved_types(references, specific?, general?)
}

/// User-model Type conformance, not schema/metaclass conformance.
pub fn compare_resolved_types(
    references: &ReferenceIndex<'_>,
    specific: ElementRef<'_>,
    general: ElementRef<'_>,
) -> Result<bool, QueryError> {
    if [specific, general]
        .iter()
        .any(|element| element.metaclass().is_some() && !element.conforms_to("Type"))
    {
        return Err(QueryError::invalid(
            specific.id(),
            "conformance comparison requires Types",
        ));
    }
    if specific.metaclass().is_none() || general.metaclass().is_none() {
        return Err(QueryError::unsupported(
            specific.id(),
            "Type metaclass is unresolved",
        ));
    }
    if specific.conforms_to("Classifier") {
        classifier_hierarchy::conforms_to(references, specific.id(), general.id())
    } else {
        // Type::conformsTo has the explicit default body false; Classifier refines it.
        Ok(false)
    }
}

pub fn resolve_feature_multiplicity(
    references: &ReferenceIndex<'_>,
    element: ElementRef<'_>,
) -> Result<MultiplicityRange, QueryError> {
    value_specifications::multiplicity(references, element)
        .map_err(|error| classify_value_error(element.id(), error))
}

pub fn resolve_feature_upper_bound(
    references: &ReferenceIndex<'_>,
    element: ElementRef<'_>,
) -> Result<UnlimitedNatural, QueryError> {
    value_specifications::upper_bound(references, element)
        .map_err(|error| classify_value_error(element.id(), error))
}

fn classify_value_error(id: &str, error: EvaluationError) -> QueryError {
    match error {
        EvaluationError::Invalid { message, .. } => QueryError::invalid(id, message),
        EvaluationError::Unsupported { message, .. } => QueryError::unsupported(id, message),
    }
}

pub fn compare_multiplicity_containment(
    references: &ReferenceIndex<'_>,
    specific: ElementRef<'_>,
    general: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let specific = resolve_feature_multiplicity(references, specific)?;
    let general = resolve_feature_multiplicity(references, general)?;
    Ok(general.includes_multiplicity(&specific))
}

/// Call within a valid redefinition context; the Classifier query establishes
/// that precondition. This result covers Property consistency, not subsetting.
pub fn compare_property_consistency(
    references: &ReferenceIndex<'_>,
    redefining: ElementRef<'_>,
    redefined: ElementRef<'_>,
) -> Result<bool, QueryError> {
    let (ElementRef::Object(specific), ElementRef::Object(general)) = (redefining, redefined)
    else {
        return Err(QueryError::unsupported(
            redefining.id(),
            "Property consistency requires canonical Property records",
        ));
    };
    if !redefining.conforms_to("Property") || !redefined.conforms_to("Property") {
        return Ok(false);
    }
    let composite = (|| {
        let general = super::associations::is_composite(general)
            .map_err(|error| QueryError::invalid(redefined.id(), error))?;
        let specific = super::associations::is_composite(specific)
            .map_err(|error| QueryError::invalid(redefining.id(), error))?;
        Ok(!general || specific)
    })();
    // UML 2.5.1 §9.9.17.7 repeats lowerBound in the upper-bound clause.
    // UMLR-424 proposes upperBound instead, but remains an open issue, not an
    // adopted erratum: preserve the published OCL rather than interval containment.
    // Bounds must be queried independently: a false lower-bound clause decides
    // the conjunction even when an upper ValueSpecification is unevaluated.
    // Malformed bounds remain errors; computable typed-null projections use
    // the bound query's normative fallback, with invariants checked separately.
    let specific_lower = value_specifications::lower_bound(references, redefining)
        .map_err(|error| classify_value_error(redefining.id(), error));
    let general_lower = value_specifications::lower_bound(references, redefined)
        .map_err(|error| classify_value_error(redefined.id(), error));
    // Evaluate both sides before choosing an error: an unsupported expression
    // must not mask a structurally invalid bound on the other Property.
    let lower_bounds = match (specific_lower, general_lower) {
        (Err(error @ QueryError::Invalid { .. }), _)
        | (_, Err(error @ QueryError::Invalid { .. })) => Err(error),
        (Err(error), _) | (_, Err(error)) => Err(error),
        (Ok(specific), Ok(general)) => Ok((specific, general)),
    };
    let lower_clause = lower_bounds
        .clone()
        .map(|(specific, general)| specific >= general);
    // Resolved bounds are non-null, including defaults. Unsupported is unknown,
    // not an empty bound. A true consequent also decides an implication whose
    // antecedent cannot be evaluated (not antecedent or consequent).
    let upper_guard = require_all([
        resolve_feature_upper_bound(references, redefining).map(|_| true),
        resolve_feature_upper_bound(references, redefined).map(|_| true),
    ]);
    let upper_clause = accept_any([
        upper_guard.map(|present| !present),
        lower_bounds.map(|(specific, general)| specific <= general),
    ]);
    require_all([
        compare_type_conformance(references, redefining, redefined),
        lower_clause,
        upper_clause,
        composite,
    ])
}

fn combine_checks(
    results: impl IntoIterator<Item = Result<bool, QueryError>>,
    decisive: bool,
) -> Result<bool, QueryError> {
    let mut unknown = None;
    let mut result = !decisive;
    for check in results {
        match check {
            Ok(value) if value == decisive => result = decisive,
            Ok(_) => {}
            Err(error @ QueryError::Unsupported { .. }) => unknown = Some(error),
            Err(error) => return Err(error),
        }
    }
    if result == decisive {
        Ok(result)
    } else {
        unknown.map_or(Ok(result), Err)
    }
}

pub(crate) fn require_all(
    results: impl IntoIterator<Item = Result<bool, QueryError>>,
) -> Result<bool, QueryError> {
    combine_checks(results, false)
}

pub(crate) fn accept_any(
    results: impl IntoIterator<Item = Result<bool, QueryError>>,
) -> Result<bool, QueryError> {
    combine_checks(results, true)
}
