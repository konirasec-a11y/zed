use super::*;

const END_FIELDS: [&str; 8] = [
    "visibility",
    "multiplicity",
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
        "visibility" => matches!(
            value.as_str(),
            Some("public" | "protected" | "private" | "package")
        ),
        "stereotype" => matches!(
            crate::StereotypeValue::read(Some(value)),
            crate::StereotypeValue::Null
                | crate::StereotypeValue::Text(_)
                | crate::StereotypeValue::Reference(_)
        ),
        "isDerived" => value.is_boolean(),
        "end1" | "end2" if !end => value.is_object(),
        "reference" if end => reference(Some(value)).is_some(),
        "navigable" if end => matches!(
            value.as_str(),
            Some("navigable" | "notNavigable" | "unspecified")
        ),
        "aggregation" if end => matches!(value.as_str(), Some("none" | "shared" | "composite")),
        "multiplicity" if end => value.as_str().is_some_and(|text| {
            text.is_empty()
                || text
                    .parse::<bellatrix_mde_contracts::uml::types::MultiplicityRange>()
                    .is_ok()
        }),
        "isReadOnly" | "isOrdered" | "isUnique" if end => value.is_boolean(),
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

// Unknown expressions and unresolved types remain source data, not invented UML bounds.
pub(super) fn supported_structure(object: &mdj::Object<'_>, index: &mdj::Index<'_>) -> bool {
    if object.type_name != "UMLAssociation"
        || in_view_composition(&object.path)
        || object
            .properties
            .iter()
            .any(|(field, value)| !supported(field, value, false))
    {
        return false;
    }
    let Some(ends) = ends(object, index) else {
        return false;
    };
    // End ownership must not hide a semantic child located outside the two
    // source slots. Their authored type references are not ownership edges.
    if index.objects().any(|child| {
        let owner = reference(child.properties.get("_parent"));
        (owner == Some(object.id) && !ends.iter().any(|end| end.id == child.id))
            || ends.iter().any(|end| owner == Some(end.id))
    }) {
        return false;
    }
    ends.iter().zip(["end1", "end2"]).all(|(end, slot)| {
        end.type_name == "UMLAssociationEnd"
            && end.path == format!("{}/{slot}", object.path)
            && reference(end.properties.get("_parent")) == Some(object.id)
            && reference(end.properties.get("reference")).is_some()
            && end
                .properties
                .get("multiplicity")
                .is_none_or(Value::is_string)
            && end.properties.get("navigable").is_none_or(|value| {
                matches!(
                    value.as_str(),
                    Some("navigable" | "notNavigable" | "unspecified")
                )
            })
            && end
                .properties
                .iter()
                .all(|(field, value)| supported(field, value, true))
    })
}

fn representable(object: &mdj::Object<'_>, index: &mdj::Index<'_>) -> bool {
    if !supported_structure(object, index) {
        return false;
    }
    if let Some(parent) = object.properties.get("_parent") {
        let Some(parent) = reference(Some(parent)).and_then(|id| index.object(id)) else {
            return false;
        };
        if !(parent.path.is_empty() && parent.type_name == "Project")
            && !matches!(
                canonical_kind_for_object(parent, index),
                Some("uml.model" | "uml.package" | "uml.class" | "uml.interface")
            )
        {
            return false;
        }
    }
    ends(object, index).is_some_and(|ends| {
        ends.iter().all(|end| {
            reference(end.properties.get("reference"))
                .and_then(|id| index.object(id))
                .and_then(|target| canonical_kind_for_object(target, index))
                .and_then(crate::uml::root::object_metaclass)
                .is_some_and(|kind| {
                    bellatrix_mde_contracts::uml::metamodel::conforms_to(kind, "Type")
                })
        })
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
                    for field in ["_parent", "reference"] {
                        if !end.properties.contains_key(field) {
                            unsupported.push(format!("end{}.{field} (absent)", position + 1));
                        }
                    }
                }
            }
            diagnostics.push(Diagnostic { path: object.path.clone(), message: format!("association and ends remain opaque: unmapped or absent fields [{}], or unrepresentable end/ownership structure. The supported encoding requires two association-owned ends, navigable/notNavigable/unspecified declarations or the omitted unspecified default, literal multiplicity ranges or the omitted empty-string UML default (1..1), and aggregation absent/none/shared/composite; ownerAttribute and qualifiers are not translated", unsupported.join(", ")) });
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
                properties: BTreeMap::from([(
                    "role".into(),
                    Value::String(
                        if index.object(&parent).is_some_and(|owner| {
                            matches!(owner.type_name, "UMLModel" | "UMLPackage")
                        }) {
                            "packagedElement"
                        } else {
                            "nestedClassifier"
                        }
                        .into(),
                    ),
                )]),
                from: parent,
                to: object.id.into(),
            });
            residue.remove("_parent");
            fields.insert("_parent".into());
        }
        let unverified = ends
            .iter()
            .filter(|end| {
                matches!(
                    end.properties.get("navigable").and_then(Value::as_str),
                    None | Some("unspecified")
                )
            })
            .map(|end| end.id.to_owned())
            .collect::<Vec<_>>();
        if !unverified.is_empty() {
            project.authoring_knowledge.associations.insert(
                object.id.into(),
                crate::AssociationKnowledge {
                    navigation_unverified: unverified,
                },
            );
        }
        let mut relation = Relation {
            id: object.id.into(),
            kind: "uml.association".into(),
            from: String::new(),
            to: String::new(),
            properties,
        };
        for (position, (end, opposite)) in ends.iter().zip(ends.iter().rev()).enumerate() {
            let mut residual = extract_residue(end.properties);
            let mut mapped_fields =
                BTreeSet::from(["_type".into(), "_parent".into(), "reference".into()]);
            let name = take_name(&mut residual, &mut mapped_fields);
            residual.remove("_parent");
            if residual.remove("navigable").is_some() {
                mapped_fields.insert("navigable".into());
            }
            let mut properties = BTreeMap::new();
            if mapped_fields.contains("name") {
                properties.insert("name".into(), Value::String(name.clone()));
            }
            if let Some(value) = residual.remove("reference") {
                properties.insert("type".into(), value);
            }
            for field in END_FIELDS
                .into_iter()
                .filter(|field| *field != "multiplicity")
            {
                if let Some(value) = residual.remove(field) {
                    properties.insert(field.into(), value);
                    mapped_fields.insert(field.into());
                }
            }
            // StarUML stores aggregation at the diamond, while UML Property
            // aggregation belongs to the opposite typed end. Keep identities,
            // roles and multiplicities fixed, and translate only aggregation.
            // Presence metadata continues to describe the original source end.
            if residual.remove("aggregation").is_some() {
                mapped_fields.insert("aggregation".into());
            }
            if let Some(value) = opposite.properties.get("aggregation") {
                properties.insert("aggregation".into(), value.clone());
            }
            // A relationship end inherits public visibility in the source,
            // while UML Property inherits no NamedElement visibility default.
            properties
                .entry("visibility".into())
                .or_insert_with(|| Value::String("public".into()));
            if residual.remove("multiplicity").is_some() {
                mapped_fields.insert("multiplicity".into());
            }
            properties.insert("association".into(), serde_json::json!({"$ref": object.id}));
            // MDJ defaults to false, UML to true. Presence metadata is encoding
            // only; the canonical false remains the sole semantic value.
            properties
                .entry("isUnique".into())
                .or_insert(Value::Bool(false));
            let element = Element {
                id: end.id.into(),
                kind: "uml.property".into(),
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
                properties: BTreeMap::from([("role".into(), Value::String("ownedEnd".into()))]),
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
        diagnostics.push(Diagnostic { path: object.path.clone(), message: "binary association and Property ends mapped with original identities; import policy interprets omitted/empty multiplicity as UML default 1..1, retaining source spelling; explicit literal ranges become owned lowerValue/upperValue specifications; StarUML aggregation is translated to the opposite UML Property, preserving diamond placement; relation endpoints, roles, multiplicities and diamonds are derived projections, not independent editable semantics".into() });
    }
    mapped
}

