use crate::ModelingSection;
use crate::property_edit::{PropertyValueType, property_contract};
use serde_json::Value;

/// Only described, editable properties may change; identities cannot be edited via a JSON path.
pub fn set_inspected_property(
    section: &mut ModelingSection,
    id: &str,
    path: &str,
    value: Value,
) -> Result<(), String> {
    let mut updated = section.clone();
    set_inspected_property_inner(&mut updated, id, path, value)?;
    crate::uml::associations::synchronize_changes(section, &mut updated)?;
    *section = updated;
    Ok(())
}

fn set_inspected_property_inner(
    section: &mut ModelingSection,
    id: &str,
    path: &str,
    value: Value,
) -> Result<(), String> {
    let field = property_contract(section, id, path)?;
    if field.value_type == PropertyValueType::ReadOnly {
        return Err("property is read-only".into());
    }
    if path == "authoring.navigation" {
        let navigation = match value.as_str() {
            Some("navigable") => crate::NavigationKnowledge::Known(true),
            Some("notNavigable") => crate::NavigationKnowledge::Known(false),
            Some("unverified") => crate::NavigationKnowledge::Unverified,
            _ => return Err("expected navigable, notNavigable, or unverified".into()),
        };
        let references = crate::uml::references::ReferenceIndex::new(
            crate::uml::root::Model::from_section(section),
        );
        let association_id = crate::uml::associations::owning_association(&references, id)?
            .ok_or("navigation editing requires an Association-owned member end")?
            .id()
            .to_owned();
        return crate::uml::associations::set_navigation_knowledge(
            section,
            &association_id,
            id,
            navigation,
        );
    }
    // Optional absence is retained; requiredness is checked by contract conversion.
    let property_null = value.is_null()
        && path.strip_prefix("properties.").is_some_and(|key| {
            crate::uml::root::Model::from_section(section)
                .element(id)
                .and_then(|element| element.metaclass())
                .and_then(|metaclass| {
                    crate::uml::schema::uml_property(metaclass, key)
                        .ok()
                        .flatten()
                })
                .is_some_and(|descriptor| matches!(descriptor.upper, "0" | "1"))
        });
    match field.value_type {
        _ if property_null => {}
        PropertyValueType::ReadOnly => return Err("property is read-only".into()),
        PropertyValueType::String => {
            value.as_str().ok_or("expected string")?;
        }
        PropertyValueType::Boolean if !value.is_boolean() => return Err("expected boolean".into()),
        PropertyValueType::Number if !value.is_number() => return Err("expected number".into()),
        PropertyValueType::Choice(_) if !value.is_string() => return Err("expected string".into()),
        PropertyValueType::Stereotype => {
            if !matches!(
                crate::StereotypeValue::read(Some(&value)),
                crate::StereotypeValue::Null
                    | crate::StereotypeValue::Text(_)
                    | crate::StereotypeValue::Reference(_)
            ) {
                return Err("expected stereotype text, null, or a reference object".into());
            }
        }
        PropertyValueType::Reference => {
            crate::uml::references::reference_id(&value)?;
        }
        _ => {}
    }
    if crate::relationship_retarget::set_endpoint(section, id, path, &value)? {
        return Ok(());
    }
    if crate::uml::associations::set_projection_property(section, id, path, &value)? {
        return Ok(());
    }
    let diagram_relation = section.diagrams.iter().any(|diagram| {
        diagram
            .views
            .iter()
            .any(|view| view.subject.as_deref() == Some(id))
    });
    if let Some(relation) = section.relations.iter_mut().find(|r| {
        r.id == id
            && (crate::uml::root::relationship_metaclass(&r.kind).is_some()
                || r.kind.starts_with("uml.")
                || diagram_relation)
    }) {
        let key = path
            .strip_prefix("properties.")
            .ok_or("relationship identity is read-only")?;
        relation.properties.insert(key.into(), value);
        return Ok(());
    }
    let element = section
        .elements
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or("element not found")?;
    match path {
        "name" => element.name = value.as_str().ok_or("expected string")?.to_owned(),
        "label" => {
            let s = value.as_str().ok_or("expected string")?;
            element.label = if s.is_empty() { None } else { Some(s.into()) };
        }
        "description" => {
            let s = value.as_str().ok_or("expected string")?;
            element.description = if s.is_empty() { None } else { Some(s.into()) };
        }
        _ => {
            let key = path
                .strip_prefix("properties.")
                .ok_or("invalid property path")?;
            element.properties.insert(key.to_owned(), value);
        }
    }
    Ok(())
}
