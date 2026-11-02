use bellatrix_mde::{Element, Project, Plan, physical_path};
use bellatrix_mde_templates::TemplatePack;
use serde_json::Value as Json;
use std::collections::HashSet;
use toml::{Table, Value};

pub fn plan_manifests(project: &Project, selected: &HashSet<String>, plan: &mut Plan,
    templates: &TemplatePack) -> Result<(), Box<dyn std::error::Error>> {
    for element in &project.elements {
        if !selected.contains(&element.id) { continue; }
        match element.kind.as_str() {
            "cargo.package" | "cargo.workspace" => {
                let directory = physical_path(project, &element.id)?;
                let table = manifest(element, plan)?;
                plan.file(directory.join("Cargo.toml"), templates.render("rust/Cargo.toml.tera",
                    &serde_json::json!({"manifest": toml::to_string_pretty(&table)?}))?.into_bytes(), &element.id)?;
            }
            _ => (),
        }
    }
    Ok(())
}
fn manifest(element: &Element, plan: &mut Plan) -> Result<Table, Box<dyn std::error::Error>> {
    if let Some(raw) = element.properties.get("manifest") {
        return match convert(raw)? { Value::Table(table) => Ok(table), _ => Err("manifest must be a TOML-compatible object".into()) };
    }
    let mut table = Table::new();
    if element.kind == "cargo.package" {
        let mut package = Table::new();
        package.insert("name".into(), Value::String(element.name.clone()));
        if let Some(version) = element.properties.get("version").and_then(Json::as_str) {
            package.insert("version".into(), Value::String(version.into()));
        } else { plan.warn("cargo.version.missing", "package has no version; Cargo may reject it", Some(&element.id)); }
        for field in ["edition", "description", "license", "repository", "publish", "authors", "build"] {
            if let Some(value) = element.properties.get(field) { package.insert(field.to_string(), convert(value)?); }
        }
        table.insert("package".into(), Value::Table(package));
    } else {
        let members = element.properties.get("members").ok_or("cargo.workspace requires a members array")?;
        let mut workspace = Table::new();
        workspace.insert("members".into(), convert(members)?);
        if let Some(value) = element.properties.get("resolver") { workspace.insert("resolver".into(), convert(value)?); }
        table.insert("workspace".into(), Value::Table(workspace));
    }
    for field in ["dependencies", "dev-dependencies", "build-dependencies", "features", "lib", "bin", "lints", "profile", "workspace"] {
        if let Some(value) = element.properties.get(field) { table.insert(field.to_string(), convert(value)?); }
    }
    Ok(table)
}
fn convert(input: &Json) -> Result<Value, Box<dyn std::error::Error>> {
    match input {
        Json::Null => Err("TOML has no null; omit the property".into()),
        Json::Bool(value) => Ok(Value::Boolean(*value)),
        Json::String(value) => Ok(Value::String(value.clone())),
        Json::Number(value) => {
            if let Some(number) = value.as_i64() { Ok(Value::Integer(number)) }
            else if let Some(number) = value.as_f64() { Ok(Value::Float(number)) }
            else { Err("unsupported numeric TOML value".into()) }
        }
        Json::Array(values) => Ok(Value::Array(values.iter().map(convert).collect::<Result<Vec<_>, _>>()?)),
        Json::Object(values) => {
            let mut table = Table::new();
            for (key, value) in values { table.insert(key.clone(), convert(value)?); }
            Ok(Value::Table(table))
        }
    }
}
