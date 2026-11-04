use super::*;

const END_FIELDS: [&str; 9] = [
    "visibility",
    "multiplicity",
    "aggregation",
    "isReadOnly",
    "isOrdered",
    "isUnique",
    "isDerived",
    "isID",
    "stereotype",
];
const ASSOCIATION_FIELDS: [&str; 3] = ["visibility", "isDerived", "stereotype"];

fn reference(value: Option<&Value>) -> Option<&str> {
    let object = value?.as_object()?;
    (object.len() == 1).then_some(())?;
    object.get("$ref")?.as_str()
}

fn supported(field: &str, value: &Value, end: bool) -> bool {
    match field {
        "_id" | "_type" | "name" => value.is_string(),
        "_parent" => reference(Some(value)).is_some(),
        "visibility" => value.is_string(),
        "stereotype" => matches!(
            crate::StereotypeValue::read(Some(value)),
            crate::StereotypeValue::Null
                | crate::StereotypeValue::Text(_)
                | crate::StereotypeValue::Reference(_)
        ),
        "isDerived" => value.is_boolean(),
        "end1" | "end2" if !end => value.is_object(),
        "reference" if end => reference(Some(value)).is_some(),
        "navigable" if end => matches!(value.as_str(), Some("navigable" | "notNavigable")),
        "aggregation" if end => value.as_str() == Some("none"),
        "multiplicity" if end => value.is_string(),
        "isReadOnly" | "isOrdered" | "isUnique" | "isID" if end => value.is_boolean(),
        _ => false,
    }
}

fn ends<'a, 'document>(
    object: &mdj::Object<'document>,
    index: &'a mdj::Index<'document>,
) -> Option<[&'a mdj::Object<'document>; 2]> {
    let first = object.properties.get("end1")?.get("_id")?.as_str()?;
    let second = object.properties.get("end2")?.get("_id")?.as_str()?;
    Some([index.object(first)?, index.object(second)?])
}

// This is an encoding boundary, not UML validation: dangling type references and
// invalid multiplicity strings stay available to the kernel unchanged.
fn representable(object: &mdj::Object<'_>, index: &mdj::Index<'_>) -> bool {
    if in_view_composition(&object.path)
        || object
            .properties
            .iter()
            .any(|(field, value)| !supported(field, value, false))
    {
        return false;
    }
    if let Some(parent) = object.properties.get("_parent") {
        let Some(parent) = reference(Some(parent)).and_then(|id| index.object(id)) else {
            return false;
        };
        if !(parent.path.is_empty() && parent.type_name == "Project")
            && canonical_kind_for_object(parent, index).is_none()
        {
            return false;
        }
    }
    let Some(ends) = ends(object, index) else {
        return false;
    };
    ends.iter().zip(["end1", "end2"]).all(|(end, slot)| {
        end.type_name == "UMLAssociationEnd"
            && end.path == format!("{}/{slot}", object.path)
            && reference(end.properties.get("_parent")) == Some(object.id)
            && reference(end.properties.get("reference")).is_some()
            && end
                .properties
                .get("multiplicity")
                .is_some_and(Value::is_string)
            && matches!(
                end.properties.get("navigable").and_then(Value::as_str),
                Some("navigable" | "notNavigable")
            )
            && end
                .properties
                .iter()
                .all(|(field, value)| supported(field, value, true))
    })
}

