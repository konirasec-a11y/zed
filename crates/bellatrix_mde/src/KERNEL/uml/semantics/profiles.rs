use super::{
    classifiers::QueryError,
    feature_compatibility,
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
    issues
}
