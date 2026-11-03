//! Executable UML 2.5.1 Property::isConsistentWith and reusable feature comparisons.
//! Original XMI/PDF contracts are authoritative. This composition is new; the
//! reused Classifier/Property query adaptations are credited in references/uml4net.

use super::{
    classifier_hierarchy,
    classifiers::QueryError,
    references::ReferenceIndex,
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
    if value.is_string() || value.get("href").is_some() {
        return Err(QueryError::unsupported(
            element.id(),
            "authored Type text and external Types cannot establish local Type conformance",
        ));
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
    compare_resolved_types(
        references,
        resolve_feature_type(references, specific)?,
        resolve_feature_type(references, general)?,
    )
}

/// User-model Type conformance, not schema/metaclass conformance.
pub fn compare_resolved_types(
    references: &ReferenceIndex<'_>,
    specific: ElementRef<'_>,
    general: ElementRef<'_>,
) -> Result<bool, QueryError> {
    if !specific.conforms_to("Type") || !general.conforms_to("Type") {
        return Err(QueryError::invalid(
            specific.id(),
            "conformance comparison requires Types",
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
    // The upper-bound clause in the published OCL repeats lowerBound. Follow
    // the accompanying interval-containment requirement, including infinity.
    require_all([
        compare_type_conformance(references, redefining, redefined),
        compare_multiplicity_containment(references, redefining, redefined),
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
