// Contracts: UML 2.5.1 9.9.2, 9.9.11 and 9.9.13; provenance and bounded
// comparison with RUML mutation recipes: docs/bellatrix/references/RUML.md.

use super::{references::ReferenceIndex, root::ElementRef, schema};
use crate::{Element, ModelingSection, StructuralDiagnostic, StructuralSeverity};
use serde_json::Value;

/// UML 9.9.2/9.9.11: the existing ownership edge sequence is the ordered set.
pub fn owned_parameters<'a>(
    references: &ReferenceIndex<'a>,
    operation_id: &str,
) -> Result<Vec<&'a Element>, String> {
    if !references.element(operation_id)?.conforms_to("Operation") {
        return Err("parameter owner must be an Operation".into());
    }
    Ok(references
        .explicit_owned_elements(operation_id)?
        .into_iter()
        .filter_map(|element| match element {
            ElementRef::Object(parameter) if element.conforms_to("Parameter") => Some(parameter),
            _ => None,
        })
        .collect())
}

pub fn direction(parameter: &Element) -> Result<String, String> {
    let descriptor = schema::uml_property("Parameter", "direction")
        .map_err(|error| error.to_string())?
        .ok_or("Parameter::direction contract is missing")?;
    let value = parameter
        .properties
        .get("direction")
        .cloned()
        .or_else(|| {
            schema::effective_default(descriptor)
                .ok()
                .flatten()
                .and_then(|default| default.scalar_json())
        })
        .ok_or("Parameter::direction has no value or generated default")?;
    schema::validate_scalar(descriptor, &value)
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    value
        .as_str()
        .map(str::to_owned)
        .ok_or("invalid Parameter::direction".into())
}

pub fn return_parameter<'a>(
    references: &ReferenceIndex<'a>,
    operation_id: &str,
) -> Result<Option<&'a Element>, String> {
    let mut result = None;
    for parameter in owned_parameters(references, operation_id)? {
        if direction(parameter)? == "return" && result.replace(parameter).is_some() {
            return Err(
                "Operation::at_most_one_return permits at most one return Parameter".into(),
            );
        }
    }
    Ok(result)
}

pub fn return_type<'a>(
    references: &ReferenceIndex<'a>,
    operation_id: &str,
) -> Result<Option<ElementRef<'a>>, String> {
    let Some(parameter) = return_parameter(references, operation_id)? else {
        return Ok(None);
    };
    let descriptor = schema::uml_property("Parameter", "type")
        .map_err(|error| error.to_string())?
        .ok_or("Parameter::type contract is missing")?;
    references.validate(
        descriptor,
        parameter.properties.get("type").unwrap_or(&Value::Null),
    )
}

pub(crate) fn move_parameter(
    section: &mut ModelingSection,
    parameter_id: &str,
    up: bool,
) -> Result<(), String> {
    let model = super::root::Model::from_section(section);
    let references = ReferenceIndex::new(model);
    let parameter = references.element(parameter_id)?;
    if !parameter.conforms_to("Parameter") {
        return Err("element must be a Parameter".into());
    }
    let owner = model.owner(parameter)?.ok_or("Parameter has no owner")?;
    let parameters = owned_parameters(&references, owner.id())?;
    let position = parameters
        .iter()
        .position(|parameter| parameter.id == parameter_id)
        .ok_or("Parameter is not owned by this Operation")?;
    let adjacent = if up {
        position.checked_sub(1)
    } else {
        position.checked_add(1)
    }
    .and_then(|index| parameters.get(index));
    let Some(adjacent) = adjacent else {
        return Ok(());
    };
    let edge = |id: &str| {
        section
            .relations
            .iter()
            .position(|relation| {
                relation.kind == "owns" && relation.from == owner.id() && relation.to == id
            })
            .ok_or("Parameter ownership edge is missing")
    };
    let first = edge(parameter_id)?;
    let second = edge(&adjacent.id)?;
    section.relations.swap(first, second);
    Ok(())
}

pub fn validate(references: &ReferenceIndex<'_>) -> Vec<StructuralDiagnostic> {
    let mut issues = Vec::new();
    for element in references.model().elements() {
        if (element.conforms_to("Operation") && element.properties().contains_key("ownedParameter"))
            || (element.conforms_to("Parameter") && element.properties().contains_key("operation"))
        {
            issues.push(StructuralDiagnostic {
                severity: StructuralSeverity::Error,
                code: "UML_PARAMETER_OWNERSHIP_STORAGE",
                subject_id: element.id().into(),
                message: "ownedParameter and operation must be projected from canonical owns edges"
                    .into(),
            });
        }
        if element.conforms_to("Operation") {
            if let Err(message) = return_parameter(references, element.id()) {
                issues.push(StructuralDiagnostic {
                    severity: StructuralSeverity::Error,
                    code: "UML_OPERATION_PARAMETERS",
                    subject_id: element.id().into(),
                    message,
                });
            }
        } else if element.conforms_to("Parameter") {
            let ElementRef::Object(parameter) = element else {
                continue;
            };
            let problem = match direction(parameter) {
                Err(error) => Some(error),
                Ok(direction) => {
                    let effect = parameter.properties.get("effect").and_then(Value::as_str);
                    let is_exception = parameter
                        .properties
                        .get("isException")
                        .and_then(Value::as_bool)
                        == Some(true);
                    let is_stream = parameter
                        .properties
                        .get("isStream")
                        .and_then(Value::as_bool)
                        == Some(true);
                    if (effect == Some("delete") && !matches!(direction.as_str(), "in" | "inout"))
                        || (effect == Some("create")
                            && !matches!(direction.as_str(), "out" | "inout" | "return"))
                    {
                        Some(
                            "Parameter::in_and_out restricts create/delete effects by direction"
                                .into(),
                        )
                    } else if is_exception && matches!(direction.as_str(), "in" | "inout") {
                        Some("Parameter::not_exception prohibits input exception Parameters".into())
                    } else if is_exception && is_stream {
                        Some("Parameter::stream_and_exception prohibits both flags together".into())
                    } else if effect.is_some()
                        && parameter
                            .properties
                            .get("type")
                            .and_then(|value| super::references::reference_id(value).ok().flatten())
                            .and_then(|id| references.element(id).ok())
                            .is_some_and(|target| target.conforms_to("DataType"))
                    {
                        Some(
                            "Parameter::object_effect prohibits an effect on a DataType Parameter"
                                .into(),
                        )
                    } else {
                        None
                    }
                }
            };
            if let Some(message) = problem {
                issues.push(StructuralDiagnostic {
                    severity: StructuralSeverity::Error,
                    code: "UML_PARAMETER_CONSTRAINT",
                    subject_id: element.id().into(),
                    message,
                });
            }
        }
    }
    issues
}
