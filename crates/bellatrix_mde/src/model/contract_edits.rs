use bellatrix_mde_contracts::{
    ruml,
    uml::{
        metamodel,
        schema::{self, Aggregation, DefaultValue, UnsupportedContract},
    },
};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

struct PendingContract {
    metaclass: &'static str,
    data: Map<String, Value>,
}

/// Converts disposable editing records at the boundary; only the returned typed
/// registry may be retained as semantic state. The caller separately retains
/// label, description, annotations, implementations and content in its sidecar.
pub fn registry_from_projection(
    elements: &[crate::Element],
    relations: &[crate::Relation],
) -> Result<ruml::Registry, String> {
    let mut pending = BTreeMap::new();
    let mut identities = BTreeSet::new();
    for element in elements {
        let metaclass = crate::uml::root::object_metaclass(&element.kind)
            .ok_or_else(|| format!("{}: unsupported object kind {}", element.id, element.kind))?;
        let mut record = pending_contract(metaclass, &element.id, &element.properties)?;
        if field_shape(metaclass, "name")?.is_some() {
            let stored_name = record.data.get("name");
            // An empty display name is the projection of None; retain None.
            // A different non-null display name is an explicit rename edit.
            let unchanged_empty =
                element.name.is_empty() && stored_name.is_none_or(|value| value.is_null());
            if !unchanged_empty
                && stored_name.and_then(Value::as_str) != Some(element.name.as_str())
            {
                record
                    .data
                    .insert("name".into(), Value::String(element.name.clone()));
            }
        }
        insert_pending(&mut pending, &mut identities, &element.id, record)?;
    }
    for relation in relations {
        if !identities.insert(relation.id.clone()) {
            return Err(format!("Duplicate projection identity {}", relation.id));
        }
        if relation.kind == "owns" || crate::contract_projection::is_annotation_link(&relation.kind)
        {
            continue;
        }
        let metaclass = crate::uml::root::relationship_metaclass(&relation.kind)
            .or_else(|| crate::uml::root::object_metaclass(&relation.kind))
            .ok_or_else(|| {
                format!(
                    "{}: unsupported relationship kind {}",
                    relation.id, relation.kind
                )
            })?;
        let properties = relation
            .properties
            .iter()
            .filter(|(key, _)| {
                !metamodel::conforms_to(metaclass, "Association")
                    || !crate::contract_projection::ASSOCIATION_DISPLAY_KEYS.contains(&key.as_str())
            })
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        pending.insert(
            relation.id.clone(),
            pending_contract(metaclass, &relation.id, &properties)?,
        );
    }
    let previous_composites = clear_projected_composites(&mut pending)?;
    let mut ownership_assignments = Vec::new();
    for relation in relations.iter().filter(|relation| relation.kind == "owns") {
        if relation.properties.keys().any(|key| key != "role") {
            return Err(format!(
                "{}: ownership projection contains fields other than role; preserve presentation metadata in its sidecar",
                relation.id
            ));
        }
        let owner = pending
            .get(&relation.from)
            .ok_or_else(|| format!("{}: missing owner {}", relation.id, relation.from))?;
        let role = match relation.properties.get("role") {
            Some(Value::String(role)) => role.clone(),
            Some(_) => return Err(format!("{}: ownership role must be a string", relation.id)),
            None => {
                let owner_ref = projection_ref(elements, relations, &relation.from)?;
                let child_ref = projection_ref(elements, relations, &relation.to)?;
                crate::uml::root::ownership_property_for_assignment(owner_ref, child_ref)
                    .map_err(|error| format!("{}: {error}", relation.id))?
                    .to_owned()
            }
        };
        if field_shape(owner.metaclass, &role)?.is_none() {
            return Err(format!(
                "{}: {} has no ownership field {role}",
                relation.id, owner.metaclass
            ));
        }
        let owner = pending
            .get_mut(&relation.from)
            .ok_or_else(|| format!("Missing owner {}", relation.from))?;
        add_reference(owner, &role, &relation.to)?;
        ownership_assignments.push((relation.from.as_str(), role, relation.to.as_str()));
    }
    for relation in relations.iter().filter(|relation| {
        relation.kind != "owns" && !crate::contract_projection::is_annotation_link(&relation.kind)
    }) {
        apply_endpoints(&mut pending, relation)?;
    }
    for (owner_id, role, child_id) in ownership_assignments {
        let owner_type = pending
            .get(owner_id)
            .ok_or_else(|| format!("Missing owner {owner_id}"))?
            .metaclass;
        let Some(property) = schema::uml_property(owner_type, &role)
            .map_err(|error| format!("{owner_type}.{role}: {error}"))?
        else {
            continue;
        };
        let child = pending
            .get_mut(child_id)
            .ok_or_else(|| format!("Missing owned element {child_id}"))?;
        for opposite in property
            .opposites
            .iter()
            .filter_map(|reference| reference.target)
            .filter_map(schema::property_by_id)
            .filter(|opposite| !opposite.is_derived && !opposite.is_read_only)
        {
            // New members need both declared association ends. Retain authored
            // null/empty fields on existing contracts rather than repairing them.
            if !child.data.contains_key(opposite.name)
                && schema::classifier(opposite.owner).is_some_and(|declaring| {
                    metamodel::conforms_to(child.metaclass, declaring.name)
                })
                && opposite
                    .property_type
                    .target
                    .and_then(schema::classifier)
                    .is_some_and(|required| metamodel::conforms_to(owner_type, required.name))
                && matches!(
                    field_shape(child.metaclass, opposite.name)?,
                    Some("s" | "?s" | "v")
                )
            {
                add_reference(child, opposite.name, owner_id)?;
            }
        }
    }
    restore_composite_order(&mut pending, previous_composites)?;
    let mut registry = ruml::Registry::new();
    for (id, mut record) in pending {
        for (field, shape) in fields(record.metaclass)? {
            if !record.data.contains_key(field) {
                let value = if shape == "v" {
                    Value::Array(Vec::new())
                } else if shape.starts_with('?') {
                    Value::Null
                } else {
                    contract_default(record.metaclass, field)?.ok_or_else(|| {
                        format!(
                            "{} {id}: required field {field} is absent and has no official default",
                            record.metaclass
                        )
                    })?
                };
                record.data.insert(
                    field.to_owned(),
                    normalize(shape, &value)
                        .map_err(|error| format!("{} {id}.{field}: {error}", record.metaclass))?,
                );
            }
        }
        let contract = serde_json::from_value::<ruml::Contract>(serde_json::json!({
            "type": record.metaclass, "data": record.data,
        }))
        .map_err(|error| format!("{} {id}: {error}", record.metaclass))?;
        registry.insert_contract(contract)?;
    }
    let (_, projected_relations) = crate::contract_projection::project_registry(&registry)?;
    let mut annotation_links: BTreeMap<_, _> = projected_relations
        .iter()
        .filter(|relation| crate::contract_projection::is_annotation_link(&relation.kind))
        .map(|relation| (relation.id.as_str(), relation))
        .collect();
    for relation in relations
        .iter()
        .filter(|relation| crate::contract_projection::is_annotation_link(&relation.kind))
    {
        if annotation_links.remove(relation.id.as_str()) != Some(relation) {
            return Err(format!(
                "{}: annotation link differs from its canonical annotatedElement/constrainedElement projection; edit the annotation's references and regenerate its link",
                relation.id
            ));
        }
    }
    if let Some(id) = annotation_links.keys().next() {
        return Err(format!(
            "{id}: missing annotation link projection; edit the annotation's references instead of deleting its derived link"
        ));
    }
    for relation in relations {
        for key in crate::contract_projection::ASSOCIATION_DISPLAY_KEYS {
            let Some(value) = relation.properties.get(key) else {
                continue;
            };
            let projected = projected_relations
                .iter()
                .find(|projected| projected.id == relation.id);
            if projected.and_then(|projected| projected.properties.get(key)) != Some(value) {
                return Err(format!(
                    "{}: {key} differs from its canonical Property/ValueSpecification projection; use the association end editing command",
                    relation.id
                ));
            }
        }
    }
    Ok(registry)
}