fn project_endpoint(relation: &mut Relation, end: &Element, position: usize) {
    let target = reference(end.properties.get("type"))
        .unwrap_or_default()
        .to_owned();
    if position == 0 {
        relation.from = target;
    } else {
        relation.to = target;
    }
}

pub(super) fn is_end(metadata: &MdjObjectResidual) -> bool {
    metadata.type_name == "UMLAssociationEnd" && metadata.mapped_fields.contains("_type")
}

pub(super) fn update_metadata(
    project: &Project,
    baseline: &Project,
    provenance: &mut MdjProvenance,
) {
    for end in &project.elements {
        if let Some(metadata) = provenance
            .object_residuals
            .get_mut(&end.id)
            .filter(|metadata| is_end(metadata))
        {
            let original = baseline.elements.iter().find(|value| value.id == end.id);
            for field in END_FIELDS
                .into_iter()
                .filter(|field| *field != "multiplicity")
                .chain(["name"])
            {
                let value = end
                    .properties
                    .get(field)
                    .filter(|value| field == "stereotype" || !value.is_null());
                let prior = original
                    .and_then(|end| end.properties.get(field))
                    .filter(|value| field == "stereotype" || !value.is_null());
                if value != prior {
                    if value.is_some() {
                        metadata.mapped_fields.insert(field.into());
                    } else {
                        metadata.mapped_fields.remove(field);
                    }
                }
            }
            if original.is_some_and(|original| bound_references(original) != bound_references(end))
            {
                metadata.mapped_fields.insert("multiplicity".into());
            }
        }
    }
    for relation in &project.relations {
        if relation.kind != "uml.association" {
            continue;
        }
        if let Some(metadata) = provenance.object_residuals.get_mut(&relation.id) {
            let original = baseline
                .relations
                .iter()
                .find(|value| value.id == relation.id);
            for field in ASSOCIATION_FIELDS.into_iter().chain(["name"]) {
                let value = relation
                    .properties
                    .get(field)
                    .filter(|value| field == "stereotype" || !value.is_null());
                let prior = original
                    .and_then(|relation| relation.properties.get(field))
                    .filter(|value| field == "stereotype" || !value.is_null());
                if value != prior {
                    if value.is_some() {
                        metadata.mapped_fields.insert(field.into());
                    } else {
                        metadata.mapped_fields.remove(field);
                    }
                }
            }
        }
        if let Some(members) = relation
            .properties
            .get("memberEnd")
            .and_then(Value::as_array)
            .filter(|members| members.len() == 2)
        {
            for (member, opposite) in members.iter().zip(members.iter().rev()) {
                let Some(id) = reference(Some(member)) else {
                    continue;
                };
                let aggregation = reference(Some(opposite))
                    .and_then(|id| project.elements.iter().find(|end| end.id == id))
                    .and_then(|end| end.properties.get("aggregation"));
                if let Some(metadata) = provenance.object_residuals.get_mut(id) {
                    let original = reference(Some(opposite))
                        .and_then(|id| baseline.elements.iter().find(|end| end.id == id))
                        .and_then(|end| end.properties.get("aggregation"));
                    if aggregation != original {
                        if aggregation.is_some() {
                            metadata.mapped_fields.insert("aggregation".into());
                        } else {
                            metadata.mapped_fields.remove("aggregation");
                        }
                    }
                }
                let unverified = project
                    .authoring_knowledge
                    .associations
                    .get(&relation.id)
                    .is_some_and(|knowledge| {
                        knowledge.navigation_unverified.iter().any(|end| end == id)
                    });
                if !unverified && let Some(metadata) = provenance.object_residuals.get_mut(id) {
                    metadata.mapped_fields.insert("navigable".into());
                }
            }
        }
    }
}

