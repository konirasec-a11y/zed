// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2019-2026 Starion Group S.A.
// Return-property query patterns adapted from uml4net OperationExtensions.cs,
// 8ef13af2c68ae1994c8fa4f5b3e231dee7e6715d. Bellatrix derives from canonical
// ownership and reports unevaluated bounds rather than assuming their values.
// Provenance and license: docs/bellatrix/references/uml4net/.
// Contracts: UML 2.5.1 9.9.2, 9.9.11 and 9.9.13; provenance and bounded
// comparison with RUML mutation recipes: docs/bellatrix/references/RUML.md.

use super::{
    references::ReferenceIndex,
    root::ElementRef,
    schema,
    types::{
        Integer, MultiplicityError, MultiplicityRange, UnlimitedNatural, literal_multiplicity,
    },
};
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
    effective_scalar(ElementRef::Object(parameter), "direction")?
        .as_str()
        .map(str::to_owned)
        .ok_or("invalid Parameter::direction".into())
}

fn effective_scalar(element: ElementRef<'_>, key: &str) -> Result<Value, String> {
    let metaclass = element.metaclass().ok_or("UML metaclass is missing")?;
    let descriptor = schema::uml_property(metaclass, key)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("{metaclass}::{key} contract is missing"))?;
    let value = if let Some(value) = element.properties().get(key) {
        value.clone()
    } else {
        schema::effective_default(descriptor)
            .map_err(|error| error.to_string())?
            .and_then(|default| default.scalar_json())
            .ok_or_else(|| format!("{metaclass}::{key} has no value or generated default"))?
    };
    schema::validate_scalar(descriptor, &value)
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    Ok(value)
}

fn effective_boolean(element: ElementRef<'_>, key: &str) -> Result<bool, String> {
    effective_scalar(element, key)?
        .as_bool()
        .ok_or_else(|| format!("{key} requires a Boolean"))
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReturnProperties {
    pub is_ordered: bool,
    pub is_unique: bool,
    pub multiplicity: Result<Option<MultiplicityRange>, MultiplicityError>,
}

impl ReturnProperties {
    pub fn lower(&self) -> Result<Option<&Integer>, MultiplicityError> {
        self.multiplicity
            .as_ref()
            .map(|range| range.as_ref().map(MultiplicityRange::lower_bound))
            .map_err(|error| *error)
    }

    pub fn upper(&self) -> Result<Option<&UnlimitedNatural>, MultiplicityError> {
        self.multiplicity
            .as_ref()
            .map(|range| range.as_ref().map(MultiplicityRange::upper_bound))
            .map_err(|error| *error)
    }
}

/// UML Operation-isOrdered.1/isUnique.1/lower.1/upper.1: absence of a return
/// Parameter is different from a return Parameter whose multiplicity is absent.
pub fn return_properties(
    references: &ReferenceIndex<'_>,
    operation_id: &str,
) -> Result<ReturnProperties, String> {
    let Some(parameter) = return_parameter(references, operation_id)? else {
        return Ok(ReturnProperties {
            is_ordered: false,
            is_unique: true,
            multiplicity: Ok(None),
        });
    };
    Ok(ReturnProperties {
        is_ordered: effective_boolean(ElementRef::Object(parameter), "isOrdered")?,
        is_unique: effective_boolean(ElementRef::Object(parameter), "isUnique")?,
        multiplicity: literal_multiplicity(&parameter.properties).map(Some),
    })
}

fn validate_body_condition(
    references: &ReferenceIndex<'_>,
    operation: ElementRef<'_>,
) -> Result<(), String> {
    let Some(value) = operation
        .properties()
        .get("bodyCondition")
        .filter(|value| !value.is_null())
    else {
        return Ok(());
    };
    if !effective_boolean(operation, "isQuery")? {
        return Err(
            "Operation::only_body_for_query requires isQuery when bodyCondition is specified"
                .into(),
        );
    }
    let descriptor = schema::uml_property("Operation", "bodyCondition")
        .map_err(|error| error.to_string())?
        .ok_or("Operation::bodyCondition contract is missing")?;
    let constraint = references
        .validate(descriptor, value)?
        .ok_or("bodyCondition is missing")?;
    if references
        .model()
        .owner(constraint)?
        .is_none_or(|owner| owner.id() != operation.id())
    {
        return Err("Operation::bodyCondition must be a Constraint owned by this Operation".into());
    }
    Ok(())
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
            if let Err(message) = validate_body_condition(references, element) {
                issues.push(StructuralDiagnostic {
                    severity: StructuralSeverity::Error,
                    code: "UML_OPERATION_BODY_CONDITION",
                    subject_id: element.id().into(),
                    message,
                });
            } else if element
                .properties()
                .get("bodyCondition")
                .is_some_and(|value| !value.is_null())
            {
                issues.push(StructuralDiagnostic { severity: StructuralSeverity::Warning,
                    code: "MDE_OPERATION_CONDITION_UNSUPPORTED", subject_id: element.id().into(),
                    message: "Operation bodyCondition ownership and only_body_for_query are checked; its constraint expression is not evaluated".into() });
            }
        } else if element.conforms_to("Parameter") {
            let ElementRef::Object(parameter) = element else {
                continue;
            };
            if let Err(error) = literal_multiplicity(&parameter.properties) {
                let unsupported = error == MultiplicityError::UnsupportedValueSpecification;
                issues.push(StructuralDiagnostic {
                    severity: if unsupported {
                        StructuralSeverity::Warning
                    } else {
                        StructuralSeverity::Error
                    },
                    code: if unsupported {
                        "MDE_PARAMETER_MULTIPLICITY_UNSUPPORTED"
                    } else {
                        "UML_PARAMETER_MULTIPLICITY"
                    },
                    subject_id: parameter.id.clone(),
                    message: error.to_string(),
                });
            }
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
