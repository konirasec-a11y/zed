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

const VISIBILITY: &[&str] = &["public", "private", "protected", "package"];

#[derive(Clone, Copy)]
struct Declared {
    key: &'static str,
    label: &'static str,
    editor: PropertyEditor,
    default: fn() -> Value,
}
fn empty() -> Value { json!("") }
fn public() -> Value { json!("public") }
fn false_value() -> Value { json!(false) }

const COMMON_CLASSIFIER: &[Declared] = &[
    Declared { key:"stereotype", label:"Stereotype", editor:PropertyEditor::Text, default:empty },
    Declared { key:"visibility", label:"Visibility", editor:PropertyEditor::Choice(VISIBILITY), default:public },
    Declared { key:"isAbstract", label:"Abstract", editor:PropertyEditor::Boolean, default:false_value },
    Declared { key:"isLeaf", label:"Leaf", editor:PropertyEditor::Boolean, default:false_value },
];
const CLASS: &[Declared] = &[
    Declared { key:"isFinalSpecialization", label:"Final specialization", editor:PropertyEditor::Boolean, default:false_value },
    Declared { key:"isActive", label:"Active", editor:PropertyEditor::Boolean, default:false_value },
];
const COMMENT: &[Declared] = &[Declared { key: "body", label: "Body", editor: PropertyEditor::Multiline, default: empty }];
const FIELD: &[Declared] = &[
    Declared { key:"aggregation", label:"Aggregation", editor:PropertyEditor::Choice(&["none", "shared", "composite"]), default:empty },
    Declared { key:"multiplicity", label:"Multiplicity notation", editor:PropertyEditor::Text, default:empty },
    Declared { key:"visibility", label:"Visibility", editor:PropertyEditor::Choice(VISIBILITY), default:public },
    Declared { key:"declaration", label:"Declaration", editor:PropertyEditor::Text, default:empty },
    Declared { key:"type", label:"Type", editor:PropertyEditor::Text, default:empty },
    Declared { key:"defaultValue", label:"Default value", editor:PropertyEditor::Text, default:empty },
    Declared { key:"isStatic", label:"Static", editor:PropertyEditor::Boolean, default:false_value },
    Declared { key:"isReadOnly", label:"Read only", editor:PropertyEditor::Boolean, default:false_value },
];
const PARAMETER: &[Declared] = &[
    Declared { key:"direction", label:"Direction", editor:PropertyEditor::ReadOnly, default:empty },
    Declared { key:"type", label:"Type reference", editor:PropertyEditor::Reference, default:empty },
];
const FUNCTION: &[Declared] = &[
    Declared { key:"type", label:"Return Type (derived)", editor:PropertyEditor::ReadOnly, default:empty },
    Declared { key:"visibility", label:"Visibility", editor:PropertyEditor::Choice(VISIBILITY), default:public },
    Declared { key:"signature", label:"Authored signature", editor:PropertyEditor::Text, default:empty },
    Declared { key:"isStatic", label:"Static", editor:PropertyEditor::Boolean, default:false_value },
    Declared { key:"isAbstract", label:"Abstract", editor:PropertyEditor::Boolean, default:false_value },
    Declared { key:"isQuery", label:"Query", editor:PropertyEditor::Boolean, default:false_value },
];

fn fields_for(kind: &str) -> impl Iterator<Item = &Declared> {
    let semantic_type = crate::UmlType::from_kind(kind);
    COMMON_CLASSIFIER.iter().chain(CLASS.iter()).filter(move |field| {
        semantic_type.is_some_and(|kind| {
            // The legacy editor template alias is not a normative metaclass.
            if kind == crate::UmlType::Template {
                return COMMON_CLASSIFIER.iter().any(|declared| declared.key == field.key);
            }
            if field.key == "stereotype" {
                kind.is_classifier()
            } else {
                kind.has_property(field.key)
            }
        })
    }).chain(match kind {
        "rust.field" | "uml.attribute" | "uml.part" | "uml.port" => FIELD,
        "uml.comment" | "uml.note" => COMMENT,
        "rust.function" | "uml.operation" => FUNCTION,
        "uml.parameter" => PARAMETER,
        _ => &[],
    }.iter())
}