pub(super) fn export_associations(
    project: &Project,
    baseline: &Project,
    original: &mdj::Index<'_>,
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
            let prior = baseline
                .relations
                .iter()
                .find(|value| value.id == relation.id)
                .and_then(|value| value.properties.get(field));
            if let Some(value) = relation
                .properties
                .get(field)
                .filter(|value| field == "stereotype" || !value.is_null())
                .filter(|value| metadata.mapped_fields.contains(field) || prior != Some(*value))
            {
                insert_mapped(&mut object, field, value.clone())?;
            }
        }
        export_owner(project, &relation.id, metadata, &mut object)?;
        for (position, member) in members.iter().enumerate() {
            let id =
                reference(Some(member)).ok_or_else(|| failure("", "invalid memberEnd encoding"))?;
            let end = project
                .elements
                .iter()
                .find(|end| {
                    end.id == id
                        && crate::uml::root::object_metaclass(&end.kind) == Some("Property")
                })
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
            let mut end_object = base_object(id, "UMLAssociationEnd", &end_metadata.residual)?;
            if let Some(name) = end.properties.get("name").filter(|value| !value.is_null()) {
                insert_mapped(&mut end_object, "name", name.clone())?;
            }
            let baseline_end = baseline
                .elements
                .iter()
                .find(|value| value.id == end.id)
                .ok_or_else(|| failure(&expected_path, "missing original Property end"))?;
            for (field, value) in &end.properties {
                if field == "type" {
                    insert_mapped(&mut end_object, "reference", value.clone())?;
                } else if END_FIELDS.contains(&field.as_str())
                    && field != "multiplicity"
                    && (field == "stereotype" || !value.is_null())
                    && (end_metadata.mapped_fields.contains(field)
                        || baseline_end.properties.get(field) != Some(value))
                {
                    insert_mapped(&mut end_object, field, value.clone())?;
                }
            }
            let notation = export_multiplicity(project, baseline, end, baseline_end, original)?;
            if let Some(notation) = notation {
                insert_mapped(&mut end_object, "multiplicity", Value::String(notation))?;
            }
            let opposite = members
                .iter()
                .rev()
                .nth(position)
                .and_then(|member| reference(Some(member)))
                .and_then(|id| project.elements.iter().find(|end| end.id == id))
                .ok_or_else(|| failure(&expected_path, "missing opposite Property end"))?;
            let baseline_aggregation = baseline
                .elements
                .iter()
                .find(|value| value.id == opposite.id)
                .and_then(|value| value.properties.get("aggregation"));
            if let Some(aggregation) = opposite.properties.get("aggregation").filter(|value| {
                end_metadata.mapped_fields.contains("aggregation")
                    || baseline_aggregation != Some(*value)
            }) {
                insert_mapped(&mut end_object, "aggregation", aggregation.clone())?;
            }
            let is_navigable = navigable
                .iter()
                .any(|value| reference(Some(value)) == Some(id));
            let unverified = project
                .authoring_knowledge
                .associations
                .get(&relation.id)
                .is_some_and(|knowledge| {
                    knowledge.navigation_unverified.iter().any(|end| end == id)
                });
            if unverified && is_navigable {
                return Err(failure(
                    &expected_path,
                    "navigation is both known and unverified",
                ));
            }
            if !unverified || end_metadata.mapped_fields.contains("navigable") {
                insert_mapped(
                    &mut end_object,
                    "navigable",
                    Value::String(
                        if unverified {
                            "unspecified"
                        } else if is_navigable {
                            "navigable"
                        } else {
                            "notNavigable"
                        }
                        .into(),
                    ),
                )?;
            }
            export_owner(project, id, end_metadata, &mut end_object)?;
            if objects
                .insert(id.into(), Value::Object(end_object))
                .is_some()
            {
                return Err(failure("", "duplicate association end identity"));
            }
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
        let ownership = project
            .relations
            .iter()
            .find(|relation| relation.kind == "owns" && relation.to == id)
            .ok_or_else(|| failure("", "missing mapped association ownership"))?;
        insert_mapped(
            object,
            "_parent",
            serde_json::json!({"$ref": ownership.from}),
        )?;
    }
    Ok(())
}

