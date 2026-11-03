use crate::ModelingSection;
use crate::property_edit::{property_contract, PropertyValueType};
use serde_json::Value;

/// Only described, editable properties may change; identities cannot be edited via a JSON path.
pub fn set_inspected_property(
    section: &mut ModelingSection, id: &str, path: &str, value: Value
) -> Result<(), String> {
    let mut updated = section.clone();
    set_inspected_property_inner(&mut updated, id, path, value)?;
    crate::uml::associations::synchronize_changes(section, &mut updated)?;
    crate::uml::associations::validate_changes(section, &updated)?;
    *section = updated;
    Ok(())
}

fn set_inspected_property_inner(
    section: &mut ModelingSection, id: &str, path: &str, value: Value
) -> Result<(), String> {
    let references = crate::uml::references::ReferenceIndex::new(
        crate::uml::root::Model::from_section(section),
    );
    let field = property_contract(section, id, path)?;
    if field.value_type == PropertyValueType::ReadOnly { return Err("property is read-only".into()); }
    let optional_null = value.is_null() && path.strip_prefix("properties.").is_some_and(|key| {
        references.element(id).ok().is_some_and(|element| {
            element.metaclass()
                .and_then(|metaclass| crate::uml::schema::uml_property(metaclass, key).ok().flatten())
                .is_some_and(|descriptor| matches!(
                    crate::uml::property_contracts::check_value(&references, element, descriptor, &value),
                    crate::uml::property_contracts::PropertyOutcome::Satisfied,
                ))
        })
    });
    match field.value_type {
        _ if optional_null => {},
        PropertyValueType::ReadOnly => return Err("property is read-only".into()),
        PropertyValueType::String => {
            value.as_str().ok_or("expected string")?;
        },
        PropertyValueType::Boolean if !value.is_boolean() => return Err("expected boolean".into()),
        PropertyValueType::Number if !value.is_number() => return Err("expected number".into()),
        PropertyValueType::Choice(options) if !value.as_str()
            .is_some_and(|candidate| options.contains(&candidate)) => {
            return Err("invalid option".into());
        },
        _ => {},
    }
    let mut canonical_type = false;
    if let Some(key) = path.strip_prefix("properties.")
        && let Some(element) = crate::uml::root::Model::from_section(section).element(id)
        && let Some(metaclass) = element.metaclass() {
        crate::uml::schema::validate_edit(metaclass, key, &value)?;
        if key == "type"
            && let Some(descriptor) = crate::uml::schema::uml_property(metaclass, key)
                .map_err(|error| error.to_string())? {

            canonical_type = true;
            references.validate(descriptor, &value)?;
            if let Some(text) = element.properties().get("type").filter(|value| value.is_string())
                && let Some(preserved) = element.properties().get("typeText")
                && preserved != text {
                return Err("existing typeText differs from the legacy type declaration; preserve both before choosing a Type".into());
            }
        }
    }
    if crate::uml::associations::set_projection_property(section, id, path, &value)? {
        return Ok(());
    }
    let diagram_relation = section.diagrams.iter().any(|diagram|
        diagram.views.iter().any(|view| view.subject.as_deref() == Some(id)));
    if let Some(relation) = section.relations.iter_mut().find(|r|
        r.id == id && (crate::uml::root::relationship_metaclass(&r.kind).is_some() || r.kind.starts_with("uml.") || diagram_relation)) {
        let key=path.strip_prefix("properties.").ok_or("relationship identity is read-only")?;
        relation.properties.insert(key.into(),value);
        return Ok(());
    }
    let element = section.elements.iter_mut().find(|e| e.id == id).ok_or("element not found")?;
    match path {
        "name" => element.name = value.as_str().ok_or("expected string")?.to_owned(),
        "label" => {
            let s = value.as_str().ok_or("expected string")?;
            element.label = if s.is_empty() {None} else {Some(s.into())};
        },
        "description" => {
            let s = value.as_str().ok_or("expected string")?;
            element.description = if s.is_empty() {None} else {Some(s.into())};
        },
        _ => {
            let key = path.strip_prefix("properties.").ok_or("invalid property path")?;
            if canonical_type {
                if let Some(text) = element.properties.get("type")
                    .filter(|value| value.is_string()).cloned() {
                    element.properties.insert("typeText".into(), text);
                }
                if value.is_null() {
                    element.properties.remove(key);
                    return Ok(());
                }
            }
            element.properties.insert(key.to_owned(),value);
        },
    }
    Ok(())
}

