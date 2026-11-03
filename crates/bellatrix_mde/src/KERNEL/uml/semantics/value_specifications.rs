//! Pure, bounded UML ValueSpecification evaluation over one canonical graph.
//! OMG UML 2.5.1 §§7.8.8 and 8.6; generated defaults and scalar contracts remain
//! authoritative. Nonliteral evaluation is deliberately not inferred from text.

use super::{
    references::{ReferenceIndex, external_reference_href, reference_id},
    root::ElementRef,
    schema,
    types::{
        Integer, MultiplicityError, MultiplicityRange, UnlimitedNatural, literal_multiplicity,
        literal_lower_bound, literal_upper_bound,
    },
};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum EvaluationError {
    #[error("Invalid UML value on {subject_id}: {message}")]
    Invalid { subject_id: String, message: String },
    #[error("Unsupported UML value on {subject_id}: {message}")]
    Unsupported { subject_id: String, message: String },
}

impl EvaluationError {
    fn invalid(id: &str, message: impl Into<String>) -> Self {
        Self::Invalid {
            subject_id: id.into(),
            message: message.into(),
        }
    }

    fn unsupported(id: &str, message: impl Into<String>) -> Self {
        Self::Unsupported {
            subject_id: id.into(),
            message: message.into(),
        }
    }

    pub fn is_unsupported(&self) -> bool {
        matches!(self, Self::Unsupported { .. })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum LiteralValue {
    Integer(Integer),
    UnlimitedNatural(UnlimitedNatural),
    Real(f64),
    Boolean(bool),
    String(Option<String>),
    Null,
}

fn resolve<'a>(
    references: &ReferenceIndex<'a>,
    id: &str,
) -> Result<ElementRef<'a>, EvaluationError> {
    references
        .element(id)
        .map_err(|error| EvaluationError::invalid(id, error))
}

/// Values are obtained from typed literal records, never parsed from their names.
/// Other ValueSpecifications can be valid UML, but this evaluator cannot execute them.
pub fn evaluate(
    references: &ReferenceIndex<'_>,
    id: &str,
) -> Result<LiteralValue, EvaluationError> {
    evaluate_literal(references, id)?.ok_or_else(|| {
        EvaluationError::unsupported(id, "nonliteral ValueSpecification evaluation is not implemented")
    })
}

fn evaluate_literal(
    references: &ReferenceIndex<'_>,
    id: &str,
) -> Result<Option<LiteralValue>, EvaluationError> {
    let element = resolve(references, id)?;
    if !element.conforms_to("ValueSpecification") {
        return Err(EvaluationError::invalid(
            id,
            "a ValueSpecification is required",
        ));
    }
    let metaclass = element
        .metaclass()
        .ok_or_else(|| EvaluationError::unsupported(id, "missing metaclass"))?;
    if metaclass == "LiteralNull" {
        return Ok(Some(LiteralValue::Null));
    }
    if !matches!(
        metaclass,
        "LiteralInteger" | "LiteralUnlimitedNatural" | "LiteralReal" | "LiteralBoolean" | "LiteralString"
    ) {
        return Ok(None);
    }
    let descriptor = schema::uml_property(metaclass, "value")
        .map_err(|error| EvaluationError::unsupported(id, error.to_string()))?
        .ok_or_else(|| EvaluationError::unsupported(id, "literal value contract missing"))?;
    let value = match element.properties().get("value") {
        Some(value) => value.clone(),
        None => schema::effective_default(descriptor)
            .map_err(|error| EvaluationError::unsupported(id, error.to_string()))?
            .and_then(|default| default.scalar_json())
            .unwrap_or(Value::Null),
    };
    schema::validate_scalar(descriptor, &value)
        .map_err(|error| EvaluationError::unsupported(id, error.to_string()))?
        .map_err(|error| EvaluationError::invalid(id, error.to_string()))?;
    let decimal = || match &value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        _ => String::new(),
    };
    let literal = match metaclass {
        "LiteralInteger" => decimal().parse().map(LiteralValue::Integer).map_err(
            |error: super::types::LiteralValueError| {
                EvaluationError::invalid(id, error.to_string())
            },
        ),
        "LiteralUnlimitedNatural" => decimal()
            .parse()
            .map(LiteralValue::UnlimitedNatural)
            .map_err(|error: super::types::LiteralValueError| {
                EvaluationError::invalid(id, error.to_string())
            }),
        "LiteralReal" => value
            .as_f64()
            .map(LiteralValue::Real)
            .ok_or_else(|| EvaluationError::invalid(id, "Real value required")),
        "LiteralBoolean" => value
            .as_bool()
            .map(LiteralValue::Boolean)
            .ok_or_else(|| EvaluationError::invalid(id, "Boolean value required")),
        "LiteralString" => Ok(LiteralValue::String(value.as_str().map(str::to_owned))),
        _ => Err(EvaluationError::unsupported(
            id,
            "literal query not implemented",
        )),
    }?;
    Ok(Some(literal))
}