// Only clear slots for which project_registry emits owns edges. In particular,
// metadata from a different UML revision must not erase a raw contract value
// when its scalar/collection shape differs from the projection descriptor.
fn clear_projected_composites(
    pending: &mut BTreeMap<String, PendingContract>,
) -> Result<BTreeMap<(String, String), Vec<Value>>, String> {
    let mut previous = BTreeMap::new();
    for (id, record) in pending {
        for (field, shape) in fields(record.metaclass)? {
            let Some(descriptor) = schema::uml_property(record.metaclass, field).ok().flatten()
            else {
                continue;
            };
            // Inspect the referenced type independently of multiplicity: the
            // scalar adapter rejects collections before resolving their type.
            // Otherwise old composite members survive a move or deletion.
            let mut scalar = *descriptor;
            scalar.upper = "1";
            if descriptor.aggregation != Aggregation::Composite
                || !matches!(
                    schema::scalar_kind(&scalar),
                    Err(UnsupportedContract::Reference)
                )
                || (shape == "v") == matches!(descriptor.upper, "0" | "1")
            {
                continue;
            }
            match shape {
                "v" => {
                    if let Some(Value::Array(values)) = record
                        .data
                        .insert(field.to_owned(), Value::Array(Vec::new()))
                    {
                        previous.insert((id.clone(), field.to_owned()), values);
                    }
                }
                "?s" => {
                    record.data.insert(field.to_owned(), Value::Null);
                }
                "s" => {
                    record.data.remove(field);
                }
                _ => {}
            }
        }
    }
    Ok(previous)
}

fn restore_composite_order(
    pending: &mut BTreeMap<String, PendingContract>,
    previous: BTreeMap<(String, String), Vec<Value>>,
) -> Result<(), String> {
    for ((id, field), old_values) in previous {
        let record = pending
            .get_mut(&id)
            .ok_or_else(|| format!("Missing owner {id}"))?;
        let values = record
            .data
            .get_mut(&field)
            .and_then(Value::as_array_mut)
            .ok_or_else(|| format!("{id}.{field}: missing reconstructed composite collection"))?;
        // owns edges identify membership, not occurrence or order: retain both
        // for surviving entries, without reviving any removed owner reference.
        let mut ordered: Vec<Value> = old_values
            .into_iter()
            .filter(|value| values.contains(value))
            .collect();
        for value in values.iter() {
            if !ordered.contains(value) {
                ordered.push(value.clone());
            }
        }
        *values = ordered;
    }
    Ok(())
}

fn projection_ref<'a>(
    elements: &'a [crate::Element],
    relations: &'a [crate::Relation],
    id: &str,
) -> Result<crate::uml::root::ElementRef<'a>, String> {
    elements
        .iter()
        .find(|element| element.id == id)
        .map(crate::uml::root::ElementRef::Object)
        .or_else(|| {
            relations
                .iter()
                .find(|relation| relation.id == id && relation.kind != "owns")
                .map(crate::uml::root::ElementRef::Relationship)
        })
        .ok_or_else(|| format!("Missing projection record {id}"))
}

fn insert_pending(
    pending: &mut BTreeMap<String, PendingContract>,
    identities: &mut BTreeSet<String>,
    id: &str,
    record: PendingContract,
) -> Result<(), String> {
    if !identities.insert(id.to_owned()) {
        return Err(format!("Duplicate projection identity {id}"));
    }
    pending.insert(id.to_owned(), record);
    Ok(())
}

fn pending_contract(
    metaclass: &'static str,
    id: &str,
    properties: &BTreeMap<String, Value>,
) -> Result<PendingContract, String> {
    let mut data = Map::new();
    data.insert("id".into(), Value::String(id.to_owned()));
    for (key, value) in properties {
        let key = key.strip_suffix('_').unwrap_or(key);
        let shape = field_shape(metaclass, key)?.ok_or_else(|| format!("{metaclass} {id}: unknown contract property {key}; move non-semantic metadata to the presentation sidecar"))?;
        let value =
            normalize(shape, value).map_err(|error| format!("{metaclass} {id}.{key}: {error}"))?;
        if key == "id" {
            if value != Value::String(id.to_owned()) {
                return Err(format!("{metaclass} {id}: conflicting id property"));
            }
        } else if data.insert(key.to_owned(), value).is_some() {
            return Err(format!(
                "{metaclass} {id}: duplicate normalized property {key}"
            ));
        }
    }
    Ok(PendingContract { metaclass, data })
}

fn normalize(shape: &str, value: &Value) -> Result<Value, String> {
    if let Some(inner) = shape.strip_prefix('?') {
        return if value.is_null() {
            Ok(Value::Null)
        } else {
            normalize(inner, value)
        };
    }
    match shape {
        "v" => value
            .as_array()
            .ok_or_else(|| "expected a collection".to_owned())?
            .iter()
            .map(|value| normalize("s", value))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        "s" => match value {
            Value::String(_) => Ok(value.clone()),
            Value::Bool(value) => Ok(Value::String(value.to_string())),
            Value::Number(value) => Ok(Value::String(value.to_string())),
            Value::Object(object) if object.len() == 1 => object
                .get("$ref")
                .filter(|value| value.is_string())
                .cloned()
                .ok_or_else(|| "expected a string or a single $ref string".to_owned()),
            _ => Err("expected a string or a single $ref string".to_owned()),
        },
        "b" => match value {
            Value::Bool(_) => Ok(value.clone()),
            Value::String(value) => value
                .parse::<bool>()
                .map(Value::Bool)
                .map_err(|error| error.to_string()),
            _ => Err("expected a boolean".to_owned()),
        },
        "i" => {
            let integer = value
                .as_i64()
                .or_else(|| value.as_str().and_then(|value| value.parse::<i64>().ok()))
                .ok_or_else(|| "expected an i32".to_owned())?;
            i32::try_from(integer)
                .map(Value::from)
                .map_err(|error| error.to_string())
        }
        literals => {
            let text = value
                .as_str()
                .ok_or_else(|| "expected an enumeration literal".to_owned())?;
            literals
                .split('/')
                .find(|literal| literal.eq_ignore_ascii_case(text))
                .map(|literal| Value::String(literal.to_owned()))
                .ok_or_else(|| format!("unknown enumeration literal {text}; expected {literals}"))
        }
    }
}

