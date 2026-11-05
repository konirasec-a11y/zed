use crate::uml::{
    references::ReferenceIndex,
    root::{ElementRef, Model},
    types::{MultiplicityRange, StandardPrimitiveType},
    value_specifications,
};
use crate::{Element, Project};
use serde_json::{Value, json};
use std::collections::HashSet;

pub(crate) fn project(model: &Project, element_id: &str) -> Result<Value, String> {
    let element = model
        .element(element_id)
        .ok_or_else(|| format!("unknown source element: {element_id}"))?;
    if element.kind != "uml.class" {
        return Err(format!("{element_id}: only UML Class has a struct mapping"));
    }
    supported_properties(element, false)?;
    let references = ReferenceIndex::new(Model::from_project(model));
    let children = references.explicit_owned_elements(element_id)?;
    let source_ids = std::iter::once(element.id.as_str())
        .chain(children.iter().map(|child| child.id()))
        .collect::<HashSet<_>>();
    let mut fields = Vec::new();
    let mut trace = vec![json!({
        "source_id": element.id, "target_kind": "rust.struct", "target_name": element.name,
    })];
    for child in children {
        let ElementRef::Object(field) = child else {
            return Err(format!(
                "{}: relationship members have no struct mapping",
                child.id()
            ));
        };
        if field.kind != "uml.attribute" {
            return Err(format!(
                "{}: only UML attributes have a field mapping",
                field.id
            ));
        }
        supported_properties(field, true)?;
        if !references.explicit_owned_elements(&field.id)?.is_empty() {
            return Err(format!(
                "{}: nested attribute elements have no field mapping",
                field.id
            ));
        }
        let multiplicity = value_specifications::multiplicity(&references, child)
            .map_err(|error| error.to_string())?;
        if multiplicity != MultiplicityRange::default() {
            return Err(format!(
                "{}: only multiplicity 1..1 has a field mapping",
                field.id
            ));
        }
        let declared_type = field.properties.get("type").unwrap_or(&Value::Null);
        let rust_type = match value_specifications::standard_primitive_reference(declared_type)? {
            Some(StandardPrimitiveType::Boolean) => "bool",
            Some(StandardPrimitiveType::String) => "::std::string::String",
            _ => {
                return Err(format!(
                    "{}: only source-qualified UML Boolean and String have a Rust type mapping",
                    field.id,
                ));
            }
        };
        fields.push(json!({
            "id": field.id, "kind": "rust.field", "name": field.name,
            "properties": {"declaration": format!("{}{}: {rust_type}", visibility(field)?, field.name)},
            "annotations": [], "implementations": [],
        }));
        trace.push(json!({
            "source_id": field.id, "target_kind": "rust.field", "target_name": field.name,
        }));
    }
    // A single declaration cannot silently discard inheritance, associations,
    // dependencies or behavioral relationships that would need another mapping.
    for relation in &model.relations {
        if relation.kind != "owns"
            && (source_ids.contains(relation.from.as_str())
                || source_ids.contains(relation.to.as_str()))
        {
            return Err(format!(
                "{}: {} has no standalone struct mapping",
                relation.id, relation.kind
            ));
        }
    }
    let path = crate::rust_emission::standalone_declaration_file(&element.name)
        .map_err(|error| error.to_string())?;
    let path = path.to_str().ok_or("artifact path is not UTF-8")?;
    Ok(json!({
        "path": path,
        "trace": trace,
        "emission": {
            "files": [], "manifests": [], "skeleton": false,
            "units": [{
                "element": {
                    "id": element.id, "kind": "rust.struct", "name": element.name,
                    "properties": {"visibility": visibility(element)?},
                    "annotations": [], "implementations": [],
                },
                "children": fields, "requires": [], "detached": false, "path": path,
            }],
        },
    }))
}

fn visibility(element: &Element) -> Result<&'static str, String> {
    match element.properties.get("visibility") {
        Some(Value::String(value)) if value == "public" => Ok("pub "),
        Some(Value::String(value)) if value == "private" => Ok(""),
        // A missing UML visibility is not a request to export a Rust symbol.
        None | Some(Value::Null) => Ok(""),
        _ => Err(format!(
            "{}: visibility has no standalone Rust mapping",
            element.id
        )),
    }
}

fn supported_properties(element: &Element, field: bool) -> Result<(), String> {
    if !element.annotations.is_empty()
        || !element.implementations.is_empty()
        || element.content.is_some()
    {
        return Err(format!(
            "{}: attached source has no UML struct mapping",
            element.id
        ));
    }
    for (name, value) in &element.properties {
        let supported = match name.as_str() {
            "visibility" => true,
            "type" | "multiplicity" if field => true,
            "isStatic" | "isReadOnly" | "isDerived" | "isDerivedUnion" | "isID" | "isOrdered"
                if field =>
            {
                value == false
            }
            "isUnique" if field => value == true,
            "aggregation" if field => value == "none",
            "isAbstract" | "isActive" | "isFinalSpecialization" | "isLeaf" if !field => {
                value == false
            }
            _ => false,
        };
        if !supported {
            return Err(format!(
                "{}: property {name} has no UML struct mapping",
                element.id
            ));
        }
    }
    Ok(())
}
