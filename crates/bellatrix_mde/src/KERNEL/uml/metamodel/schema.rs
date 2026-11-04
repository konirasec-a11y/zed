pub use bellatrix_mde_contracts::uml::schema::*;

#[cfg(test)]
use super::metamodel;
use super::root::ElementRef;
use crate::{StructuralDiagnostic, StructuralSeverity};
use serde_json::Value;

/// Read an authored scalar or its generated inherited default, applying the same
/// representation/type contract for every consumer. This does not evaluate UML rules.
pub fn read_effective_scalar(element: ElementRef<'_>, key: &str) -> Result<Value, String> {
    let metaclass = element.metaclass().ok_or("UML metaclass is missing")?;
    let descriptor = uml_property(metaclass, key)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("{metaclass}::{key} contract is missing"))?;
    let value = if let Some(value) = element.properties().get(key) {
        value.clone()
    } else {
        effective_default(descriptor)
            .map_err(|error| error.to_string())?
            .and_then(|default| default.scalar_json())
            .ok_or_else(|| format!("{metaclass}::{key} has no value or generated default"))?
    };
    validate_scalar(descriptor, &value)
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    Ok(value)
}

pub fn read_effective_boolean(element: ElementRef<'_>, key: &str) -> Result<bool, String> {
    read_effective_scalar(element, key)?
        .as_bool()
        .ok_or_else(|| format!("{key} requires a Boolean"))
}

pub fn validate_element(element: ElementRef<'_>) -> Vec<StructuralDiagnostic> {
    let Some(metaclass) = element.metaclass() else {
        return Vec::new();
    };
    element
        .properties()
        .iter()
        .filter_map(|(name, value)| {
            let descriptor = match uml_property(metaclass, name) {
                Ok(Some(descriptor)) => descriptor,
                _ => return None,
            };
            // References/collections and derived consistency are intentionally not scalar validation.
            let Err(error) = validate_scalar(descriptor, value).ok()? else {
                return None;
            };
            Some(StructuralDiagnostic {
                severity: StructuralSeverity::Error,
                code: "UML_PROPERTY_VALUE",
                subject_id: element.id().into(),
                message: format!("{metaclass}::{name}: {error}"),
            })
        })
        .collect()
}

/// These strings are editor declarations, not serialized UML Type/ValueSpecification identities.
pub fn uses_legacy_text_adapter(property: &PropertyDescriptor, value: &Value) -> bool {
    matches!(property.name, "type" | "defaultValue")
        && value.is_string()
        && matches!(scalar_kind(property), Err(UnsupportedContract::Reference))
}

pub fn validate_edit(metaclass: &str, name: &str, value: &Value) -> Result<(), String> {
    let Some(property) = uml_property(metaclass, name).map_err(|error| error.to_string())? else {
        return Ok(());
    };
    if property.is_read_only {
        return Err(format!(
            "{} is read-only in the UML metamodel",
            property.name
        ));
    }
    if property.is_derived {
        return Err(format!(
            "{}: {}",
            property.name,
            UnsupportedContract::Derivation
        ));
    }
    if property.name == "type" {
        super::references::reference_id(value)?;
        return Ok(());
    }
    if uses_legacy_text_adapter(property, value) {
        return Ok(());
    }
    match validate_scalar(property, value) {
        Ok(result) => result.map_err(|error| format!("{}: {error}", property.name)),
        // Other reference/collection adapters retain their own contract.
        Err(_) => Ok(()),
    }
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;
