use bellatrix_mde_model::{Element, Project};
use bellatrix_mde_modeling::{Plan, physical_path};
use std::{
    collections::{BTreeMap, HashSet},
    path::PathBuf,
};

pub fn plan_rust(
    project: &Project,
    selected: &HashSet<String>,
    plan: &mut Plan,
    skeleton: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut by_path: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
    for element in &project.elements {
        if !selected.contains(&element.id)
            || !matches!(
                element.kind.as_str(),
                "rust.struct" | "rust.trait" | "rust.function" | "rust.item"
            )
        {
            continue;
        }
        if element.kind == "rust.function"
            && let Some(parent) = owner(project, &element.id)
        {
            if selected.contains(&parent.id) {
                continue;
            }
            plan.warn(
                "rust.function.detached",
                "method generated without owner; snippet may not compile",
                Some(&element.id),
            );
        }
        let path = declaration_file(project, element)?;
        let rendered = match element.kind.as_str() {
            "rust.struct" => Some(render_struct(project, element, selected, plan, skeleton)),
            "rust.trait" => Some(render_trait(project, element, selected, plan)),
            "rust.item" => render_item(element, plan),
            _ => render_function(element, false, plan, skeleton),
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
    let relation = project
        .relations
        .iter()
        .find(|r| r.to == id && r.kind == "owns")?;
    let parent = project.element(&relation.from)?;
    if matches!(parent.kind.as_str(), "rust.struct" | "rust.trait") {
        Some(parent)
    } else {
        None
    }
}

fn declaration_file(
    project: &Project,
    element: &Element,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(file_relation) = project
        .relations
        .iter()
        .find(|r| r.kind == "declares" && r.to == element.id)
    {
        let file = project
            .element(&file_relation.from)
            .ok_or("missing declaration file")?;
        if file.kind != "fs.file" {
            return Err("declaration parent must be fs.file".into());
        }
        return Ok(physical_path(project, &file.id)?);
    }
    let fallback: String = element
        .name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    let filename = format!("{fallback}.rs");
    bellatrix_mde_modeling::validate_segment(&filename)?;
    Ok(PathBuf::from(filename))
}

/// Preserves target-specific source literally; the Rust compiler validates its syntax.
fn render_item(element: &Element, plan: &mut Plan) -> Option<String> {
    let source = element
        .properties
        .get("source")
        .and_then(|v| v.as_str())
        .or_else(|| {
            element
                .implementations
                .iter()
                .find(|i| i.language == "rust")
                .map(|i| i.source.as_str())
        });
    match source {
        Some(text) => Some(format!("{}{}", rust_annotations(element), text)),
        None => {
            plan.warn(
                "rust.item.source.missing",
                "item omitted: no source",
                Some(&element.id),
            );
            None
        }
    }
}

fn rust_annotations(element: &Element) -> String {
    element
        .annotations
        .iter()
        .filter(|a| a.language == "rust")
        .map(|a| format!("{}\n", a.source))
        .collect::<String>()
}

fn render_struct(
    project: &Project,
    element: &Element,
    selected: &HashSet<String>,
    plan: &mut Plan,
    skeleton: bool,
) -> String {
    let mut out = format!(
        "{}pub struct {} {{\n",
        rust_annotations(element),
        element.name
    );
    for child in children(project, &element.id, selected) {
        if child.kind != "rust.field" {
            continue;
        }
        if let Some(decl) = child.properties.get("declaration").and_then(|v| v.as_str()) {
            out.push_str(&format!(
                "    {}{},\n",
                rust_annotations(child).replace('\n', "\n    "),
                decl.trim_end_matches(',')
            ));
        } else {
            plan.warn(
                "rust.field.declaration.missing",
                "field omitted: no declaration",
                Some(&child.id),
            );
        }
    }
    out.push_str("}\n");
    let mut methods = Vec::new();
    for child in children(project, &element.id, selected) {
        if child.kind == "rust.function"
            && let Some(m) = render_function(child, false, plan, skeleton)
        {
            methods.push(m);
        }
    }
    if !methods.is_empty() {
        out.push_str(&format!(
            "\nimpl {} {{\n{}\n}}\n",
            element.name,
            methods
                .iter()
                .map(|s| indent(s))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    out
}

fn render_trait(
    project: &Project,
    element: &Element,
    selected: &HashSet<String>,
    plan: &mut Plan,
) -> String {
    let mut parts = Vec::new();
    for child in children(project, &element.id, selected) {
        if child.kind == "rust.function"
            && let Some(s) = render_function(child, true, plan, false)
        {
            parts.push(indent(&s));
        }
    }
    format!(
        "{}pub trait {} {{\n{}\n}}",
        rust_annotations(element),
        element.name,
        parts.join("\n")
    )
}

fn render_function(
    element: &Element,
    trait_member: bool,
    plan: &mut Plan,
    skeleton: bool,
) -> Option<String> {
    let signature = element.properties.get("signature").and_then(|v| v.as_str());
    let Some(signature) = signature else {
        plan.warn(
            "rust.function.signature.missing",
            "function omitted: no Rust signature",
            Some(&element.id),
        );
        return None;
    };
    let source = element
        .implementations
        .iter()
        .find(|i| i.language == "rust");
    let prefix = rust_annotations(element);
    match source {
        Some(body) => Some(format!(
            "{prefix}{} {{\n{}\n}}",
            signature.trim_end_matches(';'),
            indent(&body.source)
        )),
        None if trait_member => Some(format!("{prefix}{};", signature.trim_end_matches(';'))),
        None if skeleton => {
            plan.warn(
                "rust.function.skeleton",
                "body unavailable; skeleton will not compile as a concrete method",
                Some(&element.id),
            );
            Some(format!("{prefix}{};", signature.trim_end_matches(';')))
        }
        None => {
            plan.warn(
                "rust.function.body.missing",
                "function omitted: no implementation",
                Some(&element.id),
            );
            None
        }
    }
}

fn children<'a>(project: &'a Project, id: &str, selected: &HashSet<String>) -> Vec<&'a Element> {
    project
        .relations
        .iter()
        .filter(|r| r.kind == "owns" && r.from == id && selected.contains(&r.to))
        .filter_map(|r| project.element(&r.to))
        .collect()
}
fn indent(source: &str) -> String {
    source
        .lines()
        .map(|line| format!("    {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}
