use bellatrix_mde_contracts::{
    ruml,
    uml::{
        metamodel,
        schema::{self, Aggregation, PropertyDescriptor, ScalarKind, UnsupportedContract},
        types::StandardPrimitiveType,
    },
};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

/// Builds disposable UI records from the typed registry. These records must never
/// be written back as semantic state: the registry is their only authority.
pub fn project_registry(
    registry: &ruml::Registry,
) -> Result<(Vec<crate::Element>, Vec<crate::Relation>), String> {
    let mut elements = Vec::new();
    let mut relations = Vec::new();
    let mut classifier_properties = BTreeSet::new();
    let mut ownership_ids = BTreeSet::new();

    for entry in registry.entries().values() {
        if metamodel::conforms_to(entry.contract.type_name(), "Classifier") {
            let data = entry.data()?;
            if let Some(Value::Array(attributes)) = data.get("ownedAttribute") {
                for attribute in attributes {
                    if let Some(id) = attribute.as_str() {
                        classifier_properties.insert(id.to_owned());
                    }
                }
            }
        }
    }

    for entry in registry.entries().values() {
        let metaclass = entry.contract.type_name();
        let id = entry.contract.id();
        let data = entry.data()?;
        let object = data
            .as_object()
            .ok_or_else(|| format!("{metaclass} {id}: contract data is not an object"))?;
        let mut properties = BTreeMap::new();
        for (key, value) in object {
            if key == "id" {
                continue;
            }
            // The generator's escaped Rust identifiers may retain a trailing '_'.
            let key = key.strip_suffix('_').unwrap_or(key);
            // Metadata from a different UML revision is only a display hint. It
            // must never reject a contract accepted by the canonical registry.
            let descriptor = schema::uml_property(metaclass, key).ok().flatten();
            let Some(descriptor) = descriptor else {
                properties.insert(key.to_owned(), value.clone());
                continue;
            };
            let projected = project_value(descriptor, value);
            if descriptor.aggregation == Aggregation::Composite {
                for target in reference_ids(&projected) {
                    // Length-prefixing makes this deterministic even when IDs contain separators.
                    let ownership_id = format!(
                        "ruml:owns:{}:{id}:{}:{key}:{}:{target}",
                        id.len(),
                        key.len(),
                        target.len()
                    );
                    if registry.get_entry(&ownership_id).is_some() {
                        return Err(format!(
                            "Projection ownership ID collides with contract {ownership_id}"
                        ));
                    }
                    if !ownership_ids.insert(ownership_id.clone()) {
                        continue;
                    }
                    relations.push(crate::Relation {
                        id: ownership_id,
                        kind: "owns".into(),
                        from: id.to_owned(),
                        to: target.to_owned(),
                        properties: BTreeMap::from([(
                            "role".into(),
                            Value::String(key.to_owned()),
                        )]),
                    });
                }
            }
            properties.insert(key.to_owned(), projected);
        }
        let name = object
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let mut kind = format!("uml.{}", snake_case(metaclass));
        if metaclass == "Comment" {
            kind = "uml.note".into();
        } else if metaclass == "Property" && classifier_properties.contains(id) {
            kind = "uml.attribute".into();
        }
        let association =
            metamodel::conforms_to(metaclass, "Association") && metaclass != "AssociationClass";
        if association
            && object
                .get("memberEnd")
                .and_then(Value::as_array)
                .is_some_and(|ends| ends.len() > 2)
        {
            kind = "uml.nary_association".into();
        } else if let Some((from, to)) = endpoints(registry, id, metaclass, object)
            .map_err(|error| format!("{metaclass} {id}: {error}"))?
        {
            relations.push(crate::Relation {
                id: id.to_owned(),
                kind,
                from,
                to,
                properties,
            });
            continue;
        }
        elements.push(crate::Element {
            id: id.to_owned(),
            kind,
            name,
            label: None,
            description: None,
            properties,
            annotations: Vec::new(),
            implementations: Vec::new(),
            content: None,
        });
    }
    Ok((elements, relations))
}

fn project_value(descriptor: &PropertyDescriptor, value: &Value) -> Value {
    if value.is_null() {
        return value.clone();
    }
    if !matches!(descriptor.upper, "0" | "1") {
        let Some(values) = value.as_array() else {
            return value.clone();
        };
        let mut scalar = *descriptor;
        scalar.upper = "1";
        return Value::Array(
            values
                .iter()
                .map(|value| project_value(&scalar, value))
                .collect(),
        );
    }
    let converted = match schema::scalar_kind(descriptor) {
        Ok(ScalarKind::Primitive(StandardPrimitiveType::Boolean)) => value
            .as_str()
            .and_then(|text| text.parse::<bool>().ok())
            .map(Value::Bool),
        Ok(ScalarKind::Primitive(
            StandardPrimitiveType::Integer | StandardPrimitiveType::UnlimitedNatural,
        )) => value.as_str().and_then(|text| {
            // Preserve arbitrary precision and '*' rather than round through f64.
            text.parse::<i64>()
                .ok()
                .map(Value::from)
                .or_else(|| text.parse::<u64>().ok().map(Value::from))
        }),
        Ok(ScalarKind::Primitive(StandardPrimitiveType::Real)) => value
            .as_str()
            .and_then(|text| text.parse::<serde_json::Number>().ok())
            .map(Value::Number),
        Ok(ScalarKind::Enumeration(literals)) => value
            .as_str()
            .and_then(|text| {
                literals
                    .iter()
                    .find(|literal| literal.eq_ignore_ascii_case(text))
            })
            .map(|literal| Value::String((*literal).to_owned())),
        Err(UnsupportedContract::Reference) => {
            value.as_str().map(|id| serde_json::json!({ "$ref": id }))
        }
        _ => None,
    };
    converted.unwrap_or_else(|| value.clone())
}

