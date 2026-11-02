//! Target-specific Rust code projection from graph analysis into Tera data.
//! Semantic edges/ordering belong to bellatrix_mde_modeling. Tera handles
//! text only; publishing the resulting Plan is a separate explicit operation.
use bellatrix_mde_model::{Element, Project};
use bellatrix_mde_modeling::{GenerationModel, GenerationUnit, Plan, physical_path};
use bellatrix_mde_templates::TemplatePack;
use serde_json::json;
use std::{
    collections::{BTreeMap, HashSet},
    error::Error,
    path::PathBuf,
};

type GenResult<T> = Result<T, Box<dyn Error>>;

pub fn plan_rust(
    project: &Project,
    order: &GenerationModel,
    selected: &HashSet<String>,
    templates: &TemplatePack,
    plan: &mut Plan,
    skeleton: bool,
) -> GenResult<()> {
    let mut by_path: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
    for unit in &order.units {
        let Some(element) = project.element(&unit.id) else { continue };
        if !matches!(element.kind.as_str(),
            "rust.struct" | "rust.trait" | "rust.function" | "rust.item"
        ) {
            continue;
        }
        if element.kind == "rust.function"
            && let Some(parent) = owner(project, &element.id)
        {
            if selected.contains(&parent.id) { continue; }
            plan.warn(
                "rust.function.detached",
                "method generated without owner; snippet may not compile",
                Some(&element.id),
            );
        }
        let path = declaration_file(project, element)?;
        let rendered = match element.kind.as_str() {
            "rust.struct" => Some(render_struct(project, element, unit, selected, templates, plan, skeleton)?),
            "rust.trait" => Some(render_trait(project, element, unit, selected, templates, plan)?),
            "rust.item" => render_item(element, templates, plan)?,
            _ => render_function(element, false, templates, plan, skeleton)?,
        };
        if let Some(rendered) = rendered {
            by_path.entry(path).or_default().push(rendered);
        }
    }
    for (path, items) in by_path {
        plan.file(path, items.join("\n\n").into_bytes(), "rust declarations")?;
    }
    Ok(())
}

fn owner<'a>(project: &'a Project, id: &str) -> Option<&'a Element> {
    let relation = project.relations.iter()
        .find(|r| r.to == id && r.kind == "owns")?;
    let parent = project.element(&relation.from)?;
    matches!(parent.kind.as_str(), "rust.struct" | "rust.trait").then_some(parent)
}

fn declaration_file(project: &Project, element: &Element) -> GenResult<PathBuf> {
    if let Some(relation) = project.relations.iter()
        .find(|r| r.kind == "declares" && r.to == element.id)
    {
        let file = project.element(&relation.from)
            .ok_or("missing declaration file")?;
        if file.kind != "fs.file" {
            return Err("declaration parent must be fs.file".into());
        }
        return Ok(physical_path(project, &file.id)?);
    }
    let fallback: String = element.name.chars().map(|ch| {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            ch.to_ascii_lowercase()
        } else { '_' }
    }).collect();
    let filename = format!("{fallback}.rs");
    bellatrix_mde_modeling::validate_segment(&filename)?;
    Ok(PathBuf::from(filename))
}

/// Literal model source is a value, never evaluated as a second Tera template.
fn render_item(element: &Element, templates: &TemplatePack, plan: &mut Plan) -> GenResult<Option<String>> {
    let source = element.properties.get("source").and_then(|value| value.as_str())
        .or_else(|| element.implementations.iter()
            .find(|implementation| implementation.language == "rust")
            .map(|implementation| implementation.source.as_str()));
    match source {
        Some(source) => Ok(Some(templates.render("rust/raw_item.rs.tera",
            &json!({"annotations":rust_annotations(element),"source":source}))?)),
        None => {
            plan.warn("rust.item.source.missing", "item omitted: no source", Some(&element.id));
            Ok(None)
        }
    }
}