fn contract_default(metaclass: &str, field: &str) -> Result<Option<Value>, String> {
    let descriptor = match schema::uml_property(metaclass, field) {
        Ok(Some(descriptor)) => descriptor,
        Ok(None) | Err(_) => return Ok(None),
    };
    let default = schema::effective_default(descriptor)
        .map_err(|error| format!("{metaclass}.{field}: {error}"))?;
    Ok(match default {
        Some(DefaultValue::Boolean(value)) => Some(Value::Bool(value)),
        Some(
            DefaultValue::Integer(value)
            | DefaultValue::UnlimitedNatural(value)
            | DefaultValue::Real(value)
            | DefaultValue::String(value),
        ) => Some(Value::String(value.to_owned())),
        Some(DefaultValue::Enumeration { name, .. }) => Some(Value::String(name.to_owned())),
        Some(DefaultValue::Unsupported(_)) | None => None,
    })
}

fn add_reference(record: &mut PendingContract, field: &str, target: &str) -> Result<(), String> {
    let shape = field_shape(record.metaclass, field)?
        .ok_or_else(|| format!("{} has no field {field}", record.metaclass))?;
    let value = Value::String(target.to_owned());
    if shape == "v" {
        let existing = record
            .data
            .entry(field.to_owned())
            .or_insert_with(|| Value::Array(Vec::new()));
        let values = existing.as_array_mut().ok_or_else(|| {
            format!(
                "{}.{field}: expected reference collection",
                record.metaclass
            )
        })?;
        if !values.contains(&value) {
            values.push(value);
        }
    } else if matches!(shape, "s" | "?s") {
        match record.data.get(field) {
            Some(existing) if !existing.is_null() && existing != &value => {
                return Err(format!(
                    "{}.{field}: conflicting reference to {target}",
                    record.metaclass
                ));
            }
            _ => {
                record.data.insert(field.to_owned(), value);
            }
        }
    } else {
        return Err(format!(
            "{}.{field}: is not a reference slot",
            record.metaclass
        ));
    }
    Ok(())
}

fn apply_endpoints(
    pending: &mut BTreeMap<String, PendingContract>,
    relation: &crate::Relation,
) -> Result<(), String> {
    let record = pending
        .get(&relation.id)
        .ok_or_else(|| format!("Missing relationship {}", relation.id))?;
    let metaclass = record.metaclass;
    let endpoints = crate::contract_projection::direct_endpoint_fields(metaclass);
    if let Some((source, target, opposite)) = endpoints {
        let new_source = !record.data.contains_key(source);
        // A projected source may be derived from an owner's slot. Preserve that
        // representation rather than rewriting a complete contract on roundtrip.
        let derived_source = record.data.get(source).is_some_and(|value| {
            value.is_null()
                || value.as_str() == Some("")
                || value.as_array().is_some_and(Vec::is_empty)
        });
        if (new_source || derived_source)
            && let Some(opposite) = opposite
        {
            let owner = pending.get_mut(&relation.from).ok_or_else(|| {
                format!("{}: missing source owner {}", relation.id, relation.from)
            })?;
            add_reference(owner, opposite, &relation.id)?;
        }
        if !derived_source || opposite.is_none() {
            let record = pending
                .get_mut(&relation.id)
                .ok_or_else(|| format!("Missing relationship {}", relation.id))?;
            require_endpoint(record, source, &relation.from)?;
        }
        let record = pending
            .get_mut(&relation.id)
            .ok_or_else(|| format!("Missing relationship {}", relation.id))?;
        require_endpoint(record, target, &relation.to)?;
        if new_source && source != "client" && metamodel::conforms_to(metaclass, "Dependency") {
            // Specialized ends still belong to the non-derived Dependency
            // client/supplier supersets when a new relationship is authored.
            add_reference(record, "client", &relation.from)?;
            add_reference(record, "supplier", &relation.to)?;
        }
        return Ok(());
    }
    // AssociationClass inherits Association::memberEnd (UML 2.5.1, 11.8.2).
    // Reconcile explicit relation edits here even though automatic projection
    // keeps AssociationClass as an object to preserve its classifier identity.
    if metamodel::conforms_to(metaclass, "Association") {
        let ends = record
            .data
            .get("memberEnd")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                format!(
                    "{}: association endpoints require explicit memberEnd contracts",
                    relation.id
                )
            })?;
        let [source, target] = ends.as_slice() else {
            return Err(format!(
                "{}: binary projection requires exactly two memberEnd contracts",
                relation.id
            ));
        };
        let source = source
            .as_str()
            .ok_or_else(|| "Invalid association memberEnd".to_owned())?
            .to_owned();
        let target = target
            .as_str()
            .ok_or_else(|| "Invalid association memberEnd".to_owned())?
            .to_owned();
        for (end, endpoint) in [(&source, &relation.from), (&target, &relation.to)] {
            let property = pending
                .get_mut(end)
                .ok_or_else(|| format!("{}: missing association end {end}", relation.id))?;
            require_endpoint(property, "type", endpoint)?;
        }
        return Ok(());
    }
    Err(format!(
        "{}: no binary endpoint representation for {metaclass}; use an object projection to preserve its declared fields",
        relation.id
    ))
}

fn require_endpoint(record: &mut PendingContract, field: &str, target: &str) -> Result<(), String> {
    let shape = field_shape(record.metaclass, field)?
        .ok_or_else(|| format!("{} has no endpoint field {field}", record.metaclass))?;
    let expected = if shape == "v" {
        Value::Array(vec![Value::String(target.to_owned())])
    } else {
        Value::String(target.to_owned())
    };
    if let Some(existing) = record.data.get(field)
        && existing != &expected
    {
        return Err(format!(
            "{}.{field}: endpoint conflicts with declared contract data; edit the declared field and projection together",
            record.metaclass
        ));
    }
    record.data.insert(field.to_owned(), expected);
    Ok(())
}

pub(crate) fn field_shape(metaclass: &str, field: &str) -> Result<Option<&'static str>, String> {
    Ok(fields(metaclass)?
        .find(|(name, _)| *name == field)
        .map(|(_, shape)| shape))
}

fn fields(metaclass: &str) -> Result<impl Iterator<Item = (&'static str, &'static str)>, String> {
    Ok(contract_shape(metaclass)?
        .split_whitespace()
        .filter_map(|field| field.split_once(':')))
}

// Generated from every public field in ruml's concrete contract structs, with
// serde camelCase names and enum spellings from those same Rust sources. This
// is transport shape only, never an additional UML validator or semantic store.
// s=String, v=Vec<String>, b=bool, i=i32, ?=Option; other tokens list enum variants.
fn contract_shape(metaclass: &str) -> Result<&'static str, String> {
    CONTRACT_SHAPES
        .iter()
        .find(|(name, _)| *name == metaclass)
        .map(|(_, shape)| *shape)
        .ok_or_else(|| format!("No generated contract shape for {metaclass}"))
}

pub(crate) fn contract_type_for_kind(kind: &str) -> Option<&'static str> {
    let name = kind.strip_prefix("uml.")?;
    CONTRACT_SHAPES.iter().find_map(|(metaclass, _)| {
        let mut projected_name = String::new();
        let mut previous_lowercase = false;
        for character in metaclass.chars() {
            if character.is_uppercase() && previous_lowercase {
                projected_name.push('_');
            }
            projected_name.extend(character.to_lowercase());
            previous_lowercase = character.is_lowercase() || character.is_numeric();
        }
        (projected_name == name).then_some(*metaclass)
    })
}

