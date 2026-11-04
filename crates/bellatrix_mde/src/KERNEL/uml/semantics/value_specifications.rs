//! Pure, bounded UML ValueSpecification evaluation over one canonical graph.
//! OMG UML 2.5.1 §§7.8.8 and 8.6; generated defaults and scalar contracts remain
//! authoritative. Nonliteral evaluation is deliberately not inferred from text.

use super::{
    references::{ReferenceIndex, external_reference_href, reference_id},
    root::ElementRef,
    schema,
    types::{
        Integer, MultiplicityError, MultiplicityRange, StandardPrimitiveType, UnlimitedNatural, literal_multiplicity,
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

impl LiteralValue {
    pub fn primitive_type(&self) -> Option<StandardPrimitiveType> {
        match self {
            Self::Integer(_) => Some(StandardPrimitiveType::Integer),
            Self::UnlimitedNatural(_) => Some(StandardPrimitiveType::UnlimitedNatural),
            Self::Real(_) => Some(StandardPrimitiveType::Real),
            Self::Boolean(_) => Some(StandardPrimitiveType::Boolean),
            Self::String(_) => Some(StandardPrimitiveType::String),
            Self::Null => None,
        }
    }
}

/// Only source-qualified identities in the pinned primitive documents establish
/// normative types. A local Element named Integer is not that library identity.
pub(crate) fn standard_primitive_reference(
    value: &Value,
) -> Result<Option<StandardPrimitiveType>, String> {
    let Some(href) = external_reference_href(value)? else { return Ok(None); };
    let Some((document_url, fragment)) = href.split_once('#') else { return Ok(None); };
    let Some(source) = super::metamodel::SOURCES.iter().find(|source|
        source.url == document_url && matches!(source.document,
            schema::Document::PrimitiveTypes | schema::Document::PrimitiveTypes20131001))
    else { return Ok(None); };
    // Encoded fragments need URI normalization, which this exact-identity
    // adapter deliberately does not implement.
    if fragment.contains('%') { return Ok(None); }
    for primitive in StandardPrimitiveType::ALL {
        let identity = schema::XmiId::new(source.document, primitive.name());
        if let Some(classifier) = schema::classifier(identity) {
            if classifier.id.fragment == fragment && classifier.kind == schema::ClassifierKind::Primitive {
                return Ok(Some(primitive));
            }
        }
    }
    Err(format!("unknown PrimitiveType identity in pinned document: {href}"))
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

/// Resolve the represented instance identity without evaluating its Slots or
/// specification recursively. InstanceValue denotes that instance, not a copy
/// of its fields and not a primitive value guessed from its name.
pub fn referenced_instance<'a>(
    references: &ReferenceIndex<'a>,
    id: &str,
) -> Result<ElementRef<'a>, EvaluationError> {
    let element = resolve(references, id)?;
    if element.metaclass().is_none() {
        return Err(EvaluationError::unsupported(id, "InstanceValue metaclass is unresolved"));
    }
    if !element.conforms_to("InstanceValue") {
        return Err(EvaluationError::invalid(id, "an InstanceValue is required"));
    }
    let descriptor = schema::uml_property("InstanceValue", "instance")
        .map_err(|error| EvaluationError::unsupported(id, error.to_string()))?
        .ok_or_else(|| EvaluationError::unsupported(id, "InstanceValue instance contract is unavailable"))?;
    let value = element.properties().get("instance").unwrap_or(&Value::Null);
    match super::property_contracts::check_value(references, element, descriptor, value) {
        super::property_contracts::PropertyOutcome::Violation(message) => return Err(EvaluationError::invalid(id, message)),
        super::property_contracts::PropertyOutcome::Unverified(message) => return Err(EvaluationError::unsupported(id, message)),
        super::property_contracts::PropertyOutcome::Satisfied => {}
    }
    let target_id = reference_id(value)
        .map_err(|error| EvaluationError::invalid(id, error))?
        .ok_or_else(|| EvaluationError::invalid(id, "InstanceValue requires exactly one represented instance"))?;
    resolve(references, target_id)
}

fn evaluate_literal(
    references: &ReferenceIndex<'_>,
    id: &str,
) -> Result<Option<LiteralValue>, EvaluationError> {
    let element = resolve(references, id)?;
    let metaclass = element
        .metaclass()
        .ok_or_else(|| EvaluationError::unsupported(id, "missing metaclass"))?;
    if !element.conforms_to("ValueSpecification") {
        return Err(EvaluationError::invalid(
            id,
            "a ValueSpecification is required",
        ));
    }
    if element.conforms_to("InstanceValue") {
        referenced_instance(references, id)?;
        return Ok(None);
    }
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
    let contract = super::property_contracts::check_value(references, element, descriptor, value);
    // Ownership is known independently of the target's unresolved metaclass.
    // Its contradictions must remain invalid before selecting Unknown.
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
    match contract {
        super::property_contracts::PropertyOutcome::Satisfied => Ok(Some(target)),
        super::property_contracts::PropertyOutcome::Violation(message) => Err(EvaluationError::invalid(element.id(), message)),
        super::property_contracts::PropertyOutcome::Unverified(message) => Err(EvaluationError::unsupported(element.id(), message)),
    }
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

// Both prose constraints apply only to nonliteral bounds. Their consequents
// require separate semantic proofs; the literal evaluator proves neither for
// an arbitrary expression, even when its text looks like a constant.
fn bound_expression_guarantees(
    references: &ReferenceIndex<'_>,
    element: ElementRef<'_>,
) -> Result<(), EvaluationError> {
    let mut unknown = None;
    for key in ["lowerValue", "upperValue"] {
        let outcome = bound(references, element, key).and_then(|target| {
            match target {
                None => Ok(()),
                Some(target) if target.conforms_to("LiteralSpecification") => Ok(()),
                Some(target) => Err(EvaluationError::unsupported(
                    target.id(),
                    format!("{key}: constancy and absence of side effects of nonliteral expressions are not verified"),
                )),
            }
        });
        match outcome {
            Err(error @ EvaluationError::Invalid { .. }) => return Err(error),
            Err(error) => unknown = Some(error),
            Ok(()) => {}
        }
    }
    match unknown {
        Some(error) => Err(error),
        None => Ok(()),
    }
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

fn string_expression_subexpressions(
    references: &ReferenceIndex<'_>,
    expression: ElementRef<'_>,
) -> Result<bool, super::classifiers::QueryError> {
    use super::{classifiers::{self, QueryError}, feature_compatibility::{accept_any, require_all}, property_contracts::{self, PropertyOutcome}};
    let children = references.explicit_owned_elements(expression.id())
        .map_err(|error| QueryError::invalid(expression.id(), error))?;
    let mut contracts = Vec::new();
    let mut presences = [Ok(false), Ok(false)];
    for (position, (name, role)) in [("operand", "Expression-operand"), ("subExpression", "StringExpression-subExpression")].into_iter().enumerate() {
        let mut presence = Vec::new();
        if let Some(value) = expression.properties().get(name) {
            let descriptor = schema::uml_property("StringExpression", name)
                .map_err(|error| QueryError::unsupported(expression.id(), error.to_string()))?
                .ok_or_else(|| QueryError::unsupported(expression.id(), "missing StringExpression collection contract"))?;
            contracts.push(match property_contracts::check_value(references, expression, descriptor, value) {
                PropertyOutcome::Satisfied => Ok(true),
                PropertyOutcome::Violation(message) => Err(QueryError::invalid(expression.id(), message)),
                PropertyOutcome::Unverified(message) => Err(QueryError::unsupported(expression.id(), message)),
            });
            presence.push(value.as_array().map(|values| !values.is_empty())
                .ok_or_else(|| QueryError::invalid(expression.id(), format!("{name} requires a collection"))));
        }
        for child in &children {
            let membership = classifiers::contributes_to_owned_union(expression, *child, role);
            if expression.properties().contains_key(name) {
                // A stored collection establishes its cardinality even when a
                // child's role is unresolved. Ownership still guards validity.
                contracts.push(membership.map(|_| true));
            } else {
                presence.push(membership);
            }
        }
        presences[position] = accept_any(presence);
    }
    // The published conditional requires exactly one nonempty collection,
    // rather than merely forbidding two nonempty collections.
    let [operands, subexpressions] = presences;
    contracts.push(match (operands, subexpressions) {
        (Ok(operands), Ok(subexpressions)) => Ok(operands != subexpressions),
        (Err(error @ QueryError::Invalid { .. }), _) | (_, Err(error @ QueryError::Invalid { .. })) => Err(error),
        (Err(error), _) | (_, Err(error)) => Err(error),
    });
    require_all(contracts)
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

fn temporal_expression_observation(
    references: &ReferenceIndex<'_>,
    expression: ElementRef<'_>,
    metaclass: &str,
    expr_role: &'static str,
    observation_type: &str,
) -> Result<bool, super::classifiers::QueryError> {
    use super::{classifiers::{self, QueryError}, feature_compatibility::{accept_any, require_all}, property_contracts::{self, PropertyOutcome}};
    let contract = |name: &str, value: &Value| -> Result<bool, QueryError> {
        let descriptor = schema::uml_property(metaclass, name)
            .map_err(|error| QueryError::unsupported(expression.id(), error.to_string()))?
            .ok_or_else(|| QueryError::unsupported(expression.id(), "missing temporal expression property contract"))?;
        match property_contracts::check_value(references, expression, descriptor, value) {
            PropertyOutcome::Satisfied => Ok(true),
            PropertyOutcome::Violation(message) => Err(QueryError::invalid(expression.id(), message)),
            PropertyOutcome::Unverified(message) => Err(QueryError::unsupported(expression.id(), message)),
        }
    };
    let mut expr_contract = Ok(true);
    let mut presence = Vec::new();
    if let Some(value) = expression.properties().get("expr") {
        expr_contract = contract("expr", value);
        presence.push(match external_reference_href(value) {
            Ok(Some(_)) => Err(QueryError::unsupported(expression.id(), "expr reference has no loaded resolver")),
            Ok(None) => reference_id(value).map(|id| id.is_some())
                .map_err(|error| QueryError::invalid(expression.id(), error)),
            Err(error) => Err(QueryError::invalid(expression.id(), error)),
        });
    }
    let mut owned_exprs = 0;
    for child in references.explicit_owned_elements(expression.id())
        .map_err(|error| QueryError::invalid(expression.id(), error))?
    {
        match classifiers::contributes_to_owned_union(expression, child, expr_role) {
            Ok(true) => { owned_exprs += 1; presence.push(Ok(true)); }
            Ok(false) => {}
            Err(error) => presence.push(Err(error)),
        }
    }
    if owned_exprs > 1 {
        presence.push(Err(QueryError::invalid(expression.id(), "expr permits at most one owned ValueSpecification")));
    }
    let expr_present = require_all([expr_contract, accept_any(presence)]);

    let empty = Value::Array(Vec::new());
    let observations = expression.properties().get("observation").unwrap_or(&empty);
    let mut observation_checks = vec![contract("observation", observations)];
    if let Some(values) = observations.as_array() {
        observation_checks.push(Ok(values.len() == 1));
        for value in values {
            if let Ok(Some(id)) = reference_id(value) {
                if let Ok(observation) = references.element(id) {
                    observation_checks.push(if observation.metaclass().is_none() {
                        Err(QueryError::unsupported(id, "observation metaclass is unresolved"))
                    } else { Ok(observation.conforms_to(observation_type)) });
                }
            }
        }
    }
    // expr <> null is the implication's exemption. A known true consequent
    // can likewise decide an unresolved expr, but malformed contracts never do.
    accept_any([expr_present, require_all(observation_checks)])
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
    for value in references.model().elements().filter(|element| element.conforms_to("InstanceValue")) {
        if let Err(error) = referenced_instance(references, value.id()) {
            issues.push(crate::StructuralDiagnostic {
                severity: if error.is_unsupported() { crate::StructuralSeverity::Warning } else { crate::StructuralSeverity::Error },
                code: if error.is_unsupported() { "MDE_INSTANCE_VALUE_REFERENCE_UNVERIFIED" } else { "UML_INSTANCE_VALUE_REFERENCE" },
                subject_id: value.id().into(),
                message: error.to_string(),
            });
        }
    }
    for element in references.model().elements().filter(|element| element.conforms_to("MultiplicityElement")) {
        if let Err(error) = bound_expression_guarantees(references, element) {
            for (rule, invalid_code, unknown_code) in [
                ("value_specification_constant", "MDE_MULTIPLICITY_CONSTANT_INVALID", "MDE_MULTIPLICITY_CONSTANT_UNVERIFIED"),
                ("value_specification_no_side_effects", "MDE_MULTIPLICITY_NO_SIDE_EFFECTS_INVALID", "MDE_MULTIPLICITY_NO_SIDE_EFFECTS_UNVERIFIED"),
            ] {
                issues.push(crate::StructuralDiagnostic {
                    severity: if error.is_unsupported() { crate::StructuralSeverity::Warning } else { crate::StructuralSeverity::Error },
                    code: if error.is_unsupported() { unknown_code } else { invalid_code },
                    subject_id: element.id().into(),
                    message: format!("MultiplicityElement-{rule}: {error}"),
                });
            }
        }
    }
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
    for expression in references.model().elements() {
        let (metaclass, expr_role, observation_type, error_code, unknown_code) = if expression.conforms_to("Duration") {
            ("Duration", "Duration-expr", "DurationObservation", "UML_DURATION_OBSERVATION_REQUIRED", "MDE_DURATION_OBSERVATION_REQUIRED_UNVERIFIED")
        } else if expression.conforms_to("TimeExpression") {
            ("TimeExpression", "TimeExpression-expr", "TimeObservation", "UML_TIME_EXPRESSION_OBSERVATION_REQUIRED", "MDE_TIME_EXPRESSION_OBSERVATION_REQUIRED_UNVERIFIED")
        } else { continue; };
        let (severity, code, message) = match temporal_expression_observation(references, expression, metaclass, expr_role, observation_type) {
            Ok(true) => continue,
            Ok(false) => (crate::StructuralSeverity::Error, error_code, format!("without expr, exactly one {observation_type} is required")),
            Err(super::classifiers::QueryError::Invalid { message, .. }) =>
                (crate::StructuralSeverity::Error, error_code, message),
            Err(super::classifiers::QueryError::Unsupported { message, .. }) =>
                (crate::StructuralSeverity::Warning, unknown_code, message),
        };
        issues.push(crate::StructuralDiagnostic { severity, code, subject_id: expression.id().into(), message: format!("{metaclass}-no_expr_requires_observation: {message}") });
    }
    for expression in references.model().elements().filter(|element| element.conforms_to("StringExpression")) {
        let (severity, code, message) = match string_expression_subexpressions(references, expression) {
            Ok(true) => continue,
            Ok(false) => (crate::StructuralSeverity::Error, "UML_STRING_EXPRESSION_SUBEXPRESSIONS", "exactly one of operand and subExpression must be nonempty".to_owned()),
            Err(super::classifiers::QueryError::Invalid { message, .. }) =>
                (crate::StructuralSeverity::Error, "UML_STRING_EXPRESSION_SUBEXPRESSIONS", message),
            Err(super::classifiers::QueryError::Unsupported { message, .. }) =>
                (crate::StructuralSeverity::Warning, "MDE_STRING_EXPRESSION_SUBEXPRESSIONS_UNVERIFIED", message),
        };
        issues.push(crate::StructuralDiagnostic { severity, code, subject_id: expression.id().into(), message: format!("StringExpression-subexpressions: {message}") });
    }
    issues
}