/// A malformed literal is an error, not an expression whose computation is unsupported.
pub fn is_computable(
    references: &ReferenceIndex<'_>,
    id: &str,
) -> Result<bool, EvaluationError> {
    Ok(evaluate_literal(references, id)?.is_some())
}

/// UML's base queries return null/false for nonliteral ValueSpecifications.
/// Use is_computable() or evaluate() to distinguish that result from a literal
/// with no value of the requested type; malformed literals still return errors.
pub fn integer_value(
    references: &ReferenceIndex<'_>,
    id: &str,
) -> Result<Option<Integer>, EvaluationError> {
    Ok(match evaluate_literal(references, id)? {
        Some(LiteralValue::Integer(value)) => Some(value),
        _ => None,
    })
}

pub fn unlimited_value(
    references: &ReferenceIndex<'_>,
    id: &str,
) -> Result<Option<UnlimitedNatural>, EvaluationError> {
    Ok(match evaluate_literal(references, id)? {
        Some(LiteralValue::UnlimitedNatural(value)) => Some(value),
        _ => None,
    })
}

pub fn real_value(
    references: &ReferenceIndex<'_>,
    id: &str,
) -> Result<Option<f64>, EvaluationError> {
    Ok(match evaluate_literal(references, id)? {
        Some(LiteralValue::Real(value)) => Some(value),
        _ => None,
    })
}

pub fn boolean_value(
    references: &ReferenceIndex<'_>,
    id: &str,
) -> Result<Option<bool>, EvaluationError> {
    Ok(match evaluate_literal(references, id)? {
        Some(LiteralValue::Boolean(value)) => Some(value),
        _ => None,
    })
}

pub fn string_value(
    references: &ReferenceIndex<'_>,
    id: &str,
) -> Result<Option<String>, EvaluationError> {
    Ok(match evaluate_literal(references, id)? {
        Some(LiteralValue::String(value)) => value,
        _ => None,
    })
}

pub fn is_null(
    references: &ReferenceIndex<'_>,
    id: &str,
) -> Result<bool, EvaluationError> {
    Ok(matches!(evaluate_literal(references, id)?, Some(LiteralValue::Null)))
}

pub fn has_bound_references(element: ElementRef<'_>) -> bool {
    ["lowerValue", "upperValue"].iter().any(|key| {
        element
            .properties()
            .get(*key)
            .is_some_and(|value| !value.is_null())
    })
}

/// The reference names the bound role; `owns` remains the sole owner authority.
pub(crate) fn bound_role(owner: ElementRef<'_>, child: ElementRef<'_>) -> Option<&'static str> {
    if !owner.conforms_to("MultiplicityElement") || !child.conforms_to("ValueSpecification") {
        return None;
    }
    ["lowerValue", "upperValue"].into_iter().find(|key| {
        owner
            .properties()
            .get(*key)
            .and_then(|value| reference_id(value).ok().flatten())
            == Some(child.id())
    })
}