const CONTRACT_SHAPES: &[(&str, &str)] = &[
    (
        "Abstraction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s supplier:v client:v mapping:?s",
    ),
    (
        "AcceptCallAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v isUnmarshall:s result:v trigger:v returnInformation:s",
    ),
    (
        "AcceptEventAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v isUnmarshall:s result:v trigger:v",
    ),
    (
        "Action",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v",
    ),
    (
        "ActionExecutionSpecification",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s covered:v generalOrdering:v enclosingInteraction:?s enclosingOperand:?s start:s finish:s action:s",
    ),
    (
        "ActionInputPin",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v type:?s ordering:Unordered/Ordered/Lifo/Fifo isControlType:s upperBound:s inState:v selection:?s isOrdered:s isUnique:s upperValue:?s lowerValue:?s isControl:s fromAction:s",
    ),
    (
        "Activity",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedConnector:v ownedBehavior:v classifierBehavior:?s interfaceRealization:v ownedTrigger:v nestedClassifier:v ownedOperation:v isActive:s ownedReception:v isReentrant:s redefinedBehavior:v ownedParameter:v precondition:v postcondition:v ownedParameterSet:v specification:?s variable:v node:v isReadOnly:s edge:v partition:v isSingleExecution:s group:v",
    ),
    (
        "ActivityEdge",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s source:s target:s redefinedEdge:v inPartition:v guard:s weight:s interrupts:?s inStructuredNode:?s activity:?s",
    ),
    (
        "ActivityFinalNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v",
    ),
    (
        "ActivityGroup",
        "id:s eAnnotations:v ownedComment:v inActivity:?s",
    ),
    (
        "ActivityNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v",
    ),
    (
        "ActivityParameterNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v type:?s ordering:Unordered/Ordered/Lifo/Fifo isControlType:s upperBound:s inState:v selection:?s parameter:s",
    ),
    (
        "ActivityPartition",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s inActivity:?s isDimension:s isExternal:s node:v subpartition:v superPartition:?s represents:?s edge:v",
    ),
    (
        "Actor",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedBehavior:v classifierBehavior:?s interfaceRealization:v ownedTrigger:v",
    ),
    (
        "AddStructuralFeatureValueAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v structuralFeature:s object:s value:s isReplaceAll:s insertAt:?s",
    ),
    (
        "AddVariableValueAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v variable:s value:s isReplaceAll:s insertAt:?s",
    ),
    (
        "AnyReceiveEvent",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s",
    ),
    (
        "Artifact",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v fileName:?s nestedArtifact:v manifestation:v ownedOperation:v ownedAttribute:v",
    ),
    (
        "Association",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedEnd:v memberEnd:v isDerived:s navigableOwnedEnd:v",
    ),
    (
        "AssociationClass",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedConnector:v ownedBehavior:v classifierBehavior:?s interfaceRealization:v ownedTrigger:v nestedClassifier:v ownedOperation:v isActive:s ownedReception:v ownedEnd:v memberEnd:v isDerived:s navigableOwnedEnd:v",
    ),
    (
        "Behavior",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedConnector:v ownedBehavior:v classifierBehavior:?s interfaceRealization:v ownedTrigger:v nestedClassifier:v ownedOperation:v isActive:s ownedReception:v isReentrant:s redefinedBehavior:v ownedParameter:v precondition:v postcondition:v ownedParameterSet:v specification:?s",
    ),
    (
        "BehaviorExecutionSpecification",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s covered:v generalOrdering:v enclosingInteraction:?s enclosingOperand:?s start:s finish:s behavior:?s",
    ),
    (
        "BehavioralFeature",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s isStatic:s ownedParameter:v isAbstract:s method:v concurrency:Sequential/Guarded/Concurrent raisedException:v ownedParameterSet:v",
    ),
    (
        "BehavioredClassifier",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedBehavior:v classifierBehavior:?s interfaceRealization:v ownedTrigger:v",
    ),
    (
        "BroadcastSignalAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v argument:v onPort:?s signal:s",
    ),
    (
        "CallAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v argument:v onPort:?s isSynchronous:s result:v",
    ),
    (
        "CallBehaviorAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v argument:v onPort:?s isSynchronous:s result:v behavior:s",
    ),
    (
        "CallEvent",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s operation:s",
    ),
    (
        "CallOperationAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v argument:v onPort:?s isSynchronous:s result:v operation:s target:s",
    ),
    (
        "CentralBufferNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v type:?s ordering:Unordered/Ordered/Lifo/Fifo isControlType:s upperBound:s inState:v selection:?s",
    ),
    (
        "ChangeEvent",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s changeExpression:s",
    ),
    (
        "Class",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedConnector:v ownedBehavior:v classifierBehavior:?s interfaceRealization:v ownedTrigger:v nestedClassifier:v ownedOperation:v isActive:s ownedReception:v",
    ),
    (
        "Classifier",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v",
    ),
    (
        "ClassifierTemplateParameter",
        "id:s eAnnotations:v ownedComment:v signature:s parameteredElement:s ownedParameteredElement:?s default:?s ownedDefault:?s allowSubstitutable:s defaultClassifier:?s constrainingClassifier:?s",
    ),
    (
        "Clause",
        "id:s eAnnotations:v ownedComment:v test:v body:v predecessorClause:v successorClause:v decider:s bodyOutput:v",
    ),
    (
        "ClearAssociationAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v object:s association:s",
    ),
    (
        "ClearStructuralFeatureAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v structuralFeature:s object:s",
    ),
    (
        "ClearVariableAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v variable:s",
    ),
    (
        "Collaboration",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedBehavior:v classifierBehavior:?s interfaceRealization:v ownedTrigger:v ownedAttribute:v ownedConnector:v collaborationRole:v",
    ),
    (
        "CollaborationUse",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s type:s roleBinding:v",
    ),
    (
        "CombinedFragment",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s covered:v generalOrdering:v enclosingInteraction:?s enclosingOperand:?s interactionOperator:Seq/Alt/Opt/Break/Par/Strict/Loop/Critical/Neg/Assert/Ignore/Consider operand:v cfragmentGate:v",
    ),
    (
        "Comment",
        "id:s eAnnotations:v ownedComment:v body:?s annotatedElement:v",
    ),
    (
        "CommunicationPath",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedEnd:v memberEnd:v isDerived:s navigableOwnedEnd:v",
    ),
    (
        "Component",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedConnector:v ownedBehavior:v classifierBehavior:?s interfaceRealization:v ownedTrigger:v nestedClassifier:v ownedOperation:v isActive:s ownedReception:v isIndirectlyInstantiated:s packagedElement:v realization:v",
    ),
    (
        "ComponentRealization",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s supplier:v client:v mapping:?s abstraction:?s realizingClassifier:s",
    ),
    (
        "ConditionalNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v elementImport:v packageImport:v ownedRule:v inActivity:?s variable:v edge:v mustIsolate:s node:v isDeterminate:s isAssured:s clause:v result:v",
    ),
    (
        "ConnectableElement",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s type:?s owningTemplateParameter:?s templateParameter:?s",
    ),
    (
        "ConnectableElementTemplateParameter",
        "id:s eAnnotations:v ownedComment:v signature:s parameteredElement:s ownedParameteredElement:?s default:?s ownedDefault:?s",
    ),
    (
        "ConnectionPointReference",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s container:?s entry:v exit:v state:?s",
    ),
    (
        "Connector",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s isStatic:s type:?s redefinedConnector:v end:v kind:?Assembly/Delegation contract:v",
    ),
    (
        "ConnectorEnd",
        "id:s eAnnotations:v ownedComment:v isOrdered:s isUnique:s upperValue:?s lowerValue:?s partWithPort:?s role:s",
    ),
    (
        "ConsiderIgnoreFragment",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s covered:v generalOrdering:v enclosingInteraction:?s enclosingOperand:?s interactionOperator:Seq/Alt/Opt/Break/Par/Strict/Loop/Critical/Neg/Assert/Ignore/Consider operand:v cfragmentGate:v message:v",
    ),
    (
        "Constraint",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s constrainedElement:v specification:s context:?s",
    ),
    (
        "Continuation",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s covered:v generalOrdering:v enclosingInteraction:?s enclosingOperand:?s setting:s",
    ),
    (
        "ControlFlow",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s source:s target:s redefinedEdge:v inPartition:v guard:s weight:s interrupts:?s inStructuredNode:?s activity:?s",
    ),
    (
        "ControlNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v",
    ),
    (
        "CreateLinkAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v endData:v inputValue:v",
    ),
    (
        "CreateLinkObjectAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v endData:v inputValue:v result:s",
    ),
    (
        "CreateObjectAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v classifier:s result:s",
    ),
    (
        "CreationEvent",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s",
    ),
    (
        "DataStoreNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v type:?s ordering:Unordered/Ordered/Lifo/Fifo isControlType:s upperBound:s inState:v selection:?s",
    ),
    (
        "DataType",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedOperation:v",
    ),
    (
        "DecisionNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v decisionInput:?s",
    ),
    (
        "Dependency",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s supplier:v client:v",
    ),
    (
        "DeployedArtifact",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s",
    ),
    (
        "Deployment",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s supplier:v client:v deployedArtifact:v configuration:v location:s",
    ),
    (
        "DeploymentSpecification",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v fileName:?s nestedArtifact:v manifestation:v ownedOperation:v ownedAttribute:v deploymentLocation:?s executionLocation:?s deployment:?s",
    ),
    (
        "DeploymentTarget",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s deployment:v",
    ),
    (
        "DestroyLinkAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v endData:v inputValue:v",
    ),
    (
        "DestroyObjectAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v isDestroyLinks:s isDestroyOwnedObjects:s target:s",
    ),
    (
        "DestructionEvent",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s",
    ),
    (
        "Device",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedConnector:v ownedBehavior:v classifierBehavior:?s interfaceRealization:v ownedTrigger:v nestedClassifier:v ownedOperation:v isActive:s ownedReception:v deployment:v nestedNode:v",
    ),
    ("DirectedRelationship", "id:s eAnnotations:v ownedComment:v"),
    (
        "Duration",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s type:?s expr:?s observation:v",
    ),
    (
        "DurationConstraint",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s constrainedElement:v specification:s context:?s firstEvent:v",
    ),
    (
        "DurationInterval",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s type:?s min:s max:s",
    ),
    (
        "DurationObservation",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s event:v firstEvent:v",
    ),
    (
        "EAnnotation",
        "id:s eAnnotations:v source:?s contents:v references:v",
    ),
    (
        "EAttribute",
        "id:s eAnnotations:v name:?s ordered:?b unique:?b lowerBound:?i upperBound:?i eType:?s eGenericType:?s changeable:?b volatile:?b transient:?b defaultValueLiteral:?s unsettable:?b derived:?b iD:?b",
    ),
    (
        "EClass",
        "id:s eAnnotations:v name:?s instanceClassName:?s instanceTypeName:?s eTypeParameters:v abstract:?b interface:?b eSuperTypes:v eOperations:v eStructuralFeatures:v eGenericSuperTypes:v",
    ),
    (
        "EClassifier",
        "id:s eAnnotations:v name:?s instanceClassName:?s instanceTypeName:?s eTypeParameters:v",
    ),
    (
        "EDataType",
        "id:s eAnnotations:v name:?s instanceClassName:?s instanceTypeName:?s eTypeParameters:v serializable:?b",
    ),
    (
        "EEnum",
        "id:s eAnnotations:v name:?s instanceClassName:?s instanceTypeName:?s eTypeParameters:v serializable:?b eLiterals:v",
    ),
    (
        "EEnumLiteral",
        "id:s eAnnotations:v name:?s value:?i literal:?s",
    ),
    (
        "EGenericType",
        "id:s eUpperBound:?s eTypeArguments:v eLowerBound:?s eTypeParameter:?s eClassifier:?s",
    ),
    ("Element", "id:s eAnnotations:v ownedComment:v"),
    (
        "ElementImport",
        "id:s eAnnotations:v ownedComment:v visibility:Public/Private/Protected/Package alias:?s importedElement:s importingNamespace:s",
    ),
    ("EModelElement", "id:s eAnnotations:v"),
    ("ENamedElement", "id:s eAnnotations:v name:?s"),
    (
        "EncapsulatedClassifier",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedConnector:v",
    ),
    (
        "Enumeration",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedOperation:v ownedLiteral:v",
    ),
    (
        "EnumerationLiteral",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s deployment:v owningTemplateParameter:?s templateParameter:?s classifier:v slot:v specification:?s enumeration:?s",
    ),
    ("EObject", "id:s"),
    (
        "EOperation",
        "id:s eAnnotations:v name:?s ordered:?b unique:?b lowerBound:?i upperBound:?i eType:?s eGenericType:?s eTypeParameters:v eParameters:v eExceptions:v eGenericExceptions:v",
    ),
    (
        "EPackage",
        "id:s eAnnotations:v name:?s nsUri:?s nsPrefix:?s eClassifiers:v eSubpackages:v",
    ),
    (
        "EParameter",
        "id:s eAnnotations:v name:?s ordered:?b unique:?b lowerBound:?i upperBound:?i eType:?s eGenericType:?s",
    ),
    (
        "EReference",
        "id:s eAnnotations:v name:?s ordered:?b unique:?b lowerBound:?i upperBound:?i eType:?s eGenericType:?s changeable:?b volatile:?b transient:?b defaultValueLiteral:?s unsettable:?b derived:?b containment:?b resolveProxies:?b eOpposite:?s eKeys:v",
    ),
    (
        "EStructuralFeature",
        "id:s eAnnotations:v name:?s ordered:?b unique:?b lowerBound:?i upperBound:?i eType:?s eGenericType:?s changeable:?b volatile:?b transient:?b defaultValueLiteral:?s unsettable:?b derived:?b",
    ),
    ("ETypeParameter", "id:s eAnnotations:v name:?s eBounds:v"),
    (
        "ETypedElement",
        "id:s eAnnotations:v name:?s ordered:?b unique:?b lowerBound:?i upperBound:?i eType:?s eGenericType:?s",
    ),
    (
        "Event",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s",
    ),
    (
        "ExceptionHandler",
        "id:s eAnnotations:v ownedComment:v handlerBody:s exceptionInput:s exceptionType:v protectedNode:s",
    ),
    (
        "ExecutableNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v",
    ),
    (
        "ExecutionEnvironment",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedConnector:v ownedBehavior:v classifierBehavior:?s interfaceRealization:v ownedTrigger:v nestedClassifier:v ownedOperation:v isActive:s ownedReception:v deployment:v nestedNode:v",
    ),
    (
        "ExecutionEvent",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s",
    ),
    (
        "ExecutionOccurrenceSpecification",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s covered:v generalOrdering:v enclosingInteraction:?s enclosingOperand:?s toBefore:v event:s toAfter:v execution:s",
    ),
    (
        "ExecutionSpecification",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s covered:v generalOrdering:v enclosingInteraction:?s enclosingOperand:?s start:s finish:s",
    ),
    (
        "ExpansionNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v type:?s ordering:Unordered/Ordered/Lifo/Fifo isControlType:s upperBound:s inState:v selection:?s regionAsOutput:?s regionAsInput:?s",
    ),
    (
        "ExpansionRegion",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v elementImport:v packageImport:v ownedRule:v inActivity:?s variable:v edge:v mustIsolate:s node:v mode:Parallel/Iterative/Stream inputElement:v outputElement:v",
    ),
    (
        "Expression",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s type:?s symbol:?s operand:v",
    ),
    (
        "Extend",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s extendedCase:s condition:?s extensionLocation:v extension:s",
    ),
    (
        "Extension",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedEnd:v memberEnd:v isDerived:s navigableOwnedEnd:v",
    ),
    (
        "ExtensionEnd",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s isStatic:s type:?s isOrdered:s isUnique:s upperValue:?s lowerValue:?s isReadOnly:s owningTemplateParameter:?s templateParameter:?s deployment:v templateBinding:v ownedTemplateSignature:?s datatype:?s isDerived:s isDerivedUnion:s aggregation:None/Shared/Composite redefinedProperty:v owningAssociation:?s defaultValue:?s subsettedProperty:v association:?s qualifier:v associationEnd:?s",
    ),
    (
        "ExtensionPoint",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s useCase:s",
    ),
    (
        "Feature",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s isStatic:s",
    ),
    (
        "FinalNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v",
    ),
    (
        "FinalState",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s container:?s submachine:?s connection:v connectionPoint:v redefinedState:?s stateInvariant:?s entry:?s exit:?s doActivity:?s deferrableTrigger:v region:v",
    ),
    (
        "FlowFinalNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v",
    ),
    (
        "ForkNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v",
    ),
    (
        "FunctionBehavior",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedConnector:v ownedBehavior:v classifierBehavior:?s interfaceRealization:v ownedTrigger:v nestedClassifier:v ownedOperation:v isActive:s ownedReception:v isReentrant:s redefinedBehavior:v ownedParameter:v precondition:v postcondition:v ownedParameterSet:v specification:?s body:v language:v",
    ),
    (
        "Gate",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s message:?s",
    ),
    (
        "GeneralOrdering",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s before:s after:s",
    ),
    (
        "Generalization",
        "id:s eAnnotations:v ownedComment:v isSubstitutable:?s general:s generalizationSet:v specific:s",
    ),
    (
        "GeneralizationSet",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s isCovering:s isDisjoint:s powertype:?s generalization:v",
    ),
    (
        "Image",
        "id:s eAnnotations:v ownedComment:v content:?s location:?s format:?s",
    ),
    (
        "Include",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s addition:s includingCase:s",
    ),
    (
        "InformationFlow",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s realization:v conveyed:v informationSource:v informationTarget:v realizingActivityEdge:v realizingConnector:v realizingMessage:v",
    ),
    (
        "InformationItem",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v represented:v",
    ),
    (
        "InitialNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v",
    ),
    (
        "InputPin",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v type:?s ordering:Unordered/Ordered/Lifo/Fifo isControlType:s upperBound:s inState:v selection:?s isOrdered:s isUnique:s upperValue:?s lowerValue:?s isControl:s",
    ),
    (
        "InstanceSpecification",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s deployment:v owningTemplateParameter:?s templateParameter:?s classifier:v slot:v specification:?s",
    ),
    (
        "InstanceValue",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s type:?s instance:s",
    ),
    (
        "Interaction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedConnector:v ownedBehavior:v classifierBehavior:?s interfaceRealization:v ownedTrigger:v nestedClassifier:v ownedOperation:v isActive:s ownedReception:v isReentrant:s redefinedBehavior:v ownedParameter:v precondition:v postcondition:v ownedParameterSet:v specification:?s covered:v generalOrdering:v enclosingInteraction:?s enclosingOperand:?s lifeline:v fragment:v action:v formalGate:v message:v",
    ),
    (
        "InteractionConstraint",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s constrainedElement:v specification:s context:?s minint:?s maxint:?s",
    ),
    (
        "InteractionFragment",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s covered:v generalOrdering:v enclosingInteraction:?s enclosingOperand:?s",
    ),
    (
        "InteractionOperand",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v covered:v generalOrdering:v enclosingInteraction:?s enclosingOperand:?s guard:?s fragment:v",
    ),
    (
        "InteractionUse",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s covered:v generalOrdering:v enclosingInteraction:?s enclosingOperand:?s refersTo:s actualGate:v argument:v",
    ),
    (
        "Interface",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedOperation:v nestedClassifier:v redefinedInterface:v ownedReception:v protocol:?s",
    ),
    (
        "InterfaceRealization",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s supplier:v client:v mapping:?s contract:s implementingClassifier:s",
    ),
    (
        "InterruptibleActivityRegion",
        "id:s eAnnotations:v ownedComment:v inActivity:?s node:v interruptingEdge:v",
    ),
    (
        "Interval",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s type:?s min:s max:s",
    ),
    (
        "IntervalConstraint",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s constrainedElement:v specification:s context:?s",
    ),
    (
        "InvocationAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v argument:v onPort:?s",
    ),
    (
        "JoinNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v isCombineDuplicate:s joinSpec:s",
    ),
    (
        "Lifeline",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s represents:?s interaction:s selector:?s decomposedAs:?s coveredBy:v",
    ),
    (
        "LinkAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v endData:v inputValue:v",
    ),
    (
        "LinkEndCreationData",
        "id:s eAnnotations:v ownedComment:v value:?s end:s qualifier:v isReplaceAll:s insertAt:?s",
    ),
    (
        "LinkEndData",
        "id:s eAnnotations:v ownedComment:v value:?s end:s qualifier:v",
    ),
    (
        "LinkEndDestructionData",
        "id:s eAnnotations:v ownedComment:v value:?s end:s qualifier:v isDestroyDuplicates:s destroyAt:?s",
    ),
    (
        "LiteralBoolean",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s type:?s value:s",
    ),
    (
        "LiteralInteger",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s type:?s value:s",
    ),
    (
        "LiteralNull",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s type:?s",
    ),
    (
        "LiteralSpecification",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s type:?s",
    ),
    (
        "LiteralString",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s type:?s value:?s",
    ),
    (
        "LiteralUnlimitedNatural",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s type:?s value:s",
    ),
    (
        "LoopNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v elementImport:v packageImport:v ownedRule:v inActivity:?s variable:v edge:v mustIsolate:s node:v isTestedFirst:s bodyPart:v setupPart:v decider:s test:v result:v loopVariable:v bodyOutput:v loopVariableInput:v",
    ),
    (
        "Manifestation",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s supplier:v client:v mapping:?s utilizedElement:s",
    ),
    (
        "MergeNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v",
    ),
    (
        "Message",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s messageSort:Synchcall/Asynchcall/Asynchsignal/Createmessage/Deletemessage/Reply receiveEvent:?s sendEvent:?s connector:?s interaction:s argument:v",
    ),
    (
        "MessageEnd",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s message:?s",
    ),
    (
        "MessageEvent",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s",
    ),
    (
        "MessageOccurrenceSpecification",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s covered:v generalOrdering:v enclosingInteraction:?s enclosingOperand:?s toBefore:v event:s toAfter:v message:?s",
    ),
    (
        "Model",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s packageMerge:v packagedElement:v profileApplication:v viewpoint:?s",
    ),
    (
        "MultiplicityElement",
        "id:s eAnnotations:v ownedComment:v isOrdered:s isUnique:s upperValue:?s lowerValue:?s",
    ),
    (
        "NamedElement",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s",
    ),
    (
        "Namespace",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v",
    ),
    (
        "Node",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedConnector:v ownedBehavior:v classifierBehavior:?s interfaceRealization:v ownedTrigger:v nestedClassifier:v ownedOperation:v isActive:s ownedReception:v deployment:v nestedNode:v",
    ),
    (
        "ObjectFlow",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s source:s target:s redefinedEdge:v inPartition:v guard:s weight:s interrupts:?s inStructuredNode:?s activity:?s isMulticast:s isMultireceive:s transformation:?s selection:?s",
    ),
    (
        "ObjectNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v type:?s ordering:Unordered/Ordered/Lifo/Fifo isControlType:s upperBound:s inState:v selection:?s",
    ),
    (
        "Observation",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s",
    ),
    (
        "OccurrenceSpecification",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s covered:v generalOrdering:v enclosingInteraction:?s enclosingOperand:?s toBefore:v event:s toAfter:v",
    ),
    (
        "OpaqueAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v body:v language:v inputValue:v outputValue:v",
    ),
    (
        "OpaqueBehavior",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedConnector:v ownedBehavior:v classifierBehavior:?s interfaceRealization:v ownedTrigger:v nestedClassifier:v ownedOperation:v isActive:s ownedReception:v isReentrant:s redefinedBehavior:v ownedParameter:v precondition:v postcondition:v ownedParameterSet:v specification:?s body:v language:v",
    ),
    (
        "OpaqueExpression",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s type:?s body:v language:v behavior:?s",
    ),
    (
        "Operation",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s isStatic:s ownedParameter:v isAbstract:s method:v concurrency:Sequential/Guarded/Concurrent raisedException:v ownedParameterSet:v owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s interface:?s class:?s isQuery:s precondition:v postcondition:v redefinedOperation:v datatype:?s bodyCondition:?s",
    ),
    (
        "OperationTemplateParameter",
        "id:s eAnnotations:v ownedComment:v signature:s parameteredElement:s ownedParameteredElement:?s default:?s ownedDefault:?s",
    ),
    (
        "OutputPin",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v type:?s ordering:Unordered/Ordered/Lifo/Fifo isControlType:s upperBound:s inState:v selection:?s isOrdered:s isUnique:s upperValue:?s lowerValue:?s isControl:s",
    ),
    (
        "Package",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s packageMerge:v packagedElement:v profileApplication:v",
    ),
    (
        "PackageImport",
        "id:s eAnnotations:v ownedComment:v visibility:Public/Private/Protected/Package importedPackage:s importingNamespace:s",
    ),
    (
        "PackageMerge",
        "id:s eAnnotations:v ownedComment:v mergedPackage:s receivingPackage:s",
    ),
    (
        "PackageableElement",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s",
    ),
    (
        "Parameter",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s type:?s owningTemplateParameter:?s templateParameter:?s isOrdered:s isUnique:s upperValue:?s lowerValue:?s parameterSet:v direction:In/Inout/Out/Return defaultValue:?s isException:s isStream:s effect:?Create/Read/Update/Delete",
    ),
    (
        "ParameterSet",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s parameter:v condition:v",
    ),
    (
        "ParameterableElement",
        "id:s eAnnotations:v ownedComment:v owningTemplateParameter:?s templateParameter:?s",
    ),
    (
        "PartDecomposition",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s covered:v generalOrdering:v enclosingInteraction:?s enclosingOperand:?s refersTo:s actualGate:v argument:v",
    ),
    (
        "Pin",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v type:?s ordering:Unordered/Ordered/Lifo/Fifo isControlType:s upperBound:s inState:v selection:?s isOrdered:s isUnique:s upperValue:?s lowerValue:?s isControl:s",
    ),
    (
        "Port",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s isStatic:s type:?s isOrdered:s isUnique:s upperValue:?s lowerValue:?s isReadOnly:s owningTemplateParameter:?s templateParameter:?s deployment:v templateBinding:v ownedTemplateSignature:?s datatype:?s isDerived:s isDerivedUnion:s aggregation:None/Shared/Composite redefinedProperty:v owningAssociation:?s defaultValue:?s subsettedProperty:v association:?s qualifier:v associationEnd:?s isBehavior:s isService:s redefinedPort:v protocol:?s",
    ),
    (
        "PrimitiveType",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedOperation:v",
    ),
    (
        "Profile",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s packageMerge:v packagedElement:v profileApplication:v metaclassReference:v metamodelReference:v",
    ),
    (
        "ProfileApplication",
        "id:s eAnnotations:v ownedComment:v appliedProfile:s isStrict:s applyingPackage:s",
    ),
    (
        "Property",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s isStatic:s type:?s isOrdered:s isUnique:s upperValue:?s lowerValue:?s isReadOnly:s owningTemplateParameter:?s templateParameter:?s deployment:v templateBinding:v ownedTemplateSignature:?s datatype:?s isDerived:s isDerivedUnion:s aggregation:None/Shared/Composite redefinedProperty:v owningAssociation:?s defaultValue:?s subsettedProperty:v association:?s qualifier:v associationEnd:?s",
    ),
    (
        "ProtocolConformance",
        "id:s eAnnotations:v ownedComment:v generalMachine:s specificMachine:s",
    ),
    (
        "ProtocolStateMachine",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedConnector:v ownedBehavior:v classifierBehavior:?s interfaceRealization:v ownedTrigger:v nestedClassifier:v ownedOperation:v isActive:s ownedReception:v isReentrant:s redefinedBehavior:v ownedParameter:v precondition:v postcondition:v ownedParameterSet:v specification:?s region:v submachineState:v connectionPoint:v extendedStateMachine:v conformance:v",
    ),
    (
        "ProtocolTransition",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s kind:Internal/Local/External container:s redefinedTransition:?s guard:?s effect:?s trigger:v target:s source:s postCondition:?s preCondition:?s",
    ),
    (
        "Pseudostate",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s container:?s kind:Initial/Deephistory/Shallowhistory/Join/Fork/Junction/Choice/Entrypoint/Exitpoint/Terminate stateMachine:?s state:?s",
    ),
    (
        "QualifierValue",
        "id:s eAnnotations:v ownedComment:v qualifier:s value:s",
    ),
    (
        "RaiseExceptionAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v exception:s",
    ),
    (
        "ReadExtentAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v result:s classifier:s",
    ),
    (
        "ReadIsClassifiedObjectAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v isDirect:s classifier:s result:s object:s",
    ),
    (
        "ReadLinkAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v endData:v inputValue:v result:s",
    ),
    (
        "ReadLinkObjectEndAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v object:s end:s result:s",
    ),
    (
        "ReadLinkObjectEndQualifierAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v object:s result:s qualifier:s",
    ),
    (
        "ReadSelfAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v result:s",
    ),
    (
        "ReadStructuralFeatureAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v structuralFeature:s object:s result:s",
    ),
    (
        "ReadVariableAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v variable:s result:s",
    ),
    (
        "Realization",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s supplier:v client:v mapping:?s",
    ),
    (
        "ReceiveOperationEvent",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s operation:s",
    ),
    (
        "ReceiveSignalEvent",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s signal:s",
    ),
    (
        "Reception",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s isStatic:s ownedParameter:v isAbstract:s method:v concurrency:Sequential/Guarded/Concurrent raisedException:v ownedParameterSet:v signal:?s",
    ),
    (
        "ReclassifyObjectAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v isReplaceAll:s oldClassifier:v newClassifier:v object:s",
    ),
    (
        "RedefinableElement",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s",
    ),
    (
        "RedefinableTemplateSignature",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s parameter:v ownedParameter:v template:s extendedSignature:v",
    ),
    (
        "ReduceAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v reducer:s result:s collection:s isOrdered:s",
    ),
    (
        "Region",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s subvertex:v transition:v state:?s extendedRegion:?s stateMachine:?s",
    ),
    ("Relationship", "id:s eAnnotations:v ownedComment:v"),
    (
        "RemoveStructuralFeatureValueAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v structuralFeature:s object:s value:s isRemoveDuplicates:s removeAt:?s",
    ),
    (
        "RemoveVariableValueAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v variable:s value:s isRemoveDuplicates:s removeAt:?s",
    ),
    (
        "ReplyAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v replyToCall:s returnInformation:s replyValue:v",
    ),
    (
        "SendObjectAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v argument:v onPort:?s target:s request:s",
    ),
    (
        "SendOperationEvent",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s operation:s",
    ),
    (
        "SendSignalAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v argument:v onPort:?s target:s signal:s",
    ),
    (
        "SendSignalEvent",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s signal:s",
    ),
    (
        "SequenceNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v elementImport:v packageImport:v ownedRule:v inActivity:?s variable:v edge:v mustIsolate:s node:v executableNode:v",
    ),
    (
        "Signal",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v",
    ),
    (
        "SignalEvent",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s signal:s",
    ),
    (
        "Slot",
        "id:s eAnnotations:v ownedComment:v definingFeature:s value:v owningInstance:s",
    ),
    (
        "StartClassifierBehaviorAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v object:s",
    ),
    (
        "State",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s container:?s submachine:?s connection:v connectionPoint:v redefinedState:?s stateInvariant:?s entry:?s exit:?s doActivity:?s deferrableTrigger:v region:v",
    ),
    (
        "StateInvariant",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s covered:v generalOrdering:v enclosingInteraction:?s enclosingOperand:?s invariant:s",
    ),
    (
        "StateMachine",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedConnector:v ownedBehavior:v classifierBehavior:?s interfaceRealization:v ownedTrigger:v nestedClassifier:v ownedOperation:v isActive:s ownedReception:v isReentrant:s redefinedBehavior:v ownedParameter:v precondition:v postcondition:v ownedParameterSet:v specification:?s region:v submachineState:v connectionPoint:v extendedStateMachine:v",
    ),
    (
        "Stereotype",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedConnector:v ownedBehavior:v classifierBehavior:?s interfaceRealization:v ownedTrigger:v nestedClassifier:v ownedOperation:v isActive:s ownedReception:v icon:v",
    ),
    (
        "StringExpression",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s type:?s symbol:?s operand:v templateBinding:v ownedTemplateSignature:?s subExpression:v owningExpression:?s",
    ),
    (
        "StructuralFeature",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s isStatic:s type:?s isOrdered:s isUnique:s upperValue:?s lowerValue:?s isReadOnly:s",
    ),
    (
        "StructuralFeatureAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v structuralFeature:s object:s",
    ),
    (
        "StructuredActivityNode",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v elementImport:v packageImport:v ownedRule:v inActivity:?s variable:v edge:v mustIsolate:s node:v",
    ),
    (
        "StructuredClassifier",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedAttribute:v ownedConnector:v",
    ),
    (
        "Substitution",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s supplier:v client:v mapping:?s contract:s substitutingClassifier:s",
    ),
    (
        "TemplateBinding",
        "id:s eAnnotations:v ownedComment:v signature:s parameterSubstitution:v boundElement:s",
    ),
    (
        "TemplateParameter",
        "id:s eAnnotations:v ownedComment:v signature:s parameteredElement:s ownedParameteredElement:?s default:?s ownedDefault:?s",
    ),
    (
        "TemplateParameterSubstitution",
        "id:s eAnnotations:v ownedComment:v formal:s actual:v ownedActual:v templateBinding:s",
    ),
    (
        "TemplateSignature",
        "id:s eAnnotations:v ownedComment:v parameter:v ownedParameter:v template:s",
    ),
    (
        "TemplateableElement",
        "id:s eAnnotations:v ownedComment:v templateBinding:v ownedTemplateSignature:?s",
    ),
    (
        "TestIdentityAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v first:s second:s result:s",
    ),
    (
        "TimeConstraint",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s constrainedElement:v specification:s context:?s firstEvent:?s",
    ),
    (
        "TimeEvent",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s isRelative:s when:s",
    ),
    (
        "TimeExpression",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s type:?s expr:?s observation:v",
    ),
    (
        "TimeInterval",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s type:?s min:s max:s",
    ),
    (
        "TimeObservation",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s event:s firstEvent:s",
    ),
    (
        "Transition",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s kind:Internal/Local/External container:s redefinedTransition:?s guard:?s effect:?s trigger:v target:s source:s",
    ),
    (
        "Trigger",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s event:s port:v",
    ),
    (
        "Type",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s",
    ),
    (
        "TypedElement",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s type:?s",
    ),
    (
        "UnmarshallAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v result:v unmarshallType:s object:s",
    ),
    (
        "Usage",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s supplier:v client:v",
    ),
    (
        "UseCase",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s elementImport:v packageImport:v ownedRule:v isLeaf:s owningTemplateParameter:?s templateParameter:?s templateBinding:v ownedTemplateSignature:?s isAbstract:s generalization:v powertypeExtent:v redefinedClassifier:v substitution:v representation:?s collaborationUse:v ownedUseCase:v useCase:v ownedBehavior:v classifierBehavior:?s interfaceRealization:v ownedTrigger:v include:v extend:v extensionPoint:v subject:v",
    ),
    (
        "ValuePin",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v type:?s ordering:Unordered/Ordered/Lifo/Fifo isControlType:s upperBound:s inState:v selection:?s isOrdered:s isUnique:s upperValue:?s lowerValue:?s isControl:s value:s",
    ),
    (
        "ValueSpecification",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s owningTemplateParameter:?s templateParameter:?s type:?s",
    ),
    (
        "ValueSpecificationAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v value:s result:s",
    ),
    (
        "Variable",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s type:?s owningTemplateParameter:?s templateParameter:?s isOrdered:s isUnique:s upperValue:?s lowerValue:?s scope:?s activityScope:?s",
    ),
    (
        "VariableAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v variable:s",
    ),
    (
        "Vertex",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s container:?s",
    ),
    (
        "WriteLinkAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v endData:v inputValue:v",
    ),
    (
        "WriteStructuralFeatureAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v structuralFeature:s object:s value:s",
    ),
    (
        "WriteVariableAction",
        "id:s eAnnotations:v ownedComment:v name:?s visibility:?Public/Private/Protected/Package clientDependency:v nameExpression:?s isLeaf:s inStructuredNode:?s activity:?s outgoing:v incoming:v inPartition:v inInterruptibleRegion:v redefinedNode:v handler:v localPrecondition:v localPostcondition:v variable:s value:s",
    ),
];