fn bound_references(end: &Element) -> [Option<&str>; 2] {
    ["lowerValue", "upperValue"].map(|field| reference(end.properties.get(field)))
}

pub(super) fn materialize_bounds(
    project: &mut Project,
    index: &mdj::Index<'_>,
    mapped: &BTreeSet<String>,
) -> Result<(), MdjError> {
    let mut section = crate::ModelingSection::from_model(project);
    // Reserve source identities not yet represented in the semantic section,
    // so synthesized bounds cannot collide with retained interchange objects.
    let mut reserved = BTreeSet::new();
    for object in index.objects() {
        if object.id != project.id
            && !section.elements.iter().any(|value| value.id == object.id)
            && !section.relations.iter().any(|value| value.id == object.id)
        {
            reserved.insert(object.id.to_owned());
            section.elements.push(Element {
                id: object.id.into(),
                kind: "mdj.reserved".into(),
                name: String::new(),
                label: None,
                description: None,
                properties: BTreeMap::new(),
                annotations: Vec::new(),
                implementations: Vec::new(),
                content: None,
            });
        }
    }
    for id in mapped {
        let Some(object) = index
            .object(id)
            .filter(|object| object.type_name == "UMLAssociationEnd")
        else {
            continue;
        };
        let Some(notation) = object
            .properties
            .get("multiplicity")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
        else {
            continue;
        };
        crate::uml::associations::set_end_multiplicity(&mut section, &project.id, id, notation)
            .map_err(|error| failure(&object.path, error))?;
    }
    section
        .elements
        .retain(|element| !reserved.contains(&element.id));
    project.elements = section.elements;
    project.relations = section.relations;
    Ok(())
}

