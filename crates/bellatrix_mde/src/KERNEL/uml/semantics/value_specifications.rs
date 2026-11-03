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

fn default_lower_bound(element: ElementRef<'_>) -> Integer {
    if element.conforms_to("ExtensionEnd") { 0.into() } else { 1.into() }
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
        return bound(references, element, "lowerValue")?.map_or(Ok(default_lower_bound(element)), |target| {
            match evaluate(references, target.id())? {
                LiteralValue::Integer(value) => Ok(value),
                _ if element.conforms_to("ExtensionEnd") => Err(EvaluationError::invalid(
                    target.id(), "ExtensionEnd::lowerBound requires an Integer value",
                )),
                // lowerBound's fallback is distinct from lower_is_integer.
                // A computable value with no Integer projection yields 1;
                // multiplicity() still diagnoses the violated invariant.
                _ => Ok(1.into()),
            }
        });
    }
    require_assigned_values(references, element)?;
    if !element.properties().contains_key("multiplicity") {
        return Ok(default_lower_bound(element));
    }
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
        if !element.properties().contains_key("multiplicity") {
            return MultiplicityRange::from_bounds(Some(default_lower_bound(element)), None)
                .map_err(|error| EvaluationError::invalid(element.id(), error.to_string()));
        }
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
    MultiplicityRange::from_bounds(Some(lower.unwrap_or_else(|| default_lower_bound(element))), upper)
        .map_err(|error| EvaluationError::invalid(element.id(), error.to_string()))
}

fn string_expression_operands(
    references: &ReferenceIndex<'_>,
    expression: ElementRef<'_>,
) -> Result<bool, super::classifiers::QueryError> {
    use super::{classifiers::{self, QueryError}, property_contracts::{self, PropertyOutcome}};
    let mut checks = Vec::new();
    let literal_string = |operand: ElementRef<'_>| {
        if operand.metaclass().is_none() {
            Err(QueryError::unsupported(operand.id(), "operand metaclass is unresolved"))
        } else {
            Ok(operand.conforms_to("LiteralString"))
        }
    };
    if let Some(value) = expression.properties().get("operand") {
        let descriptor = schema::uml_property("StringExpression", "operand")
            .map_err(|error| QueryError::unsupported(expression.id(), error.to_string()))?
            .ok_or_else(|| QueryError::unsupported(expression.id(), "missing operand property contract"))?;
        checks.push(match property_contracts::check_value(references, expression, descriptor, value) {
            PropertyOutcome::Satisfied => Ok(true),
            PropertyOutcome::Violation(message) => Err(QueryError::invalid(expression.id(), message)),
            PropertyOutcome::Unverified(message) => Err(QueryError::unsupported(expression.id(), message)),
        });
        // A known non-string operand disproves forAll even beside unresolved
        // references. The complete contract check above still retains Invalid.
        if let Some(values) = value.as_array() {
            for value in values {
                if let Ok(Some(id)) = reference_id(value) {
                    if let Ok(operand) = references.element(id) {
                        checks.push(literal_string(operand));
                    }
                }
            }
        }
    }
    for child in references.explicit_owned_elements(expression.id())
        .map_err(|error| QueryError::invalid(expression.id(), error))?
    {
        // StringExpression children can also be subExpression or nameExpression;
        // ownership alone must not select the operand role for those children.
        match classifiers::contributes_to_owned_union(expression, child, "Expression-operand") {
            Ok(true) => checks.push(literal_string(child)),
            Ok(false) => {}
            Err(error) => checks.push(Err(error)),
        }
    }
    super::feature_compatibility::require_all(checks)
}

fn constrained_element_cardinality(
    references: &ReferenceIndex<'_>,
    constraint: ElementRef<'_>,
    maximum: usize,
) -> Result<bool, super::classifiers::QueryError> {
    use super::{classifiers::QueryError, property_contracts::{self, PropertyOutcome}};
    let Some(value) = constraint.properties().get("constrainedElement") else {
        return Ok(false);
    };
    let descriptor = schema::uml_property("Constraint", "constrainedElement")
        .map_err(|error| QueryError::unsupported(constraint.id(), error.to_string()))?
        .ok_or_else(|| QueryError::unsupported(constraint.id(), "missing constrainedElement contract"))?;
    let contract = match property_contracts::check_value(references, constraint, descriptor, value) {
        PropertyOutcome::Satisfied => Ok(true),
        PropertyOutcome::Violation(message) => Err(QueryError::invalid(constraint.id(), message)),
        PropertyOutcome::Unverified(message) => Err(QueryError::unsupported(constraint.id(), message)),
    };
    let count = value.as_array()
        .ok_or_else(|| QueryError::invalid(constraint.id(), "constrainedElement requires a collection"))?
        .len();
    // An external identity cannot repair an already disproven cardinality;
    // malformed or duplicate references still invalidate the collection itself.
    super::feature_compatibility::require_all([contract, Ok((1..=maximum).contains(&count))])
}