fn rust_annotations(element: &Element) -> String {
    element.annotations.iter()
        .filter(|annotation| annotation.language == "rust")
        .map(|annotation| format!("{}\n", annotation.source))
        .collect()
}

fn render_struct(
    project: &Project, element: &Element, unit: &GenerationUnit, selected: &HashSet<String>,
    templates: &TemplatePack, plan: &mut Plan, skeleton: bool,
) -> GenResult<String> {
    let mut fields = Vec::new();
    let mut methods = Vec::new();
    for child in children(project, &element.id, selected) {
        if child.kind == "rust.field" {
            if let Some(declaration) = child.properties.get("declaration").and_then(|v|v.as_str()) {
                fields.push(format!("{}{},",
                    rust_annotations(child).replace('\n', "\n    "),
                    declaration.trim_end_matches(',')
                ));
            } else {
                plan.warn("rust.field.declaration.missing",
                    "field omitted: no declaration", Some(&child.id));
            }
        } else if child.kind == "rust.function"
            && let Some(method) = render_function(child, false, templates, plan, skeleton)?
        {
            methods.push(indent(&method));
        }
    }
    Ok(templates.render("rust/struct.rs.tera", &json!({
        "annotations": rust_annotations(element),
        "name": &element.name,
        "fields": fields,
        "methods": methods,
        // Graph references are resolved by the MDE domain. A target adapter
        // can map these to imports using its own language/metamodel rules.
        "requires": &unit.requires,
    }))?)
}

fn render_trait(
    project: &Project, element: &Element, unit: &GenerationUnit, selected: &HashSet<String>,
    templates: &TemplatePack, plan: &mut Plan,
) -> GenResult<String> {
    let mut methods = Vec::new();
    for child in children(project, &element.id, selected) {
        if child.kind == "rust.function"
            && let Some(method) = render_function(child, true, templates, plan, false)?
        {
            methods.push(indent(&method));
        }
    }
    Ok(templates.render("rust/trait.rs.tera", &json!({
        "annotations": rust_annotations(element),
        "name": &element.name,
        "methods": methods,
        "requires": &unit.requires,
    }))?)
}

fn render_function(
    element: &Element, trait_member: bool, templates: &TemplatePack,
    plan: &mut Plan, skeleton: bool,
) -> GenResult<Option<String>> {
    let Some(signature) = element.properties.get("signature").and_then(|v|v.as_str()) else {
        plan.warn("rust.function.signature.missing",
            "function omitted: no Rust signature", Some(&element.id));
        return Ok(None);
    };
    let signature = signature.trim_end_matches(';');
    let body = element.implementations.iter()
        .find(|implementation| implementation.language == "rust");
    let annotations = rust_annotations(element);
    let rendered = match body {
        Some(implementation) => templates.render("rust/function.rs.tera", &json!({
            "annotations":annotations,
            "signature":signature,
            "body":indent(&implementation.source),
        }))?,
        None if trait_member => templates.render("rust/signature.rs.tera", &json!({
            "annotations":annotations, "signature":signature,
        }))?,
        None if skeleton => {
            plan.warn("rust.function.skeleton",
                "body unavailable; skeleton will not compile as a concrete method",
                Some(&element.id));
            templates.render("rust/signature.rs.tera", &json!({
                "annotations":annotations, "signature":signature,
            }))?
        }
        None => {
            plan.warn("rust.function.body.missing",
                "function omitted: no implementation", Some(&element.id));
            return Ok(None);
        }
    };
    Ok(Some(rendered))
}

fn children<'a>(
    project: &'a Project, id: &str, selected: &HashSet<String>,
) -> Vec<&'a Element> {
    project.relations.iter()
        .filter(|relation| relation.kind == "owns"
            && relation.from == id && selected.contains(&relation.to))
        .filter_map(|relation| project.element(&relation.to))
        .collect()
}

fn indent(source: &str) -> String {
    source.lines().map(|line| format!("    {line}"))
        .collect::<Vec<_>>().join("\n")
}