fn reference_ids(value: &Value) -> Vec<&str> {
    match value {
        Value::Array(values) => values
            .iter()
            .filter_map(|value| value.get("$ref").and_then(Value::as_str))
            .collect(),
        Value::Object(object) => object
            .get("$ref")
            .and_then(Value::as_str)
            .into_iter()
            .collect(),
        _ => Vec::new(),
    }
}

fn endpoint(object: &Map<String, Value>, key: &str) -> Option<String> {
    match object.get(key) {
        Some(Value::String(id)) if !id.is_empty() => Some(id.clone()),
        Some(Value::Array(values)) if values.len() == 1 => values
            .first()
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .map(str::to_owned),
        _ => None,
    }
}

fn opposite_endpoint(
    registry: &ruml::Registry,
    id: &str,
    slot: &str,
) -> Result<Option<String>, String> {
    let mut owners = BTreeSet::new();
    for entry in registry.entries().values() {
        let data = entry.data()?;
        let matches = match data.get(slot) {
            Some(Value::String(target)) => target == id,
            Some(Value::Array(targets)) => targets.iter().any(|target| target.as_str() == Some(id)),
            _ => false,
        };
        if matches {
            owners.insert(entry.contract.id().to_owned());
        }
    }
    // Ambiguity is not a registry error. Keep the contract as an element instead.
    Ok(if owners.len() == 1 {
        owners.into_iter().next()
    } else {
        None
    })
}

fn endpoints(
    registry: &ruml::Registry,
    id: &str,
    metaclass: &str,
    object: &Map<String, Value>,
) -> Result<Option<(String, String)>, String> {
    let fields = match metaclass {
        "Generalization" => Some(("specific", "general", Some("generalization"))),
        "Include" => Some(("includingCase", "addition", Some("include"))),
        "Extend" => Some(("extension", "extendedCase", Some("extend"))),
        "ElementImport" => Some((
            "importingNamespace",
            "importedElement",
            Some("elementImport"),
        )),
        "PackageImport" => Some((
            "importingNamespace",
            "importedPackage",
            Some("packageImport"),
        )),
        "PackageMerge" => Some(("receivingPackage", "mergedPackage", Some("packageMerge"))),
        "TemplateBinding" => Some(("boundElement", "signature", Some("templateBinding"))),
        "InterfaceRealization" => Some((
            "implementingClassifier",
            "contract",
            Some("interfaceRealization"),
        )),
        "Substitution" => Some(("substitutingClassifier", "contract", Some("substitution"))),
        "ProfileApplication" => Some((
            "applyingPackage",
            "appliedProfile",
            Some("profileApplication"),
        )),
        "AssociationClass" => return Ok(None),
        _ if metamodel::conforms_to(metaclass, "Dependency") => Some(("client", "supplier", None)),
        _ if metamodel::conforms_to(metaclass, "ActivityEdge")
            || metamodel::conforms_to(metaclass, "Transition") =>
        {
            Some(("source", "target", None))
        }
        _ => None,
    };
    if let Some((source, target, opposite)) = fields {
        let mut from = endpoint(object, source);
        let source_absent = match object.get(source) {
            None | Some(Value::Null) => true,
            Some(Value::String(value)) => value.is_empty(),
            Some(Value::Array(values)) => values.is_empty(),
            _ => false,
        };
        if source_absent && let Some(opposite) = opposite {
            from = opposite_endpoint(registry, id, opposite)?;
        }
        return Ok(from.zip(endpoint(object, target)));
    }
    if metamodel::conforms_to(metaclass, "Association") {
        let Some(ends) = object.get("memberEnd").and_then(Value::as_array) else {
            return Ok(None);
        };
        let [source, target] = ends.as_slice() else {
            return Ok(None);
        };
        let type_of_end = |value: &Value| -> Result<Option<String>, String> {
            let Some(entry) = value.as_str().and_then(|id| registry.get_entry(id)) else {
                return Ok(None);
            };
            let data = entry.data()?;
            Ok(data
                .get("type")
                .or_else(|| data.get("type_"))
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .map(str::to_owned))
        };
        return Ok(type_of_end(source)?.zip(type_of_end(target)?));
    }
    Ok(None)
}

fn snake_case(name: &str) -> String {
    let mut result = String::new();
    let mut previous_lowercase = false;
    for character in name.chars() {
        if character.is_uppercase() && previous_lowercase {
            result.push('_');
        }
        result.extend(character.to_lowercase());
        previous_lowercase = character.is_lowercase() || character.is_numeric();
    }
    result
}
