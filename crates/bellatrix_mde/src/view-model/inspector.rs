//! Inspector schema and immutable projection for any selected MDE element.
//!
//! The Inspector is a model capability. It is NOT a fixed GPUI form, nor does
//! it own Project documents. Kinds compose the base element descriptors with
//! their specific descriptors and with existing typed JSON properties.
use crate::{Element, ModelingSection};
use serde_json::{Value, json};
use crate::uml::root::NamedElement;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertyEditor {
    Text,
    Multiline,
    Boolean,
    Number,
    Choice(&'static [&'static str]),
    Reference,
    ReadOnly,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InspectorProperty {
    /// Stable address inside the selected Element: `name`, `label`,
    /// `description`, or `properties.<key>`.
    pub path: String,
    pub label: String,
    pub group: &'static str,
    pub editor: PropertyEditor,
    pub value: Value,
    pub editable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectorMember {
    pub id: String,
    pub name: String,
    pub display_label: String,
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectorReferenceChoice {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ElementInspector {
    pub element_id: String,
    pub kind: String,
    pub name: String,
    pub display_label: String,
    pub properties: Vec<InspectorProperty>,
    pub reference_choices: Vec<InspectorReferenceChoice>,
    /// Owned attributes/operations are model Elements, not strings in a form.
    pub attributes: Vec<InspectorMember>,
    pub operations: Vec<InspectorMember>,
    pub parameters: Vec<InspectorMember>,
    pub receptions: Vec<InspectorMember>,
    pub template_parameters: Vec<InspectorMember>,
    pub ports: Vec<InspectorMember>,
    pub parts: Vec<InspectorMember>,
}

use crate::property_edit::{fields_for, PropertyValueType};

fn declared_label(kind: &str, key: &str) -> &'static str {
    match (kind, key) {
        ("rust.function" | "uml.operation", "isOrdered") => "Return ordered (derived)",
        ("rust.function" | "uml.operation", "isUnique") => "Return unique (derived)",
        ("rust.function" | "uml.operation", "lower") => "Return lower (derived)",
        ("rust.function" | "uml.operation", "upper") => "Return upper (derived)",
        ("rust.function" | "uml.operation", "type") => "Return Type (derived)",
        ("uml.parameter", "type") => "Type reference",
        (_, "stereotype") => "Stereotype",
        (_, "visibility") => "Visibility",
        (_, "isAbstract") => "Abstract",
        (_, "isLeaf") => "Leaf",
        (_, "isFinalSpecialization") => "Final specialization",
        (_, "isActive") => "Active",
        (_, "body") => "Body",
        (_, "aggregation") => "Aggregation",
        (_, "multiplicity") => "Multiplicity notation",
        (_, "declaration") => "Declaration",
        (_, "type") => "Type",
        (_, "defaultValue") => "Default value",
        (_, "isStatic") => "Static",
        (_, "isReadOnly") => "Read only",
        (_, "isOrdered") => "Ordered",
        (_, "isUnique") => "Unique",
        (_, "direction") => "Direction",
        (_, "lower") => "Return lower (derived)",
        (_, "upper") => "Return upper (derived)",
        (_, "signature") => "Authored signature",
        (_, "isQuery") => "Query",
        _ => "Property",
    }
}

fn inherited_value(element: &Element, key: &str, fallback: fn() -> Value) -> Value {
    element.properties.get(key).cloned().unwrap_or_else(|| {
        if key == "body" && crate::uml::root::ElementRef::Object(element).comment().is_some() { json!(element.name) } else { fallback() }
    })
}

fn editor_for_value_type(value_type: PropertyValueType, multiline: bool) -> PropertyEditor {
    match value_type {
        PropertyValueType::String if multiline => PropertyEditor::Multiline,
        PropertyValueType::String => PropertyEditor::Text,
        PropertyValueType::Boolean => PropertyEditor::Boolean,
        PropertyValueType::Number => PropertyEditor::Number,
        PropertyValueType::Choice(options) => PropertyEditor::Choice(options),
        PropertyValueType::Reference => PropertyEditor::Reference,
        PropertyValueType::ReadOnly => PropertyEditor::ReadOnly,
    }
}

fn apply_normative_contracts(element: crate::uml::root::ElementRef<'_>, properties: &mut [InspectorProperty]) {
    for field in properties {
        let Some(key) = field.path.strip_prefix("properties.") else { continue; };
        let fallback = match field.editor {
            PropertyEditor::Text | PropertyEditor::Multiline => PropertyValueType::String,
            PropertyEditor::Boolean => PropertyValueType::Boolean,
            PropertyEditor::Number => PropertyValueType::Number,
            PropertyEditor::Choice(options) => PropertyValueType::Choice(options),
            PropertyEditor::Reference => PropertyValueType::Reference,
            PropertyEditor::ReadOnly => PropertyValueType::ReadOnly,
        };
        let contract = crate::property_edit::normalize_property_contract(element, key, fallback, field.value.clone());
        field.editor = editor_for_value_type(contract.value_type, field.editor == PropertyEditor::Multiline);
        field.editable = contract.value_type != PropertyValueType::ReadOnly;
        field.value = contract.value;
        if key == "type" && contract.value_type == PropertyValueType::Reference {
            field.label = "Type reference".into();
        }
    }
}

fn root_properties(section: &ModelingSection, id: &str) -> Vec<InspectorProperty> {
    let model = crate::uml::root::Model::from_section(section);
    let Some(element) = model.element(id) else { return Vec::new(); };
    let owner = model.owner(element).map(|owner| json!(owner.map(|owner| owner.id()))).unwrap_or_else(|error| json!(error));
    let namespace = element.named().map(|named| model.namespace(named)).transpose().map(|namespace| json!(namespace.flatten().map(|namespace| namespace.element().id()))).unwrap_or_else(|error| json!(error));
    let qualified = element.named().map(|named| named.qualified_name(model)).transpose().map(|name| json!(name.flatten())).unwrap_or_else(|error| json!(error));
    [("uml.metaclass", "UML metaclass", json!(element.metaclass())), ("uml.owner", "Owner", json!(owner)),
     ("uml.namespace", "Namespace", json!(namespace)), ("uml.qualifiedName", "Qualified name", json!(qualified))]
        .into_iter().map(|(path, label, value)| InspectorProperty { path: path.into(), label: label.into(), group: "UML identity", editor: PropertyEditor::ReadOnly, value, editable: false }).collect()
}

pub(crate) fn append_namespace_properties(
    references: &crate::uml::references::ReferenceIndex<'_>,
    id: &str,
    properties: &mut Vec<InspectorProperty>,
) {
    let Ok(element) = references.element(id) else { return; };
    if element.conforms_to("RedefinableElement") {
        let value = crate::uml::redefinition_contexts::resolve_redefinition_contexts(references, element)
            .map(|contexts| json!(contexts.iter().map(|context| json!({"$ref": context.id()})).collect::<Vec<_>>()))
            .unwrap_or_else(|error| json!(error.to_string()));
        properties.push(InspectorProperty { path: "uml.redefinitionContext".into(), label: "Redefinition contexts".into(), group: "UML members", editor: PropertyEditor::ReadOnly, value, editable: false });
    }
    if let Some(warning) = crate::uml::classifiers::redefinition_consistency_diagnostic(references, element) {
        properties.push(InspectorProperty {
            path: "uml.redefinition.status".into(), label: "Redefinition consistency status".into(),
            group: "UML members", editor: PropertyEditor::ReadOnly, value: json!(warning.message), editable: false,
        });
    }
    if element.conforms_to("Property") {
        let rows = [
            ("uml.subsettedProperty", "Subsetted Properties", crate::uml::property_subsetting::subsetted_properties(references, element)),
            ("uml.subsettingContext", "Subsetting contexts", crate::uml::property_subsetting::subsetting_context(references, element)),
        ];
        for (path, label, result) in rows {
            let value = result.map(|values| json!(values.iter().map(|value| json!({"$ref": value.id()})).collect::<Vec<_>>()))
                .unwrap_or_else(|error| json!(error.to_string()));
            properties.push(InspectorProperty { path: path.into(), label: label.into(), group: "UML members", editor: PropertyEditor::ReadOnly, value, editable: false });
        }
        if let Some(issue) = crate::uml::property_subsetting::subsetting_diagnostic(references, element) {
            properties.push(InspectorProperty { path: "uml.subsetting.status".into(), label: "Subsetting status".into(), group: "UML members", editor: PropertyEditor::ReadOnly, value: json!(issue.message), editable: false });
        }
    }
    if !element.conforms_to("Namespace") { return; }
    let result = crate::uml::classifiers::query(references, id);
    let mut append = |path: &str, label: &str, value: Value| properties.push(InspectorProperty {
        path: path.into(), label: label.into(), group: "UML members",
        editor: PropertyEditor::ReadOnly, value, editable: false,
    });
    let ids = |members: &[crate::uml::root::ElementRef<'_>]| {
        Value::Array(members.iter().map(|member| json!({"$ref": member.id()})).collect())
    };
    match result {
        Err(error) => append("uml.members.status", "Member query unavailable", json!(error.to_string())),
        Ok(query) => {
            append("uml.ownedMember", "Owned members", ids(&query.owned_members));
            append("uml.member", "All members", ids(&query.members));
            append("uml.importedMember", "Imported members", ids(&query.imported_members));
            append("uml.importedNames", "Imported names and aliases", json!(query.imported_names));
            if !query.excluded_imports.is_empty() {
                append("uml.excludedImports", "Excluded colliding imports", ids(&query.excluded_imports));
            }
            if let Some(visible) = query.visible_members {
                append("uml.visibleMembers", "Public package members", ids(&visible));
            }
            if let Some(classifier) = query.classifier {
                append("uml.general", "Direct parents", ids(&classifier.parents));
                append("uml.allParents", "All ancestor classifiers", ids(&classifier.all_parents));
                match classifier.all_slottable_features {
                    Ok(features) => append("uml.allSlottableFeatures", "All slottable features", ids(&features)),
                    Err(error) => append("uml.allSlottableFeatures.status", "Slottable feature query unavailable", json!(error.to_string())),
                }
                append("uml.inheritedMember", "Inherited members", ids(&classifier.inherited_members));
                append("uml.feature", "Direct features", ids(&classifier.features));
                append("uml.attribute", "Direct attributes", ids(&classifier.attributes));
                append("uml.allFeatures", "All features", ids(&classifier.all_features));
                append("uml.allAttributes", "All attributes", ids(&classifier.all_attributes));
            }
        }
    }
}

fn append_association_properties(
    references: &crate::uml::references::ReferenceIndex<'_>,
    id: &str,
    properties: &mut Vec<InspectorProperty>,
) {
    use crate::uml::{associations, root::ElementRef};
    let Ok(element) = references.element(id) else { return; };
    let mut append = |path: &str, label: &str, value: Result<Value, String>| properties.push(InspectorProperty {
        path: path.into(), label: label.into(), group: "UML association ends",
        editor: PropertyEditor::ReadOnly,
        value: value.unwrap_or_else(|error| json!(error)), editable: false,
    });
    if element.conforms_to("Association") {
        append("uml.memberEnd", "Member ends (ordered)", associations::member_ends(references, id)
            .map(|ends| json!(ends.iter().map(|end| json!({"$ref": end.id})).collect::<Vec<_>>())));
        append("uml.ownedEnd", "Association-owned ends", associations::owned_ends(references, id)
            .map(|ends| json!(ends.iter().map(|end| json!({"$ref": end.id})).collect::<Vec<_>>())));
        append("uml.navigableOwnedEnd", "Navigable owned ends", associations::navigable_owned_ends(references, id)
            .map(|ends| json!(ends.iter().map(|end| json!({"$ref": end.id})).collect::<Vec<_>>())));
        append("uml.endType", "End Types", associations::end_types(references, id)
            .map(|types| json!(types.iter().map(|element| json!({"$ref": element.id()})).collect::<Vec<_>>())));
    } else if let ElementRef::Object(property) = element && element.conforms_to("Property") {
        append("uml.association", "Association", associations::association_of(references, id)
            .map(|association| association.map_or(Value::Null, |association| json!({"$ref": association.id()}))));
        append("uml.owningAssociation", "Owning Association", associations::owning_association(references, id)
            .map(|owner| owner.map_or(Value::Null, |owner| json!({"$ref": owner.id()}))));
        append("uml.opposite", "Binary opposite", associations::opposite(references, id)
            .map(|opposite| opposite.map_or(Value::Null, |opposite| json!({"$ref": opposite.id}))));
        append("uml.isComposite", "Composite (derived)", associations::is_composite(property).map(Value::Bool));
        append("uml.isNavigable", "Navigable (derived)", associations::is_navigable(references, id).map(Value::Bool));
    }
}

pub fn inspect_element(section: &ModelingSection, id: &str) -> Option<ElementInspector> {
    let references = crate::uml::references::ReferenceIndex::new(
        crate::uml::root::Model::from_section(section),
    );
    inspect_element_with_references(section, id, &references)
}

pub(crate) fn inspect_element_with_references(
    section: &ModelingSection,
    id: &str,
    references: &crate::uml::references::ReferenceIndex<'_>,
) -> Option<ElementInspector> {
    let mut reference_choices = references.element(id).ok()
        .and_then(|element| element.metaclass())
        .and_then(|metaclass| crate::uml::schema::uml_property(metaclass, "type").ok().flatten())
        .map(|property| references.candidates(property).into_iter().map(|target| {
            let name = target.named()
                .and_then(|named| crate::uml::root::Model::qualified_name_with_index(named, references).ok().flatten())
                .unwrap_or_else(|| target.named().and_then(|named| named.name())
                    .unwrap_or("(unnamed)").into());
            InspectorReferenceChoice {
                id: target.id().into(),
                label: format!("{name} · {} [{}]", target.metaclass().unwrap_or(target.kind()), target.id()),
            }
        }).collect::<Vec<_>>()).unwrap_or_default();
    reference_choices.sort_by(|left, right| left.label.cmp(&right.label));
    // Relationship records share Element identity without acquiring unrelated Class properties.
    if let Some(relation) = section.relations.iter().find(|relation| relation.id == id
        && (crate::uml::root::relationship_metaclass(&relation.kind).is_some() || relation.kind.starts_with("uml.") || section.diagrams.iter().any(|diagram|
            diagram.views.iter().any(|view| view.subject.as_deref() == Some(id))
        ))) {
        let source = section.elements.iter().find(|e|e.id==relation.from)
            .map(|e|e.name.as_str()).unwrap_or(relation.from.as_str());
        let target = section.elements.iter().find(|e|e.id==relation.to)
            .map(|e|e.name.as_str()).unwrap_or(relation.to.as_str());
        let mut properties=vec![
            InspectorProperty {path:"id".into(),label:"ID".into(),group:"Identity",
                editor:PropertyEditor::ReadOnly,value:json!(relation.id),editable:false},
            InspectorProperty {path:"kind".into(),label:"Relationship".into(),group:"Identity",
                editor:PropertyEditor::ReadOnly,value:json!(relation.kind),editable:false},
            InspectorProperty {path:"from".into(),label:"Source".into(),group:"Endpoints",
                editor:PropertyEditor::ReadOnly,value:json!(source),editable:false},
            InspectorProperty {path:"to".into(),label:"Target".into(),group:"Endpoints",
                editor:PropertyEditor::ReadOnly,value:json!(target),editable:false},
        ];
        properties.extend(root_properties(section, id));
        append_namespace_properties(references, id, &mut properties);
        append_association_properties(references, id, &mut properties);
        let semantic_named = crate::uml::root::Model::from_section(section).element(id).and_then(|element| element.named()).is_some();
        let mut known=HashSet::new();
        for definition in crate::property_edit::relationship_fields(crate::uml::root::ElementRef::Relationship(relation)) {
            let key = definition.key;
            known.insert(key);
            let label = match key {
                "name" if semantic_named => "Name",
                "name" => "Editor label (not a UML name)",
                "sourceMultiplicity" => "Source multiplicity",
                "targetMultiplicity" => "Target multiplicity",
                "sourceRole" => "Source role",
                "targetRole" => "Target role",
                "navigable" => "Navigable",
                "stereotype" => "Stereotype",
                "end1Aggregation" => "Source diamond",
                "end2Aggregation" => "Target diamond",
                _ => "Property",
            };
            properties.push(InspectorProperty {
                path:format!("properties.{key}"), label:label.into(), group:"Relationship",
                editor:editor_for_value_type(definition.value_type, false),
                value:relation.properties.get(key).cloned().unwrap_or_else(definition.default),
                editable:true,
            });
        }
        for (key,value) in &relation.properties {
            if known.contains(key.as_str()) {continue;}
            let (editor,editable)=if key=="end1Aggregation" {
                (PropertyEditor::Choice(&["none","shared","composite"]),true)
            } else { match value {
                Value::String(_)=>(PropertyEditor::Text,true),
                Value::Bool(_)=>(PropertyEditor::Boolean,true),
                Value::Number(_)=>(PropertyEditor::Number,true),
                _=>(PropertyEditor::ReadOnly,false),
            } };
            properties.push(InspectorProperty {
                path:format!("properties.{key}"),label:key.clone(),
                group:"Other properties",editor,value:value.clone(),editable,
            });
        }
        apply_normative_contracts(crate::uml::root::ElementRef::Relationship(relation), &mut properties);
        return Some(ElementInspector {
            element_id:relation.id.clone(),kind:relation.kind.clone(),
            name:format!("{source} → {target}"),
            display_label:format!("{source} → {target}"),
            reference_choices,properties,attributes:Vec::new(),operations:Vec::new(),parameters:Vec::new(),receptions:Vec::new(),
            template_parameters:Vec::new(),ports:Vec::new(),parts:Vec::new(),
        });
    }
    let element = section.elements.iter().find(|element| element.id == id)?;
    let has_type_contract = crate::uml::root::ElementRef::Object(element).metaclass()
        .and_then(|metaclass| crate::uml::schema::uml_property(metaclass, "type").ok().flatten()).is_some();
    let semantic_name = crate::template_parametered_element(section, element).map_or(element.name.as_str(), |exposed| exposed.name.as_str());
    let display_label = if crate::templates::is_parameter(&element.kind) { crate::template_parameter_label_with_references(section, element, references) } else { crate::element_display_label_with_references(element, references) };
    let mut properties = vec![
        InspectorProperty { path:"id".into(), label:"ID".into(), group:"Identity",
            editor:PropertyEditor::ReadOnly, value:json!(element.id), editable:false },
        InspectorProperty { path:"kind".into(), label:"Type".into(), group:"Identity",
            editor:PropertyEditor::ReadOnly, value:json!(element.kind), editable:false },
        InspectorProperty { path:"name".into(), label:"Name".into(), group:"Identity",
            editor:PropertyEditor::Text, value:json!(semantic_name), editable:true },
        InspectorProperty { path:"label".into(), label:"Label metadata (not diagram name)".into(), group:"Metadata",
            editor:PropertyEditor::Text, value:json!(element.label.clone().unwrap_or_default()), editable:true },
    ];
    properties.extend(root_properties(section, id));
    append_namespace_properties(references, id, &mut properties);
    append_association_properties(references, id, &mut properties);
    if crate::uml::root::ElementRef::Object(element).named().is_none()
        && let Some(name) = properties.iter_mut().find(|property| property.path == "name") {
        name.label = if crate::templates::is_parameter(&element.kind) { "Parametered element name" } else { "Editor label (not a UML name)" }.into();
        name.group = "Metadata";
    }
    let mut known = HashSet::new();
    for definition in fields_for(&element.kind) {
        if !known.insert(definition.key) { continue; }
        properties.push(InspectorProperty {
            path: format!("properties.{}", definition.key),
            label:declared_label(&element.kind, definition.key).into(), group:"Specialized",
            editor:editor_for_value_type(definition.value_type, definition.key == "body"),
            value:inherited_value(element, definition.key, definition.default),
            editable:true,
        });
    }
    // Unknown collections remain visible but cannot be modified as scalar values.
    for (key,value) in &element.properties {
        if known.contains(key.as_str()) { continue; }
        let (editor, editable) = match value {
            Value::String(_) => (PropertyEditor::Text, true),
            Value::Bool(_) => (PropertyEditor::Boolean, true),
            Value::Number(_) => (PropertyEditor::Number, true),
            _ => (PropertyEditor::ReadOnly, false),
        };
        properties.push(InspectorProperty {path:format!("properties.{key}"),label:key.clone(),
            group:"Properties",editor,value:value.clone(),editable});
    }
    if has_type_contract && let Some(text) = element.properties.get("type").filter(|value| value.is_string()) {
        properties.push(InspectorProperty {path:"legacy.type".into(),label:"Type text (unresolved)".into(),group:"Authored text",editor:PropertyEditor::ReadOnly,value:text.clone(),editable:false});
    }
    if has_type_contract && let Some(field) = properties.iter_mut().find(|field| field.path == "properties.typeText") {
        field.label = "Preserved type text (unresolved)".into();
        field.group = "Authored text";
        field.editor = PropertyEditor::ReadOnly;
        field.editable = false;
    }
    properties.push(InspectorProperty {path:"description".into(),label:"Documentation".into(),
        group:"Documentation",editor:PropertyEditor::Multiline,
        value:json!(element.description.clone().unwrap_or_default()),editable:true});

    let mut attributes = Vec::new();
    let mut operations = Vec::new();
    let mut receptions = Vec::new();
    let parameters = if crate::uml::root::ElementRef::Object(element).conforms_to("Operation") {
        crate::uml::operations::owned_parameters(references, id)
            .unwrap_or_default()
            .into_iter()
            .map(|member| InspectorMember {
                id: member.id.clone(),
                name: member.name.clone(),
                kind: member.kind.clone(),
                display_label: crate::element_display_label_with_references(member, references),
            })
            .collect()
    } else {
        Vec::new()
    };
    let template_parameters = crate::template_parameters(section, &element.id).into_iter().map(|member| InspectorMember {
        id:member.id.clone(), name:crate::template_parametered_element(section, member).unwrap_or(member).name.clone(), kind:member.kind.clone(),
        display_label: crate::template_parameter_label_with_references(section, member, references),
    }).collect();
    let mut ports=Vec::new();
    let mut parts=Vec::new();
    for relation in &section.relations {
        if relation.kind != "owns" || relation.from != element.id {continue;}
        if let Some(member) = section.elements.iter().find(|e| e.id == relation.to) {
            let descriptor = InspectorMember {
                id:member.id.clone(),name:member.name.clone(),kind:member.kind.clone(),
                display_label:crate::element_display_label_with_references(member, references),
            };
            match member.kind.as_str() {
                "rust.field" | "uml.attribute" => attributes.push(descriptor),
                "rust.function" | "uml.operation" => operations.push(descriptor),
                "uml.reception" => receptions.push(descriptor),

                "uml.port" => ports.push(descriptor),
                "uml.part" => parts.push(descriptor),
                _ => {},
            }
        }
    }
    apply_normative_contracts(crate::uml::root::ElementRef::Object(element), &mut properties);
    if crate::uml::value_specifications::has_bound_references(crate::uml::root::ElementRef::Object(element)) {
        if let Some(field) = properties.iter_mut().find(|field| field.path == "properties.multiplicity") {
            field.editor = PropertyEditor::ReadOnly;
            field.editable = false;
            field.group = "Authored text";
            field.label = "Preserved multiplicity notation (bound identities are authoritative)".into();
        }
        let bounds = crate::uml::value_specifications::multiplicity(references, crate::uml::root::ElementRef::Object(element));
        for (path, label, lower) in [("uml.lower", "Lower bound (derived)", true), ("uml.upper", "Upper bound (derived)", false)] {
            let value = match &bounds {
                Ok(bounds) if lower => json!(bounds.lower_bound().to_string()),
                Ok(bounds) => json!(bounds.upper_bound().to_string()),
                Err(error) => json!(error.to_string()),
            };
            properties.push(InspectorProperty { path: path.into(), label: label.into(), group: "UML derived", editor: PropertyEditor::ReadOnly, value, editable: false });
        }
    }
    if crate::uml::root::ElementRef::Object(element).conforms_to("Operation") {
        let result = crate::uml::operations::return_properties(references, id);
        for key in ["isOrdered", "isUnique", "lower", "upper"] {
            if let Some(field) = properties.iter_mut().find(|field| field.path == format!("properties.{key}")) {
                field.value = match &result {
                    Err(error) => json!(error),
                    Ok(result) => match key {
                        "isOrdered" => json!(result.is_ordered),
                        "isUnique" => json!(result.is_unique),
                        "lower" => result.lower().map(|value| json!(value.map(ToString::to_string))).unwrap_or_else(|error| json!(error.to_string())),
                        "upper" => result.upper().map(|value| json!(value.map(ToString::to_string))).unwrap_or_else(|error| json!(error.to_string())),
                        _ => Value::Null,
                    },
                };
            }
            if let Some(value) = element.properties.get(key) {
                properties.push(InspectorProperty {
                    path: format!("legacy.operation.{key}"), label: format!("Stored {key} (ignored; derived from Parameter)"),
                    group: "Authored text", editor: PropertyEditor::ReadOnly, value: value.clone(), editable: false,
                });
            }
        }
        if let Some(field) = properties
            .iter_mut()
            .find(|field| field.path == "properties.type")
        {
            field.value = match crate::uml::operations::return_type(references, id) {
                Ok(Some(target)) => json!({"$ref": target.id()}),
                Ok(None) => Value::Null,
                Err(error) => json!(error),
            };
        }
        if let Some(value) = element
            .properties
            .get("type")
            .filter(|value| !value.is_string() && !value.is_null())
        {
            properties.push(InspectorProperty {
                path: "legacy.operation.type".into(),
                label: "Stored return Type (ignored; derived from Parameter)".into(),
                group: "Authored text",
                editor: PropertyEditor::ReadOnly,
                value: value.clone(),
                editable: false,
            });
        }
    }

    Some(ElementInspector {element_id:element.id.clone(),kind:element.kind.clone(),
        name:semantic_name.into(),display_label,reference_choices,properties,attributes,operations,parameters,receptions,
        template_parameters,ports,parts})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::set_inspected_property;
    use crate::property_edit::VISIBILITY;
    fn fixture() -> ModelingSection {
        let model: crate::Project = serde_json::from_str(include_str!("../../fixtures/todo.json")).unwrap();
        ModelingSection::from_model(&model)
    }
    #[test]
    fn projected_editors_use_the_domain_edit_contract() {
        let project: crate::Project = serde_json::from_str(include_str!("../../fixtures/todo.json")).expect("fixture");
        let section = ModelingSection::from_model(&project);
        for id in section.elements.iter().map(|element| element.id.as_str())
            .chain(section.relations.iter().map(|relation| relation.id.as_str())) {
            let Some(inspected) = inspect_element(&section, id) else { continue; };
            for field in inspected.properties {
                let contract = crate::property_edit::property_contract(&section, id, &field.path)
                    .unwrap_or_else(|error| panic!("{id} {}: {error}", field.path));
                let expected = editor_for_value_type(contract.value_type, field.editor == PropertyEditor::Multiline);
                assert_eq!(field.editor, expected, "{id} {}", field.path);
                assert_eq!(field.editable, contract.value_type != PropertyValueType::ReadOnly, "{id} {}", field.path);
            }
        }
    }

    #[test]
    fn class_inherits_common_properties_and_owns_attributes_and_operations() {
        let section=fixture();
        let c=inspect_element(&section,"type-list").unwrap();
        assert_eq!(c.kind,"rust.struct");
        assert!(c.properties.iter().any(|p| p.path=="properties.visibility" &&
            p.value == json!("public") && p.editor==PropertyEditor::Choice(VISIBILITY)));
        assert!(c.properties.iter().any(|p| p.path=="properties.isActive" &&
            p.editor==PropertyEditor::Boolean));
        assert!(c.attributes.iter().any(|f| f.name=="tasks"));
        assert!(c.operations.iter().any(|f| f.name=="add"));
    }
    #[test]
    fn specialized_field_has_declaration_but_not_class_flags() {
        let section=fixture();
        let f=inspect_element(&section,"field-title").unwrap();
        assert!(f.properties.iter().any(|p| p.path=="properties.declaration"));
        assert!(!f.properties.iter().any(|p| p.path=="properties.isActive"));
    }
    #[test]
    fn updates_are_typed_and_reject_identity_changes() {
        let mut section=fixture();
        assert!(set_inspected_property(&mut section,"type-list","id",json!("oops")).is_err());
        assert!(set_inspected_property(&mut section,"type-list","properties.isActive",json!("yes")).is_err());
        assert!(set_inspected_property(&mut section,"type-list","properties.visibility",json!("other")).is_err());
        set_inspected_property(&mut section,"type-list","properties.visibility",json!("private")).unwrap();
        set_inspected_property(&mut section,"type-list","description",json!("Stored documentation")).unwrap();
        assert_eq!(inspect_element(&section,"type-list").unwrap().properties.iter()
            .find(|p| p.path=="properties.visibility").unwrap().value,json!("private"));
    }
}