pub(super) fn bound_ids(project: &Project) -> Result<BTreeSet<String>, MdjError> {
    let mut result = BTreeSet::new();
    let Some(provenance) = &project.mdj_provenance else {
        return Ok(result);
    };
    for end in &project.elements {
        if !provenance.object_residuals.get(&end.id).is_some_and(is_end) {
            continue;
        }
        for (field, expected_kind) in [
            ("lowerValue", "uml.literal_integer"),
            ("upperValue", "uml.literal_unlimited_natural"),
        ] {
            let Some(value) = end.properties.get(field).filter(|value| !value.is_null()) else {
                continue;
            };
            let id = reference(Some(value))
                .ok_or_else(|| failure("", "invalid multiplicity bound reference"))?;
            let literal = project
                .elements
                .iter()
                .find(|value| value.id == id)
                .ok_or_else(|| failure("", "missing multiplicity bound"))?;
            let owners = project
                .relations
                .iter()
                .filter(|relation| relation.kind == "owns" && relation.to == id)
                .collect::<Vec<_>>();
            if literal.kind != expected_kind
                || owners.len() != 1
                || owners.first().is_none_or(|owner| {
                    owner.from != end.id
                        || owner.properties.get("role").and_then(Value::as_str) != Some(field)
                })
                || !result.insert(id.into())
            {
                return Err(failure(
                    "",
                    "MDJ multiplicity requires exclusive owned literal bounds",
                ));
            }
        }
    }
    Ok(result)
}

fn export_multiplicity(
    project: &Project,
    baseline: &Project,
    end: &Element,
    original_end: &Element,
    original: &mdj::Index<'_>,
) -> Result<Option<String>, MdjError> {
    let source = original
        .object(&end.id)
        .and_then(|object| object.properties.get("multiplicity"))
        .and_then(Value::as_str);
    let current_refs = bound_references(end);
    let original_refs = bound_references(original_end);
    let bound_values = |project: &Project, refs: [Option<&str>; 2]| {
        refs.map(|id| {
            id.and_then(|id| project.elements.iter().find(|element| element.id == id))
                .and_then(|element| element.properties.get("value"))
                .cloned()
        })
    };
    if current_refs == original_refs
        && bound_values(project, current_refs) == bound_values(baseline, original_refs)
    {
        return Ok(source.map(str::to_owned));
    }
    if current_refs.iter().all(Option::is_none) {
        return Ok(Some(String::new()));
    }
    let references =
        crate::uml::references::ReferenceIndex::new(crate::uml::root::Model::from_project(project));
    let range = crate::uml::associations::multiplicity(&references, end)
        .map_err(|error| failure("", format!("unrepresentable MDJ multiplicity: {error:?}")))?;
    let lower = range.lower_bound().to_string();
    let upper = range.upper_bound().to_string();
    Ok(Some(if lower == upper {
        lower
    } else {
        format!("{lower}..{upper}")
    }))
}