fn inherited_value(element: &Element, key: &str, fallback: fn() -> Value) -> Value {
    element.properties.get(key).cloned().unwrap_or_else(|| {
        if key == "body" && crate::uml::root::ElementRef::Object(element).comment().is_some() { json!(element.name) } else { fallback() }
    })
}

fn apply_normative_contracts(element: crate::uml::root::ElementRef<'_>, properties: &mut [InspectorProperty]) {
    use crate::uml::schema::{self, ScalarKind};
    use crate::uml::types::StandardPrimitiveType;
    let Some(metaclass) = element.metaclass() else { return; };
    for field in properties {
        let Some(key) = field.path.strip_prefix("properties.") else { continue; };
        let Ok(Some(descriptor)) = schema::uml_property(metaclass, key) else { continue; };
        if (key == "ownedParameter" && element.conforms_to("Operation"))
            || (key == "operation" && element.conforms_to("Parameter")) {
            field.editor = PropertyEditor::ReadOnly;
            field.editable = false;
            continue;
        }
        if descriptor.is_read_only || descriptor.is_derived {
            field.editor = PropertyEditor::ReadOnly;
            field.editable = false;
            continue;
        }
        if key == "type" && (element.conforms_to("Property") || element.conforms_to("Parameter")) {
            field.editor = PropertyEditor::Reference;
            field.label = "Type reference".into();
            field.editable = true;
            if field.value.is_string() { field.value = Value::Null; }
            continue;
        }
        if schema::uses_legacy_text_adapter(descriptor, &field.value) { continue; }
        let Ok(kind) = schema::scalar_kind(descriptor) else { continue; };
        field.editable = true;
        field.editor = match kind {
            ScalarKind::Enumeration(literals) => PropertyEditor::Choice(literals),
            ScalarKind::Primitive(StandardPrimitiveType::Boolean) => PropertyEditor::Boolean,
            ScalarKind::Primitive(StandardPrimitiveType::Integer | StandardPrimitiveType::UnlimitedNatural) => PropertyEditor::Text,
            ScalarKind::Primitive(StandardPrimitiveType::Real) => PropertyEditor::Number,
            ScalarKind::Primitive(StandardPrimitiveType::String) => if field.editor == PropertyEditor::Multiline { PropertyEditor::Multiline } else { PropertyEditor::Text },
        };
        if !element.properties().contains_key(key) {
            match schema::effective_default(descriptor) {
                Ok(Some(default)) => {
                    if let Some(value) = default.scalar_json() { field.value = value; }
                },
                Ok(None) if descriptor.lower == "0" && key != "body" => field.value = Value::Null,
                _ => {},
            }
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
    if let Some(warning) = crate::uml::classifiers::redefinition_consistency_warning(element) {
        properties.push(InspectorProperty {
            path: "uml.redefinition.status".into(), label: "Redefinition consistency not evaluated".into(),
            group: "UML members", editor: PropertyEditor::ReadOnly, value: json!(warning.message), editable: false,
        });
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
            if let Some(visible) = query.visible_members {
                append("uml.visibleMembers", "Public package members", ids(&visible));
            }
            if let Some(classifier) = query.classifier {
                append("uml.general", "Direct parents", ids(&classifier.parents));
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
        append("uml.endType", "End Types", associations::end_types(references, id)
            .map(|types| json!(types.iter().map(|element| json!({"$ref": element.id()})).collect::<Vec<_>>())));
    } else if let ElementRef::Object(property) = element && element.conforms_to("Property") {
        append("uml.association", "Association", associations::association_of(references, id)
            .map(|association| association.map_or(Value::Null, |association| json!({"$ref": association.id()}))));
        append("uml.owningAssociation", "Owning Association", references.model().owner(element)
            .map(|owner| owner.filter(|owner| owner.conforms_to("Association"))
                .map_or(Value::Null, |owner| json!({"$ref": owner.id()}))));
        append("uml.opposite", "Binary opposite", associations::opposite(references, id)
            .map(|opposite| opposite.map_or(Value::Null, |opposite| json!({"$ref": opposite.id}))));
        append("uml.isComposite", "Composite (derived)", associations::is_composite(property).map(Value::Bool));
    }
}

pub fn inspect_element(section: &ModelingSection, id: &str) -> Option<ElementInspector> {
    let references = crate::uml::references::ReferenceIndex::new(
        crate::uml::root::Model::from_section(section),
    );
    inspect_element_with_references(section, id, &references)
}

fn inspect_element_with_references(
    section: &ModelingSection,
    id: &str,
    references: &crate::uml::references::ReferenceIndex<'_>,
) -> Option<ElementInspector> {
    let model = crate::uml::root::Model::from_section(section);
    let mut reference_choices = references.element(id).ok()
        .and_then(|element| element.metaclass())
        .and_then(|metaclass| crate::uml::schema::uml_property(metaclass, "type").ok().flatten())
        .map(|property| references.candidates(property).into_iter().map(|target| {
            let name = target.named()
                .and_then(|named| named.qualified_name(model).ok().flatten())
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
        let common = [
            ("name",if semantic_named { "Name" } else { "Editor label (not a UML name)" },PropertyEditor::Text,Value::String(String::new())),
            ("sourceMultiplicity","Source multiplicity",PropertyEditor::Text,json!("")),
            ("targetMultiplicity","Target multiplicity",PropertyEditor::Text,json!("")),
            ("sourceRole","Source role",PropertyEditor::Text,json!("")),
            ("targetRole","Target role",PropertyEditor::Text,json!("")),
            ("navigable","Navigable",PropertyEditor::Boolean,json!(false)),
            ("stereotype","Stereotype",PropertyEditor::Text,json!("")),
        ];
        let mut known=HashSet::new();
        for (key,label,editor,default) in common {
            known.insert(key);
            properties.push(InspectorProperty {
                path:format!("properties.{key}"),label:label.into(),
                group:"Relationship",editor,
                value:relation.properties.get(key).cloned().unwrap_or(default),
                editable:true,
            });
        }
        if crate::uml::root::relationship_metaclass(&relation.kind) == Some("Association") {
            for (key, label) in [("end1Aggregation", "Source diamond"), ("end2Aggregation", "Target diamond")] {
                known.insert(key);
                properties.push(InspectorProperty {
                    path: format!("properties.{key}"), label: label.into(), group: "Relationship",
                    editor: PropertyEditor::Choice(&["none", "shared", "composite"]),
                    value: relation.properties.get(key).cloned().unwrap_or_else(|| json!("none")), editable: true,
                });
            }
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
            label:definition.label.into(), group:"Specialized",
            editor:definition.editor,
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
    if crate::uml::root::ElementRef::Object(element).conforms_to("Operation") {
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
    let inspected = inspect_element_with_references(section, id, &references).ok_or("element not found")?;
    let field = inspected.properties.iter().find(|f| f.path == path)
        .ok_or("property is not declared")?;
    if !field.editable { return Err("property is read-only".into()); }
    let optional_null = value.is_null() && path.strip_prefix("properties.").is_some_and(|key| {
        crate::uml::root::Model::from_section(section).element(id)
            .and_then(|element| element.metaclass())
            .and_then(|metaclass| crate::uml::schema::uml_property(metaclass, key).ok().flatten())
            .is_some_and(|descriptor| crate::uml::schema::validate_scalar(descriptor, &value) == Ok(Ok(())))
    });
    match field.editor {
        _ if optional_null => {},
        PropertyEditor::ReadOnly => return Err("property is read-only".into()),
        PropertyEditor::Text | PropertyEditor::Multiline => {
            value.as_str().ok_or("expected string")?;
        },
        PropertyEditor::Boolean if !value.is_boolean() => return Err("expected boolean".into()),
        PropertyEditor::Number if !value.is_number() => return Err("expected number".into()),
        PropertyEditor::Choice(options) if !value.as_str()
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

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> ModelingSection {
        let model: crate::Project = serde_json::from_str(include_str!("../fixtures/todo.json")).unwrap();
        ModelingSection::from_model(&model)
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