fn bound<'a>(
    references: &ReferenceIndex<'a>,
    element: ElementRef<'a>,
    key: &str,
) -> Result<Option<ElementRef<'a>>, EvaluationError> {
    let Some(value) = element
        .properties()
        .get(key)
        .filter(|value| !value.is_null())
    else {
        return Ok(None);
    };
    if external_reference_href(value)
        .map_err(|error| EvaluationError::invalid(element.id(), error))?.is_some()
    {
        return Err(EvaluationError::unsupported(
            element.id(),
            format!("external {key} cannot be resolved locally"),
        ));
    }
    let id = reference_id(value)
        .map_err(|error| EvaluationError::invalid(element.id(), error))?
        .ok_or_else(|| EvaluationError::invalid(element.id(), "bound identity is missing"))?;
    let target = resolve(references, id)?;
    let descriptor = schema::uml_property("MultiplicityElement", key)
        .map_err(|error| EvaluationError::unsupported(element.id(), error.to_string()))?
        .ok_or_else(|| EvaluationError::unsupported(element.id(), "bound contract missing"))?;
    references
        .validate(descriptor, value)
        .map_err(|error| EvaluationError::invalid(element.id(), error))?;
    if references
        .model()
        .owner(target)
        .map_err(|error| EvaluationError::invalid(id, error))?
        .is_none_or(|owner| owner.id() != element.id())
    {
        return Err(EvaluationError::invalid(
            id,
            format!("{key} must be owned by {}", element.id()),
        ));
    }
    Ok(Some(target))
}

fn require_assigned_values(
    references: &ReferenceIndex<'_>,
    element: ElementRef<'_>,
) -> Result<(), EvaluationError> {
    if references
        .model()
        .owned_elements(element)
        .map_err(|error| EvaluationError::invalid(element.id(), error))?
        .iter()
        .any(|child| child.conforms_to("ValueSpecification"))
    {
        return Err(EvaluationError::unsupported(
            element.id(),
            "owned ValueSpecification role is not represented by lowerValue/upperValue",
        ));
    }
    Ok(())
}

fn classify_multiplicity_error(id: &str, error: MultiplicityError) -> EvaluationError {
    if error == MultiplicityError::UnsupportedValueSpecification {
        EvaluationError::unsupported(id, error.to_string())
    } else {
        EvaluationError::invalid(id, error.to_string())
    }
}

fn evaluate_upper_bound(
    references: &ReferenceIndex<'_>,
    element: ElementRef<'_>,
) -> Result<UnlimitedNatural, EvaluationError> {
    bound(references, element, "upperValue")?.map_or(Ok(1.into()), |target| {
        match evaluate(references, target.id())? {
            LiteralValue::UnlimitedNatural(value) => Ok(value),
            _ => Err(EvaluationError::invalid(target.id(), "MultiplicityElement::upper_is_unlimitedNatural requires a non-null unlimitedValue()")),
        }
    })
}

/// UML lowerBound() does not depend on whether upperValue can be evaluated.
pub fn lower_bound(
    references: &ReferenceIndex<'_>,
    element: ElementRef<'_>,
) -> Result<Integer, EvaluationError> {
    if !element.conforms_to("MultiplicityElement") {
        return Err(EvaluationError::invalid(element.id(), "a MultiplicityElement is required"));
    }
    if has_bound_references(element) {
        return bound(references, element, "lowerValue")?.map_or(Ok(1.into()), |target| {
            match evaluate(references, target.id())? {
                LiteralValue::Integer(value) => Ok(value),
                // lowerBound's fallback is distinct from lower_is_integer.
                // A computable value with no Integer projection yields 1;
                // multiplicity() still diagnoses the violated invariant.
                _ => Ok(1.into()),
            }
        });
    }
    require_assigned_values(references, element)?;
    literal_lower_bound(element.properties())
        .map_err(|error| classify_multiplicity_error(element.id(), error))
}

