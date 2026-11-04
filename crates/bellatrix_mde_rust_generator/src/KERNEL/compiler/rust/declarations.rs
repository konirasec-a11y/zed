use super::emission::{Rendered, TargetElement, Unit};
use bellatrix_mde_templates::TemplatePack;
use serde_json::json;
use std::{collections::BTreeMap, error::Error};

type GenResult<T> = Result<T, Box<dyn Error>>;

pub(super) fn render_rust(units: &[Unit], templates: &TemplatePack,
    rendered: &mut Rendered, skeleton: bool) -> GenResult<()> {
    let mut by_path: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for unit in units {
        let element = &unit.element;
        if unit.detached {
            rendered.warn("rust.function.detached", "method generated without owner; snippet may not compile", Some(&element.id));
        }
        let source = match element.kind.as_str() {
            "rust.struct" => Some(render_struct(element, unit, templates, rendered, skeleton)?),
            "rust.trait" => Some(render_trait(element, unit, templates, rendered)?),
            "rust.item" => render_item(element, templates, rendered)?,
            "rust.function" => render_function(element, false, templates, rendered, skeleton)?,
            _ => return Err("unsupported Rust emission unit".into()),
        };
        if let Some(source) = source { by_path.entry(unit.path.clone()).or_default().push(source); }
    }
    for (path, items) in by_path { rendered.file(path, items.join("\n\n").into_bytes(), "rust declarations"); }
    Ok(())
}
/// Literal source is a value, never evaluated as another Tera template.
fn render_item(element: &TargetElement, templates: &TemplatePack, plan: &mut Rendered) -> GenResult<Option<String>> {
    let source = element.properties.get("source").and_then(|value| value.as_str())
        .or_else(|| element.implementations.iter().find(|implementation| implementation.language == "rust")
            .map(|implementation| implementation.source.as_str()));
    match source {
        Some(source) => Ok(Some(templates.render("rust/raw_item.rs.tera", &json!({"annotations": rust_annotations(element), "source": source}))?)),
        None => { plan.warn("rust.item.source.missing", "item omitted: no source", Some(&element.id)); Ok(None) }
    }
}
fn rust_annotations(element: &TargetElement) -> String {
    element.annotations.iter().filter(|annotation| annotation.language == "rust")
        .map(|annotation| format!("{}\n", annotation.source)).collect()
}
fn render_struct(element: &TargetElement, unit: &Unit,
    templates: &TemplatePack, plan: &mut Rendered, skeleton: bool) -> GenResult<String> {
    let mut fields = Vec::new(); let mut methods = Vec::new();
    for child in &unit.children {
        if child.kind == "rust.field" {
            if let Some(declaration) = child.properties.get("declaration").and_then(|value| value.as_str()) {
                fields.push(format!("{}{},", rust_annotations(child).replace('\n', "\n    "), declaration.trim_end_matches(',')));
            } else { plan.warn("rust.field.declaration.missing", "field omitted: no declaration", Some(&child.id)); }
        } else if child.kind == "rust.function" && let Some(method) = render_function(child, false, templates, plan, skeleton)? {
            methods.push(indent(&method));
        }
    }
    Ok(templates.render("rust/struct.rs.tera", &json!({
        "annotations": rust_annotations(element), "name": &element.name,
        "fields": fields, "methods": methods, "requires": &unit.requires,
    }))?)
}
fn render_trait(element: &TargetElement, unit: &Unit,
    templates: &TemplatePack, plan: &mut Rendered) -> GenResult<String> {
    let mut methods = Vec::new();
    for child in &unit.children {
        if child.kind == "rust.function" && let Some(method) = render_function(child, true, templates, plan, false)? {
            methods.push(indent(&method));
        }
    }
    Ok(templates.render("rust/trait.rs.tera", &json!({
        "annotations": rust_annotations(element), "name": &element.name, "methods": methods, "requires": &unit.requires,
    }))?)
}
fn render_function(element: &TargetElement, trait_member: bool, templates: &TemplatePack, plan: &mut Rendered,
    skeleton: bool) -> GenResult<Option<String>> {
    let Some(signature) = element.properties.get("signature").and_then(|value| value.as_str()) else {
        plan.warn("rust.function.signature.missing", "function omitted: no Rust signature", Some(&element.id)); return Ok(None);
    };
    let signature = signature.trim_end_matches(';');
    let body = element.implementations.iter().find(|implementation| implementation.language == "rust");
    let annotations = rust_annotations(element);
    let rendered = match body {
        Some(implementation) => templates.render("rust/function.rs.tera", &json!({
            "annotations": annotations, "signature": signature, "body": indent(&implementation.source),
        }))?,
        None if trait_member => templates.render("rust/signature.rs.tera", &json!({"annotations": annotations, "signature": signature}))?,
        None if skeleton => {
            plan.warn("rust.function.skeleton", "body unavailable; skeleton will not compile as a concrete method", Some(&element.id));
            templates.render("rust/signature.rs.tera", &json!({"annotations": annotations, "signature": signature}))?
        }
        None => { plan.warn("rust.function.body.missing", "function omitted: no implementation", Some(&element.id)); return Ok(None); }
    };
    Ok(Some(rendered))
}
fn indent(source: &str) -> String { source.lines().map(|line| format!("    {line}")).collect::<Vec<_>>().join("\n") }