fn duration_first_event_multiplicity(
    references: &ReferenceIndex<'_>,
    element: ElementRef<'_>,
    metaclass: &str,
    events_property: &str,
) -> Result<bool, super::classifiers::QueryError> {
    use super::{classifiers::QueryError, property_contracts::{self, PropertyOutcome}};
    let empty = Value::Array(Vec::new());
    let mut checks = Vec::new();
    let mut lengths = [None; 2];
    for (position, name) in [events_property, "firstEvent"].into_iter().enumerate() {
        let descriptor = schema::uml_property(metaclass, name)
            .map_err(|error| QueryError::unsupported(element.id(), error.to_string()))?
            .ok_or_else(|| QueryError::unsupported(element.id(), "missing duration property contract"))?;
        if !element.properties().contains_key(name) && descriptor.lower != "0" {
            checks.push(Err(QueryError::unsupported(element.id(), format!("required {name} collection is not materialized"))));
            continue;
        }
        let value = element.properties().get(name).unwrap_or(&empty);
        checks.push(match property_contracts::check_value(references, element, descriptor, value) {
            PropertyOutcome::Satisfied => Ok(true),
            PropertyOutcome::Violation(message) => Err(QueryError::invalid(element.id(), message)),
            PropertyOutcome::Unverified(message) => Err(QueryError::unsupported(element.id(), message)),
        });
        lengths[position] = Some(value.as_array()
            .ok_or_else(|| QueryError::invalid(element.id(), format!("{name} requires a collection")))?
            .len());
    }
    checks.push(match (lengths[0], lengths[1]) {
        (Some(events), Some(first_events)) => Ok(first_events == if events == 2 { 2 } else { 0 }),
        // Both branches of the normative conditional exclude any other size,
        // even when a required event collection has not been materialized.
        (None, Some(first_events)) if first_events != 0 && first_events != 2 => Ok(false),
        _ => Err(QueryError::unsupported(element.id(), "duration collection cardinality is unresolved")),
    });
    super::feature_compatibility::require_all(checks)
}

/// Structural results only. Callers decide whether an edit is semantic or a
/// permissive visual draft; evaluation never changes the graph.
pub fn validate(references: &ReferenceIndex<'_>) -> Vec<crate::StructuralDiagnostic> {
    let mut issues = references
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
        .collect::<Vec<_>>();
    for expression in references.model().elements().filter(|element| element.conforms_to("StringExpression")) {
        let (severity, code, message) = match string_expression_operands(references, expression) {
            Ok(true) => continue,
            Ok(false) => (crate::StructuralSeverity::Error, "UML_STRING_EXPRESSION_OPERANDS", "all operands must be LiteralStrings".to_owned()),
            Err(super::classifiers::QueryError::Invalid { message, .. }) =>
                (crate::StructuralSeverity::Error, "UML_STRING_EXPRESSION_OPERANDS", message),
            Err(super::classifiers::QueryError::Unsupported { message, .. }) =>
                (crate::StructuralSeverity::Warning, "MDE_STRING_EXPRESSION_OPERANDS_UNVERIFIED", message),
        };
        issues.push(crate::StructuralDiagnostic { severity, code, subject_id: expression.id().into(), message: format!("StringExpression-operands: {message}") });
    }
    for constraint in references.model().elements() {
        let (maximum, rule, error_code, unknown_code) = if constraint.conforms_to("TimeConstraint") {
            (1, "TimeConstraint-has_one_constrainedElement", "UML_TIME_CONSTRAINT_CONSTRAINED_ELEMENTS", "MDE_TIME_CONSTRAINT_CONSTRAINED_ELEMENTS_UNVERIFIED")
        } else if constraint.conforms_to("DurationConstraint") {
            (2, "DurationConstraint-has_one_or_two_constrainedElements", "UML_DURATION_CONSTRAINT_CONSTRAINED_ELEMENTS", "MDE_DURATION_CONSTRAINT_CONSTRAINED_ELEMENTS_UNVERIFIED")
        } else { continue; };
        let (severity, code, message) = match constrained_element_cardinality(references, constraint, maximum) {
            Ok(true) => continue,
            Ok(false) => (crate::StructuralSeverity::Error, error_code, format!("requires 1..={maximum} constrainedElements")),
            Err(super::classifiers::QueryError::Invalid { message, .. }) =>
                (crate::StructuralSeverity::Error, error_code, message),
            Err(super::classifiers::QueryError::Unsupported { message, .. }) =>
                (crate::StructuralSeverity::Warning, unknown_code, message),
        };
        issues.push(crate::StructuralDiagnostic { severity, code, subject_id: constraint.id().into(), message: format!("{rule}: {message}") });
    }
    for element in references.model().elements() {
        let (metaclass, events_property, error_code, unknown_code) = if element.conforms_to("DurationConstraint") {
            ("DurationConstraint", "constrainedElement", "UML_DURATION_CONSTRAINT_FIRST_EVENT", "MDE_DURATION_CONSTRAINT_FIRST_EVENT_UNVERIFIED")
        } else if element.conforms_to("DurationObservation") {
            ("DurationObservation", "event", "UML_DURATION_OBSERVATION_FIRST_EVENT", "MDE_DURATION_OBSERVATION_FIRST_EVENT_UNVERIFIED")
        } else { continue; };
        let (severity, code, message) = match duration_first_event_multiplicity(references, element, metaclass, events_property) {
            Ok(true) => continue,
            Ok(false) => (crate::StructuralSeverity::Error, error_code, format!("firstEvent must contain two values exactly when {events_property} has two elements, and no values otherwise")),
            Err(super::classifiers::QueryError::Invalid { message, .. }) =>
                (crate::StructuralSeverity::Error, error_code, message),
            Err(super::classifiers::QueryError::Unsupported { message, .. }) =>
                (crate::StructuralSeverity::Warning, unknown_code, message),
        };
        issues.push(crate::StructuralDiagnostic { severity, code, subject_id: element.id().into(), message: format!("{metaclass}-first_event_multiplicity: {message}") });
    }
    issues
}
