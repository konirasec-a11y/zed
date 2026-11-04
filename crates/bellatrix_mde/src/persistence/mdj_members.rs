use super::*;

const NAMED_FIELDS: &[&str] = &["visibility", "stereotype"];
const INTERFACE_FIELDS: &[&str] = &[
    "visibility", "stereotype", "isAbstract", "isLeaf", "isFinalSpecialization",
];
const CLASS_FIELDS: &[&str] = &[
    "visibility", "stereotype", "isAbstract", "isActive", "isLeaf", "isFinalSpecialization",
];
const ATTRIBUTE_FIELDS: &[&str] = &[
    "visibility", "stereotype", "type", "multiplicity", "isStatic", "isLeaf", "isReadOnly",
    "isOrdered", "isUnique", "isDerived", "aggregation", "isID", "defaultValue",
];
const OPERATION_FIELDS: &[&str] = &[
    "visibility", "stereotype", "isStatic", "isLeaf", "isQuery", "isAbstract", "concurrency",
];
const PARAMETER_FIELDS: &[&str] = &[
    "visibility", "stereotype", "type", "multiplicity", "isOrdered", "isUnique", "direction", "defaultValue",
];

pub(super) fn fields(kind: &str) -> &'static [&'static str] {
    match kind {
        "UMLClass" => CLASS_FIELDS,
        "UMLInterface" => INTERFACE_FIELDS,
        "UMLAttribute" => ATTRIBUTE_FIELDS,
        "UMLOperation" => OPERATION_FIELDS,
        "UMLParameter" => PARAMETER_FIELDS,
        "UMLModel" | "UMLPackage" => NAMED_FIELDS,
        _ => &[],
    }
}

fn reference(value: &Value) -> Option<&str> {
    let object = value.as_object().filter(|object| object.len() == 1)?;
    object.get("$ref")?.as_str()
}

pub(super) fn supported_property(kind: &str, field: &str, value: &Value) -> bool {
    if !fields(kind).contains(&field) {
        return false;
    }
    match field {
        "stereotype" => matches!(crate::StereotypeValue::read(Some(value)),
            crate::StereotypeValue::Null | crate::StereotypeValue::Text(_) | crate::StereotypeValue::Reference(_)),
        // These are encoding checks. Enum values, bounds and reference targets
        // remain kernel inputs, including unresolved textual Type declarations.
        "visibility" | "multiplicity" | "aggregation" | "concurrency" | "direction" | "defaultValue" => value.is_string(),
        "type" => value.is_null() || value.is_string() || reference(value).is_some(),
        _ => value.is_boolean(),
    }
}

fn collections(kind: &str) -> &'static [(&'static str, &'static str)] {
    match kind {
        "UMLClass" | "UMLInterface" => &[("attributes", "UMLAttribute"), ("operations", "UMLOperation")],
        "UMLOperation" => &[("parameters", "UMLParameter")],
        _ => &[],
    }
}

pub(super) fn supported_field(kind: &str, field: &str, value: &Value) -> bool {
    if supported_property(kind, field, value) {
        return true;
    }
    if collections(kind).iter().any(|(collection, _)| *collection == field) {
        return value.is_array();
    }
    match (kind, field) {
        (_, "documentation") => value.is_string(),
        // An empty source-only specification does not assert a UML expression.
        ("UMLOperation", "specification") => value.as_str() == Some(""),
        ("UMLClass" | "UMLInterface", "receptions" | "behaviors")
        | ("UMLOperation", "raisedExceptions" | "preconditions" | "postconditions" | "bodyConditions") =>
            value.as_array().is_some_and(Vec::is_empty),
        _ => false,
    }
}

fn supported_object(object: &mdj::Object<'_>, index: &mdj::Index<'_>) -> bool {
    if object.type_name == "UMLAssociation" {
        // Check the contained association without climbing back to its owner.
        // The enclosing classifier's ancestry is checked by the caller.
        return associations::supported_structure(object, index);
    }
    canonical_kind(object.type_name).is_some()
        && !in_view_composition(&object.path)
        && object.properties.get("name").is_none_or(Value::is_string)
        && object.properties.iter().all(|(field, value)| supported_semantic_field(object.type_name, field, value))
        && supported_structure(object, index)
}

