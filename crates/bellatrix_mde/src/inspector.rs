//! Inspector schema and immutable projection for any selected MDE element.
//!
//! The Inspector is a model capability. It is NOT a fixed GPUI form, nor does
//! it own Project documents. Kinds compose the base element descriptors with
//! their specific descriptors and with existing typed JSON properties.
use crate::{Element, ModelingSection};
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertyEditor {
    Text,
    Multiline,
    Boolean,
    Number,
    Choice(&'static [&'static str]),
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

#[derive(Clone, Debug, PartialEq)]
pub struct ElementInspector {
    pub element_id: String,
    pub kind: String,
    pub name: String,
    pub display_label: String,
    pub properties: Vec<InspectorProperty>,
    /// Owned attributes/operations are model Elements, not strings in a form.
    pub attributes: Vec<InspectorMember>,
    pub operations: Vec<InspectorMember>,
    pub receptions: Vec<InspectorMember>,
    pub template_parameters: Vec<InspectorMember>,
    pub ports: Vec<InspectorMember>,
    pub parts: Vec<InspectorMember>,
}

const VISIBILITY: &[&str] = &["public", "protected", "private", "package"];

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
const FIELD: &[Declared] = &[
    Declared { key:"visibility", label:"Visibility", editor:PropertyEditor::Choice(VISIBILITY), default:public },
    Declared { key:"declaration", label:"Declaration", editor:PropertyEditor::Text, default:empty },
    Declared { key:"type", label:"Type", editor:PropertyEditor::Text, default:empty },
    Declared { key:"defaultValue", label:"Default value", editor:PropertyEditor::Text, default:empty },
    Declared { key:"isStatic", label:"Static", editor:PropertyEditor::Boolean, default:false_value },
    Declared { key:"isReadOnly", label:"Read only", editor:PropertyEditor::Boolean, default:false_value },
];
const FUNCTION: &[Declared] = &[
    Declared { key:"visibility", label:"Visibility", editor:PropertyEditor::Choice(VISIBILITY), default:public },
    Declared { key:"signature", label:"Signature", editor:PropertyEditor::Text, default:empty },
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
        "rust.function" | "uml.operation" => FUNCTION,
        _ => &[],
    }.iter())
}

fn inherited_value(element: &Element, key: &str, fallback: fn() -> Value) -> Value {
    element.properties.get(key).cloned().unwrap_or_else(fallback)
}

pub fn inspect_element(section: &ModelingSection, id: &str) -> Option<ElementInspector> {
    // Connections are Relations, not fake Elements inheriting Class properties.
    if let Some(relation) = section.relations.iter().find(|relation| relation.id == id
        && (relation.kind.starts_with("uml.") || section.diagrams.iter().any(|diagram|
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
        let common = [
            ("name","Name",PropertyEditor::Text,Value::String(String::new())),
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
        return Some(ElementInspector {
            element_id:relation.id.clone(),kind:relation.kind.clone(),
            name:format!("{source} → {target}"),
            display_label:format!("{source} → {target}"),
            properties,attributes:Vec::new(),operations:Vec::new(),receptions:Vec::new(),
            template_parameters:Vec::new(),ports:Vec::new(),parts:Vec::new(),
        });
    }
    let element = section.elements.iter().find(|element| element.id == id)?;
    let semantic_name = crate::template_parametered_element(section, element).map_or(element.name.as_str(), |exposed| exposed.name.as_str());
    let display_label = if crate::templates::is_parameter(&element.kind) { crate::template_parameter_label(section, element) } else { crate::element_display_label(element) };
    let mut properties = vec![
        InspectorProperty { path:"id".into(), label:"ID".into(), group:"Identity",
            editor:PropertyEditor::ReadOnly, value:json!(element.id), editable:false },
        InspectorProperty { path:"kind".into(), label:"Type".into(), group:"Identity",
            editor:PropertyEditor::ReadOnly, value:json!(element.kind), editable:false },
        InspectorProperty { path:"name".into(), label:"Name".into(), group:"Identity",
            editor:PropertyEditor::Text, value:json!(semantic_name), editable:true },
        InspectorProperty { path:"label".into(), label:"Label".into(), group:"Identity",
            editor:PropertyEditor::Text, value:json!(element.label.clone().unwrap_or_default()), editable:true },
    ];
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
    properties.push(InspectorProperty {path:"description".into(),label:"Documentation".into(),
        group:"Documentation",editor:PropertyEditor::Multiline,
        value:json!(element.description.clone().unwrap_or_default()),editable:true});

    let mut attributes = Vec::new();
    let mut operations = Vec::new();
    let mut receptions = Vec::new();
    let template_parameters = crate::template_parameters(section, &element.id).into_iter().map(|member| InspectorMember {
        id:member.id.clone(), name:crate::template_parametered_element(section, member).unwrap_or(member).name.clone(), kind:member.kind.clone(),
        display_label: crate::template_parameter_label(section, member),
    }).collect();
    let mut ports=Vec::new();
    let mut parts=Vec::new();
    for relation in &section.relations {
        if relation.kind != "owns" || relation.from != element.id {continue;}
        if let Some(member) = section.elements.iter().find(|e| e.id == relation.to) {
            let descriptor = InspectorMember {
                id:member.id.clone(),name:member.name.clone(),kind:member.kind.clone(),
                display_label:crate::element_display_label(member),
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
    Some(ElementInspector {element_id:element.id.clone(),kind:element.kind.clone(),
        name:semantic_name.into(),display_label,properties,attributes,operations,receptions,
        template_parameters,ports,parts})
}

/// Only described, editable properties may change; identities cannot be edited via a JSON path.
pub fn set_inspected_property(
    section: &mut ModelingSection, id: &str, path: &str, value: Value
) -> Result<(), String> {
    let inspected = inspect_element(section, id).ok_or("element not found")?;
    let field = inspected.properties.iter().find(|f| f.path == path)
        .ok_or("property is not declared")?;
    if !field.editable { return Err("property is read-only".into()); }
    match field.editor {
        PropertyEditor::ReadOnly => return Err("property is read-only".into()),
        PropertyEditor::Text | PropertyEditor::Multiline => {
            let s = value.as_str().ok_or("expected string")?;
            if path == "name" && s.trim().is_empty() {return Err("name may not be empty".into());}
        },
        PropertyEditor::Boolean if !value.is_boolean() => return Err("expected boolean".into()),
        PropertyEditor::Number if !value.is_number() => return Err("expected number".into()),
        PropertyEditor::Choice(options) if !value.as_str()
            .is_some_and(|candidate| options.contains(&candidate)) => {
            return Err("invalid option".into());
        },
        _ => {},
    }
    let diagram_relation = section.diagrams.iter().any(|diagram|
        diagram.views.iter().any(|view| view.subject.as_deref() == Some(id)));
    if let Some(relation) = section.relations.iter_mut().find(|r|
        r.id == id && (r.kind.starts_with("uml.") || diagram_relation)) {
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