/// UML upperBound() is independent of lowerValue evaluation. Overall interval
/// validity is checked separately by multiplicity(), never inferred here.
pub fn upper_bound(
    references: &ReferenceIndex<'_>,
    element: ElementRef<'_>,
) -> Result<UnlimitedNatural, EvaluationError> {
    if !element.conforms_to("MultiplicityElement") {
        return Err(EvaluationError::invalid(
            element.id(),
            "a MultiplicityElement is required",
        ));
    }
    if has_bound_references(element) {
        return bound(references, element, "upperValue")?.map_or(Ok(1.into()), |target| {
            match evaluate(references, target.id())? {
                LiteralValue::UnlimitedNatural(value) => Ok(value),
                // Keep the query fallback separate from the strict validator.
                _ => Ok(1.into()),
            }
        });
    }
    require_assigned_values(references, element)?;
    literal_upper_bound(element.properties())
        .map_err(|error| classify_multiplicity_error(element.id(), error))
}

/// Resolve identity bounds before the retained notation adapter. Unknown values
/// are not absence; they cannot prove a valid interval or composition bound.
pub fn multiplicity(
    references: &ReferenceIndex<'_>,
    element: ElementRef<'_>,
) -> Result<MultiplicityRange, EvaluationError> {
    if !element.conforms_to("MultiplicityElement") {
        return Err(EvaluationError::invalid(
            element.id(),
            "a MultiplicityElement is required",
        ));
    }
    if !has_bound_references(element) {
        require_assigned_values(references, element)?;
        return literal_multiplicity(element.properties())
            .map_err(|error| classify_multiplicity_error(element.id(), error));
    }
    let lower = (|| bound(references, element, "lowerValue")?
        .map(|target| match evaluate(references, target.id())? {
            LiteralValue::Integer(value) => Ok(value),
            _ => Err(EvaluationError::invalid(
                target.id(),
                "MultiplicityElement::lower_is_integer requires a non-null integerValue()",
            )),
        })
        .transpose())();
    let lower = lower.and_then(|lower| {
        if lower.as_ref().is_some_and(Integer::is_negative) {
            Err(EvaluationError::invalid(element.id(), "lower bound cannot be negative"))
        } else { Ok(lower) }
    });
    // Evaluate both bounds before selecting Unknown; an invalid opposite bound
    // remains a violation even when its neighbor is an opaque expression.
    let upper = evaluate_upper_bound(references, element).map(Some);
    let (lower, upper) = match (lower, upper) {
        (Err(error @ EvaluationError::Invalid { .. }), _) | (_, Err(error @ EvaluationError::Invalid { .. })) => return Err(error),
        (Err(error), _) | (_, Err(error)) => return Err(error),
        (Ok(lower), Ok(upper)) => (lower, upper),
    };
    MultiplicityRange::from_bounds(lower, upper)
        .map_err(|error| EvaluationError::invalid(element.id(), error.to_string()))
}

/// Structural results only. Callers decide whether an edit is semantic or a
/// permissive visual draft; evaluation never changes the graph.
pub fn validate(references: &ReferenceIndex<'_>) -> Vec<crate::StructuralDiagnostic> {
    references
        .model()
        .elements()
        .filter(|element| element.conforms_to("MultiplicityElement"))
        .filter_map(|element| {
            multiplicity(references, element).err().map(|error| {
                let unsupported = error.is_unsupported();
                let parameter = element.conforms_to("Parameter");
                crate::StructuralDiagnostic {
                    severity: if unsupported {
                        crate::StructuralSeverity::Warning
                    } else {
                        crate::StructuralSeverity::Error
                    },
                    code: match (parameter, unsupported) {
                        (true, true) => "MDE_PARAMETER_MULTIPLICITY_UNSUPPORTED",
                        (true, false) => "UML_PARAMETER_MULTIPLICITY",
                        (false, true) => "MDE_MULTIPLICITY_UNSUPPORTED",
                        (false, false) => "UML_MULTIPLICITY_VALUE",
                    },
                    subject_id: element.id().into(),
                    message: error.to_string(),
                }
            })
        })
        .collect()
}
