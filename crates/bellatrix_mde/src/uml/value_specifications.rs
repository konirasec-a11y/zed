//! Pure, bounded UML ValueSpecification evaluation over one canonical graph.
//! OMG UML 2.5.1 §§7.8.8 and 8.6; generated defaults and scalar contracts remain
//! authoritative. Nonliteral evaluation is deliberately not inferred from text.

use super::{
    references::{ReferenceIndex, reference_id},
    root::ElementRef,
    schema,
    types::{
        Integer, MultiplicityError, MultiplicityRange, UnlimitedNatural, literal_multiplicity,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiteralValue {
    Integer(Integer),
    UnlimitedNatural(UnlimitedNatural),
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
        return Ok(LiteralValue::Null);
    }
    if !matches!(
        metaclass,
        "LiteralInteger" | "LiteralUnlimitedNatural" | "LiteralBoolean" | "LiteralString"
    ) {
        return Err(EvaluationError::unsupported(
            id,
            format!("{metaclass} evaluation is not implemented"),
        ));
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
    match metaclass {
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
        "LiteralBoolean" => value
            .as_bool()
            .map(LiteralValue::Boolean)
            .ok_or_else(|| EvaluationError::invalid(id, "Boolean value required")),
        "LiteralString" => Ok(LiteralValue::String(value.as_str().map(str::to_owned))),
        _ => Err(EvaluationError::unsupported(
            id,
            "literal query not implemented",
        )),
    }
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
    if value.get("href").is_some() {
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
        return literal_multiplicity(element.properties()).map_err(|error| {
            if error == MultiplicityError::UnsupportedValueSpecification {
                EvaluationError::unsupported(element.id(), error.to_string())
            } else {
                EvaluationError::invalid(element.id(), error.to_string())
            }
        });
    }
    let lower = bound(references, element, "lowerValue")?
        .map(|target| match evaluate(references, target.id())? {
            LiteralValue::Integer(value) => Ok(value),
            _ => Err(EvaluationError::invalid(
                target.id(),
                "MultiplicityElement::lower_is_integer requires a non-null integerValue()",
            )),
        })
        .transpose()?;
    let upper = bound(references, element, "upperValue")?.map(|target| {
        match evaluate(references, target.id())? {
            LiteralValue::UnlimitedNatural(value) => Ok(value),
            _ => Err(EvaluationError::invalid(target.id(), "MultiplicityElement::upper_is_unlimitedNatural requires a non-null unlimitedValue()")),
        }
    }).transpose()?;
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
