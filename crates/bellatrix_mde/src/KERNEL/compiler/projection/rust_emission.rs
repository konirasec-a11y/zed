use crate::{
    Element, Graph, Project, Scope, analyze_generation_graph, physical_path, select_elements,
    validate_segment,
};
use serde_json::{Value, json};
use std::{error::Error, path::PathBuf};

// These are read-only emission tasks, not another model. Selection, dependency
// order and placement must be resolved against the authoritative graph here.
pub(crate) fn project(
    model: &Project,
    scope: &Scope,
    skeleton: bool,
) -> Result<Value, Box<dyn Error>> {
    Graph::new(model).validate()?;
    let selected = select_elements(model, scope)?;
    let generation = analyze_generation_graph(model, &selected)?;
    let mut files = Vec::new();
    let mut manifests = Vec::new();
    for element in &model.elements {
        if !selected.contains(&element.id) {
            continue;
        }
        match element.kind.as_str() {
            "fs.directory" => files.push(json!({
                "path": path_text(physical_path(model, &element.id)?)?,
                "contents": null, "origin": element.id,
            })),
            "fs.file" => {
                // Empty placement placeholders must not collide with the source
                // artifact emitted for their selected declarations.
                if element.content.is_none()
                    && model.relations.iter().any(|relation| {
                        relation.kind == "declares"
                            && relation.from == element.id
                            && selected.contains(&relation.to)
                    })
                {
                    continue;
                }
                let bytes = match &element.content {
                    Some(content) => content.bytes()?,
                    None => Vec::new(),
                };
                files.push(json!({
                    "path": path_text(physical_path(model, &element.id)?)?,
                    "contents": bytes, "origin": element.id,
                }));
            }
            "cargo.package" | "cargo.workspace" => manifests.push(json!({
                "element": target_element(element),
                "path": path_text(physical_path(model, &element.id)?.join("Cargo.toml"))?,
            })),
            _ => {}
        }
    }
    let mut units = Vec::new();
    for unit in &generation.units {
        let element = model.element(&unit.id).ok_or("missing generation element")?;
        if !matches!(
            element.kind.as_str(),
            "rust.struct" | "rust.trait" | "rust.function" | "rust.item"
        ) {
            continue;
        }
        let parent = model.relations.iter()
            .find(|relation| relation.to == element.id && relation.kind == "owns")
            .and_then(|relation| model.element(&relation.from))
            .filter(|parent| matches!(parent.kind.as_str(), "rust.struct" | "rust.trait"));
        let detached = element.kind == "rust.function" && parent.is_some();
        if detached && parent.is_some_and(|parent| selected.contains(&parent.id)) {
            continue;
        }
        let children = model.relations.iter()
            .filter(|relation| relation.kind == "owns" && relation.from == element.id
                && selected.contains(&relation.to))
            .filter_map(|relation| model.element(&relation.to))
            .map(target_element).collect::<Vec<_>>();
        units.push(json!({
            "element": target_element(element), "children": children,
            "requires": unit.requires, "detached": detached,
            "path": path_text(declaration_file(model, element)?)?,
        }));
    }
    Ok(json!({"files": files, "manifests": manifests, "units": units, "skeleton": skeleton}))
}

fn target_element(element: &Element) -> Value {
    json!({
        "id": element.id, "kind": element.kind, "name": element.name,
        "properties": element.properties,
        "annotations": element.annotations, "implementations": element.implementations,
    })
}

fn declaration_file(model: &Project, element: &Element) -> Result<PathBuf, Box<dyn Error>> {
    if let Some(relation) = model.relations.iter()
        .find(|relation| relation.kind == "declares" && relation.to == element.id)
    {
        let file = model.element(&relation.from).ok_or("missing declaration file")?;
        if file.kind != "fs.file" {
            return Err("declaration parent must be fs.file".into());
        }
        return Ok(physical_path(model, &file.id)?);
    }
    let fallback: String = element.name.chars().map(|character| {
        if character.is_ascii_alphanumeric() || character == '_' {
            character.to_ascii_lowercase()
        } else {
            '_'
        }
    }).collect();
    let filename = format!("{fallback}.rs");
    validate_segment(&filename)?;
    Ok(PathBuf::from(filename))
}

fn path_text(path: PathBuf) -> Result<String, Box<dyn Error>> {
    path.into_os_string().into_string().map_err(|_| "artifact path is not UTF-8".into())
}