pub(super) fn supported_structure(object: &mdj::Object<'_>, index: &mdj::Index<'_>) -> bool {
    if let Some(slot) = match object.type_name {
        "UMLAttribute" => Some("attributes"),
        "UMLOperation" => Some("operations"),
        "UMLParameter" => Some("parameters"),
        _ => None,
    } {
        let Some(owner) = object.properties.get("_parent").and_then(reference).and_then(|id| index.object(id)) else {
            return false;
        };
        let supported_owner = if object.type_name == "UMLParameter" {
            owner.type_name == "UMLOperation"
        } else {
            matches!(owner.type_name, "UMLClass" | "UMLInterface")
        };
        if !supported_owner || !contained_in_slot(owner, slot, object) {
            return false;
        }
    }
    for (slot, expected_type) in collections(object.type_name) {
        let Some(value) = object.properties.get(*slot) else { continue };
        let Some(children) = value.as_array() else { return false };
        for child in children {
            let Some(child) = child.get("_id").and_then(Value::as_str).and_then(|id| index.object(id)) else {
                return false;
            };
            if child.type_name != *expected_type
                || !contained_in_slot(object, slot, child)
                || child.properties.get("_parent").and_then(reference) != Some(object.id)
                || !supported_object(child, index)
            {
                return false;
            }
        }
    }
    if matches!(object.type_name, "UMLClass" | "UMLInterface" | "UMLAttribute" | "UMLOperation" | "UMLParameter")
        && let Some(children) = object.properties.get("ownedElements").and_then(Value::as_array)
    {
        for child in children {
            let Some(child) = child.get("_id").and_then(Value::as_str).and_then(|id| index.object(id)) else {
                return false;
            };
            if !contained_in_slot(object, "ownedElements", child) {
                return false;
            }
            // A classifier must not become apparently complete by
            // hiding opaque owned members. Embedded diagrams are presentation.
            if is_diagram(child.type_name) {
                continue;
            }
            if !matches!(object.type_name, "UMLClass" | "UMLInterface")
                || child.properties.get("_parent").and_then(reference) != Some(object.id)
                || !supported_object(child, index)
            {
                return false;
            }
        }
    }
    if matches!(object.type_name, "UMLClass" | "UMLInterface" | "UMLAttribute" | "UMLOperation" | "UMLParameter") {
        // An explicit owner can disagree with physical containment. Such a
        // child must not disappear from the completeness decision merely
        // because it was stored elsewhere in the MDJ tree.
        for child in index.objects().filter(|child| {
            child.properties.get("_parent").and_then(reference) == Some(object.id)
        }) {
            if is_diagram(child.type_name)
                || (in_view_composition(&child.path) && child.type_name.ends_with("View"))
            {
                continue;
            }
            let in_members = collections(object.type_name).iter().any(|(slot, expected_type)| {
                child.type_name == *expected_type && contained_in_slot(object, slot, child)
            });
            let in_owned_elements = matches!(object.type_name, "UMLClass" | "UMLInterface")
                && contained_in_slot(object, "ownedElements", child);
            if !(in_members || in_owned_elements) {
                return false;
            }
        }
    }
    true
}

fn contained_in_slot(owner: &mdj::Object<'_>, slot: &str, child: &mdj::Object<'_>) -> bool {
    owner.properties.get(slot).and_then(Value::as_array).is_some_and(|children| {
        children.iter().enumerate().any(|(position, value)| {
            value.get("_id").and_then(Value::as_str) == Some(child.id)
                && child.path == format!("{}/{slot}/{position}", owner.path)
        })
    })
}

pub(super) fn import_properties(kind: &str, residue: &mut BTreeMap<String, Value>, mapped: &mut BTreeSet<String>) -> BTreeMap<String, Value> {
    let mut properties = BTreeMap::new();
    for field in fields(kind) {
        if let Some(value) = residue.remove(*field) {
            properties.insert((*field).into(), value);
            mapped.insert((*field).into());
        }
    }
    if matches!(kind, "UMLAttribute" | "UMLOperation" | "UMLParameter") {
        // NamedElement has no default visibility, unlike PackageableElement.
        properties.entry("visibility".into()).or_insert_with(|| Value::String("public".into()));
    }
    if matches!(kind, "UMLAttribute" | "UMLParameter") {
        properties.entry("multiplicity".into()).or_insert_with(|| Value::String(String::new()));
        properties.entry("isUnique".into()).or_insert(Value::Bool(false));
        properties.entry("type".into()).or_insert_with(|| Value::String(String::new()));
        // This is the source's textual editor declaration, not a fabricated
        // ValueSpecification. The existing kernel text adapter keeps it unverified.
        properties.entry("defaultValue".into()).or_insert_with(|| Value::String(String::new()));
    }
    properties
}

pub(super) fn preserves_omitted_default(kind: &str, field: &str, value: &Value) -> bool {
    if matches!(kind, "UMLAttribute" | "UMLOperation" | "UMLParameter") && field == "visibility" {
        return value.as_str() == Some("public");
    }
    matches!(kind, "UMLAttribute" | "UMLParameter") && match field {
        "isUnique" => value == &Value::Bool(false),
        "multiplicity" | "type" | "defaultValue" => value.as_str() == Some(""),
        _ => false,
    }
}

pub(super) fn update_metadata(element: &Element, metadata: &mut MdjObjectResidual) {
    if element.description.is_some() {
        metadata.mapped_fields.insert("documentation".into());
    } else {
        metadata.mapped_fields.remove("documentation");
    }
    for field in fields(&metadata.type_name) {
        if !metadata.mapped_fields.contains(*field)
            && element.properties.get(*field).is_some_and(|value| preserves_omitted_default(&metadata.type_name, field, value))
        {
            continue;
        }
        if element.properties.contains_key(*field) {
            metadata.mapped_fields.insert((*field).into());
        } else {
            metadata.mapped_fields.remove(*field);
        }
    }
}