pub(super) fn import_associations(
    index: &mdj::Index<'_>,
    project: &mut Project,
    residuals: &mut Residuals,
    paths: &Paths,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeSet<String> {
    let mut mapped = BTreeSet::new();
    let mut associations: Vec<_> = index
        .objects()
        .filter(|object| object.type_name == "UMLAssociation")
        .collect();
    associations.sort_by(|left, right| path_order(&left.path).cmp(&path_order(&right.path)));
    for object in associations {
        if !representable(object, index) {
            let mut unsupported = object
                .properties
                .iter()
                .filter(|(field, value)| !supported(field, value, false))
                .map(|(field, _)| field.clone())
                .collect::<Vec<_>>();
            if let Some(ends) = ends(object, index) {
                for (position, end) in ends.iter().enumerate() {
                    for (field, value) in end.properties {
                        if !supported(field, value, true) {
                            unsupported.push(format!("end{}.{field}", position + 1));
                        }
                    }
                    for field in ["_parent", "reference", "navigable", "multiplicity"] {
                        if !end.properties.contains_key(field) {
                            unsupported.push(format!("end{}.{field} (absent)", position + 1));
                        }
                    }
                }
            }
            diagnostics.push(Diagnostic { path: object.path.clone(), message: format!("association and ends remain opaque: unmapped or absent fields [{}], or unrepresentable end/ownership structure. The supported encoding requires two association-owned ends, explicit navigable/notNavigable states, explicit multiplicity strings and aggregation absent/none; unspecified navigability, aggregation diamonds, ownerAttribute and qualifiers are not translated", unsupported.join(", ")) });
            continue;
        }
        let Some(ends) = ends(object, index) else {
            continue;
        };
        let mut residue = extract_residue(object.properties);
        let mut fields = BTreeSet::from(["_type".into()]);
        let name = take_name(&mut residue, &mut fields);
        let mut properties = BTreeMap::new();
        if fields.contains("name") {
            properties.insert("name".into(), Value::String(name));
        }
        for field in ASSOCIATION_FIELDS {
            if let Some(value) = residue.remove(field) {
                properties.insert(field.into(), value);
                fields.insert(field.into());
            }
        }
        properties.insert(
            "memberEnd".into(),
            serde_json::json!([{ "$ref": ends[0].id }, { "$ref": ends[1].id }]),
        );
        properties.insert(
            "navigableOwnedEnd".into(),
            Value::Array(
                ends.iter()
                    .filter(|end| {
                        end.properties.get("navigable").and_then(Value::as_str) == Some("navigable")
                    })
                    .map(|end| serde_json::json!({"$ref": end.id}))
                    .collect(),
            ),
        );
        if let Some(parent) = reference(residue.get("_parent"))
            .filter(|parent| {
                index
                    .object(parent)
                    .is_some_and(|parent| canonical_kind_for_object(parent, index).is_some())
            })
            .map(str::to_owned)
        {
            project.relations.push(Relation {
                id: ownership_relation_id(object.id, paths),
                kind: "owns".into(),
                from: parent,
                to: object.id.into(),
                properties: BTreeMap::new(),
            });
            residue.remove("_parent");
            fields.insert("_parent".into());
        }
        let mut relation = Relation {
            id: object.id.into(),
            kind: "uml.association".into(),
            from: String::new(),
            to: String::new(),
            properties,
        };
        for (position, end) in ends.into_iter().enumerate() {
            let mut residual = extract_residue(end.properties);
            let mut mapped_fields = BTreeSet::from([
                "_type".into(),
                "_parent".into(),
                "reference".into(),
                "navigable".into(),
            ]);
            let name = take_name(&mut residual, &mut mapped_fields);
            residual.remove("_parent");
            residual.remove("navigable");
            let mut properties = BTreeMap::new();
            if let Some(value) = residual.remove("reference") {
                properties.insert("type".into(), value);
            }
            for field in END_FIELDS {
                if let Some(value) = residual.remove(field) {
                    properties.insert(field.into(), value);
                    mapped_fields.insert(field.into());
                }
            }
            // MDJ defaults to false, UML to true. Presence metadata is encoding
            // only; the canonical false remains the sole semantic value.
            properties
                .entry("isUnique".into())
                .or_insert(Value::Bool(false));
            let element = Element {
                id: end.id.into(),
                kind: "uml.attribute".into(),
                name,
                label: None,
                description: None,
                properties,
                annotations: Vec::new(),
                implementations: Vec::new(),
                content: None,
            };
            project.relations.push(Relation {
                id: ownership_relation_id(end.id, paths),
                kind: "owns".into(),
                from: object.id.into(),
                to: end.id.into(),
                properties: BTreeMap::new(),
            });
            project_endpoint(&mut relation, &element, position);
            project.elements.push(element);
            residuals.insert(
                end.id.into(),
                MdjObjectResidual {
                    type_name: end.type_name.into(),
                    residual,
                    mapped_fields,
                },
            );
            mapped.insert(end.id.into());
        }
        project.relations.push(relation);
        residuals.insert(
            object.id.into(),
            MdjObjectResidual {
                type_name: object.type_name.into(),
                residual: residue,
                mapped_fields: fields,
            },
        );
        mapped.insert(object.id.into());
        diagnostics.push(Diagnostic { path: object.path.clone(), message: "binary association and Property ends mapped with original identities; relation endpoints, roles and multiplicities are derived projections, not independent editable semantics".into() });
    }
    mapped
}

fn project_endpoint(relation: &mut Relation, end: &Element, position: usize) {
    let target = reference(end.properties.get("type"))
        .unwrap_or_default()
        .to_owned();
    let (role, multiplicity) = if position == 0 {
        relation.from = target;
        ("sourceRole", "sourceMultiplicity")
    } else {
        relation.to = target;
        ("targetRole", "targetMultiplicity")
    };
    if !end.name.is_empty() {
        relation
            .properties
            .insert(role.into(), Value::String(end.name.clone()));
    }
    if let Some(value) = end.properties.get("multiplicity") {
        relation
            .properties
            .insert(multiplicity.into(), value.clone());
    }
}

pub(super) fn is_end(metadata: &MdjObjectResidual) -> bool {
    metadata.type_name == "UMLAssociationEnd" && metadata.mapped_fields.contains("_type")
}

pub(super) fn update_metadata(project: &Project, provenance: &mut MdjProvenance) {
    for end in &project.elements {
        if let Some(metadata) = provenance
            .object_residuals
            .get_mut(&end.id)
            .filter(|metadata| is_end(metadata))
        {
            for field in END_FIELDS {
                if field == "isUnique"
                    && !metadata.mapped_fields.contains(field)
                    && end.properties.get(field) == Some(&Value::Bool(false))
                {
                    continue;
                }
                if end.properties.contains_key(field) {
                    metadata.mapped_fields.insert(field.into());
                } else {
                    metadata.mapped_fields.remove(field);
                }
            }
        }
    }
    for relation in &project.relations {
        if relation.kind != "uml.association" {
            continue;
        }
        if let Some(metadata) = provenance.object_residuals.get_mut(&relation.id) {
            for field in ASSOCIATION_FIELDS.into_iter().chain(["name"]) {
                if relation.properties.contains_key(field) {
                    metadata.mapped_fields.insert(field.into());
                } else {
                    metadata.mapped_fields.remove(field);
                }
            }
        }
    }
}

pub(super) fn export_associations(
    project: &Project,
    objects: &mut BTreeMap<String, Value>,
) -> Result<(), MdjError> {
    let provenance = project
        .mdj_provenance
        .as_ref()
        .ok_or_else(|| failure("", "missing provenance"))?;
    for relation in project
        .relations
        .iter()
        .filter(|relation| relation.kind == "uml.association")
    {
        let metadata = provenance
            .object_residuals
            .get(&relation.id)
            .filter(|metadata| {
                metadata.type_name == "UMLAssociation" && metadata.mapped_fields.contains("_type")
            })
            .ok_or_else(|| failure("", "association has no source mapping"))?;
        let members = relation
            .properties
            .get("memberEnd")
            .and_then(Value::as_array)
            .filter(|members| members.len() == 2)
            .ok_or_else(|| failure("", "association memberEnd shape is not representable"))?;
        let navigable = relation
            .properties
            .get("navigableOwnedEnd")
            .and_then(Value::as_array)
            .ok_or_else(|| failure("", "missing association navigability"))?;
        let mut object = base_object(&relation.id, "UMLAssociation", &metadata.residual)?;
        for field in ASSOCIATION_FIELDS.into_iter().chain(["name"]) {
            if let Some(value) = relation.properties.get(field) {
                insert_mapped(&mut object, field, value.clone())?;
            }
        }
        export_owner(project, &relation.id, metadata, &mut object)?;
        let mut expected = Relation {
            id: relation.id.clone(),
            kind: relation.kind.clone(),
            from: String::new(),
            to: String::new(),
            properties: relation
                .properties
                .iter()
                .filter(|(field, _)| {
                    ASSOCIATION_FIELDS.contains(&field.as_str())
                        || matches!(field.as_str(), "name" | "memberEnd" | "navigableOwnedEnd")
                })
                .map(|(field, value)| (field.clone(), value.clone()))
                .collect(),
        };
        for (position, member) in members.iter().enumerate() {
            let id =
                reference(Some(member)).ok_or_else(|| failure("", "invalid memberEnd encoding"))?;
            let end = project
                .elements
                .iter()
                .find(|end| end.id == id && end.kind == "uml.attribute")
                .ok_or_else(|| failure("", "missing mapped Property end"))?;
            let end_metadata = provenance
                .object_residuals
                .get(id)
                .filter(|metadata| is_end(metadata))
                .ok_or_else(|| failure("", "missing end metadata"))?;
            let expected_path = format!(
                "{}/end{}",
                provenance
                    .object_paths
                    .get(&relation.id)
                    .ok_or_else(|| failure("", "missing association path"))?,
                position + 1
            );
            if provenance.object_paths.get(id) != Some(&expected_path) {
                return Err(failure(
                    "",
                    "reordering or replacing association ends is not yet supported",
                ));
            }
            let mut end_object = match named_object(
                id,
                "UMLAssociationEnd",
                &end.name,
                &end_metadata.residual,
                &end_metadata.mapped_fields,
            )? {
                Value::Object(object) => object,
                _ => return Err(failure("", "invalid end object")),
            };
            for (field, value) in &end.properties {
                if field == "type" {
                    insert_mapped(&mut end_object, "reference", value.clone())?;
                } else if END_FIELDS.contains(&field.as_str()) {
                    if field == "aggregation" && value.as_str() != Some("none") {
                        return Err(failure(
                            &expected_path,
                            "aggregation shared/composite cannot yet be exported with equivalent StarUML end notation; the edit must not be discarded or move the diamond silently",
                        ));
                    }
                    if field == "isUnique"
                        && !end_metadata.mapped_fields.contains(field)
                        && value == &Value::Bool(false)
                    {
                        continue;
                    }
                    insert_mapped(&mut end_object, field, value.clone())?;
                } else {
                    return Err(failure(
                        "",
                        "Property field has no lossless association-end mapping",
                    ));
                }
            }
            let is_navigable = navigable
                .iter()
                .any(|value| reference(Some(value)) == Some(id));
            insert_mapped(
                &mut end_object,
                "navigable",
                Value::String(
                    if is_navigable {
                        "navigable"
                    } else {
                        "notNavigable"
                    }
                    .into(),
                ),
            )?;
            export_owner(project, id, end_metadata, &mut end_object)?;
            project_endpoint(&mut expected, end, position);
            if objects
                .insert(id.into(), Value::Object(end_object))
                .is_some()
            {
                return Err(failure("", "duplicate association end identity"));
            }
        }
        if &expected != relation {
            return Err(failure(
                "",
                "association projection conflicts with authoritative Property ends",
            ));
        }
        if objects
            .insert(relation.id.clone(), Value::Object(object))
            .is_some()
        {
            return Err(failure("", "duplicate association identity"));
        }
    }
    Ok(())
}

fn export_owner(
    project: &Project,
    id: &str,
    metadata: &MdjObjectResidual,
    object: &mut Map<String, Value>,
) -> Result<(), MdjError> {
    if metadata.mapped_fields.contains("_parent") {
        let provenance = project
            .mdj_provenance
            .as_ref()
            .ok_or_else(|| failure("", "missing provenance"))?;
        let ownership_id = ownership_relation_id(id, &provenance.object_paths);
        let ownership = project
            .relations
            .iter()
            .find(|relation| {
                relation.id == ownership_id && relation.kind == "owns" && relation.to == id
            })
            .ok_or_else(|| failure("", "missing mapped association ownership"))?;
        insert_mapped(
            object,
            "_parent",
            serde_json::json!({"$ref": ownership.from}),
        )?;
    }
    Ok(())
}
