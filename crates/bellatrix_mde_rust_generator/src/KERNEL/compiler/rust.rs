//! Graph-directed Rust/Cargo artifacts. This is not a Rust compiler.
#[path = "cargo/manifest.rs"]
mod cargo;
#[path = "rust/declarations.rs"]
mod declarations;
#[path = "artifacts/files.rs"]
mod files;
use bellatrix_mde::{Project, Graph, Plan, Scope, analyze_generation_graph, select_elements};
use bellatrix_mde_templates::TemplatePack;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GenerationOptions { pub skeleton: bool }

pub fn plan(project: &Project, scope: &Scope) -> Result<Plan, Box<dyn std::error::Error>> {
    plan_with_options(project, scope, GenerationOptions::default())
}
pub fn default_templates() -> Result<TemplatePack, Box<dyn std::error::Error>> {
    Ok(TemplatePack::embedded([
        ("rust/struct.rs.tera", include_str!("rust/templates/struct.rs.tera")),
        ("rust/trait.rs.tera", include_str!("rust/templates/trait.rs.tera")),
        ("rust/function.rs.tera", include_str!("rust/templates/function.rs.tera")),
        ("rust/signature.rs.tera", include_str!("rust/templates/signature.rs.tera")),
        ("rust/raw_item.rs.tera", include_str!("rust/templates/raw_item.rs.tera")),
        ("rust/Cargo.toml.tera", include_str!("rust/templates/Cargo.toml.tera")),
    ])?)
}
pub fn plan_with_options(project: &Project, scope: &Scope, options: GenerationOptions) -> Result<Plan, Box<dyn std::error::Error>> {
    let templates = default_templates()?;
    plan_with_templates(project, scope, options, &templates)
}
/// Validate/select, analyze, render, and return an immutable publication plan.
pub fn plan_with_templates(project: &Project, scope: &Scope, options: GenerationOptions,
    templates: &TemplatePack) -> Result<Plan, Box<dyn std::error::Error>> {
    plan_with_templates_inner(project, scope, options, templates, None)
}

fn plan_with_templates_inner(project: &Project, scope: &Scope, options: GenerationOptions,
    templates: &TemplatePack, mut trace: Option<&mut bellatrix_mde::ExecutionTrace>) -> Result<Plan, Box<dyn std::error::Error>> {
    match trace.as_deref_mut() {
        Some(trace) => Graph::new(project).validate_traced(trace)?,
        None => Graph::new(project).validate()?,
    }
    let selected = select_elements(project, scope)?;
    let generation_model = match trace {
        Some(trace) => bellatrix_mde::analyze_generation_graph_traced(project, &selected, trace)?,
        None => analyze_generation_graph(project, &selected)?,
    };
    let mut result = Plan::new();
    files::plan_files(project, &selected, &mut result)?;
    cargo::plan_manifests(project, &selected, &mut result, templates)?;
    declarations::plan_rust(project, &generation_model, &selected, templates, &mut result, options.skeleton)?;
    Ok(result)
}
/// The same planner with observed kernel and generation stages recorded in a caller-owned run.
pub fn plan_with_options_traced(project: &Project, scope: &Scope, options: GenerationOptions, trace: &mut bellatrix_mde::ExecutionTrace) -> Result<Plan, Box<dyn std::error::Error>> {
    use bellatrix_mde::{TracePhase, TraceOutcome};
    trace.emit("generation", "rust.plan", TracePhase::Start, Some(&project.id), &[], None);
    let result = (|| {
        let templates = default_templates()?;
        plan_with_templates_inner(project, scope, options, &templates, Some(trace))
    })();
    let (phase, outcome) = if result.is_ok() { (TracePhase::Finish, TraceOutcome::Success) } else { (TracePhase::Error, TraceOutcome::Error) };
    trace.emit("generation", "rust.plan", phase, Some(&project.id), &[], Some(outcome));
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Project { serde_json::from_str(include_str!("../../../../bellatrix_mde/fixtures/todo.json")).unwrap() }
    #[test]
    fn tera_is_active_and_can_be_replaced_per_target_without_graph_mutation() {
        let project = fixture();
        let baseline = plan(&project, &Scope::Project).unwrap();
        let mut templates = default_templates().unwrap();
        assert!(templates.contains("rust/struct.rs.tera"));
        templates.override_template("rust/struct.rs.tera",
            "{{ annotations }}pub struct {{ name }} { {% for field in fields %}{{ field }} {% endfor %}}{% for dep in requires %}\n// depends on {{ dep.name }} ({{ dep.id }}){% endfor %}"
        ).unwrap();
        let changed = plan_with_templates(&project, &Scope::Project, GenerationOptions::default(), &templates).unwrap();
        let source = |plan: &Plan| String::from_utf8(plan.artifacts().find(|artifact| artifact.path.to_string_lossy().ends_with("task.rs"))
            .unwrap().contents.as_ref().unwrap().clone()).unwrap();
        assert_ne!(source(&baseline), source(&changed));
        assert!(source(&changed).contains("pub struct TaskList"));
        assert!(source(&changed).contains("// depends on Task (type-task)"));
        assert_eq!(project.relations.iter().filter(|relation| relation.kind == "depends_on").count(), 1);
    }
    #[test]
    fn invalid_target_template_aborts_before_publication() {
        let pack = TemplatePack::embedded([("rust/struct.rs.tera", "{{ name }}")]).unwrap();
        assert!(plan_with_templates(&fixture(), &Scope::Project, GenerationOptions::default(), &pack).is_err());
    }
    #[test]
    fn code_generation_uses_graph_order_and_valid_toml_without_mutating_source() {
        let project = fixture(); let original = project.clone();
        let result = plan(&project, &Scope::Project).unwrap();
        let file = |path: &str| String::from_utf8(result.artifacts().find(|artifact| artifact.path.to_string_lossy() == path)
            .unwrap().contents.as_ref().unwrap().clone()).unwrap();
        let rust = file("src/task.rs");
        assert!(rust.find("pub struct Task {").unwrap() < rust.find("pub struct TaskList {").unwrap());
        assert!(rust.contains("pub fn add(")); assert!(rust.contains("pub tasks: Vec<Task>"));
        let manifest = file("Cargo.toml");
        let toml: toml::Table = toml::from_str(&manifest).unwrap();
        assert_eq!(toml["package"]["name"].as_str(), Some("todo_example"));
        assert_eq!(project, original, "analysis and Tera rendering are pure");
    }
    #[test]
    fn fixture_has_real_rust_and_arbitrary_assets() {
        let planned = plan(&fixture(), &Scope::Project).unwrap();
        let artifacts = planned.artifacts().collect::<Vec<_>>();
        assert!(artifacts.iter().any(|artifact| artifact.path.to_string_lossy() == "Cargo.toml"));
        assert!(artifacts.iter().any(|artifact| artifact.path.to_string_lossy() == "clippy.toml"));
        let file = artifacts.iter().find(|artifact| artifact.path.to_string_lossy().ends_with("task.rs")).unwrap();
        let source = String::from_utf8(file.contents.clone().unwrap()).unwrap();
        assert!(source.contains("pub struct Task")); assert!(!source.contains("todo!()")); assert!(!source.contains("unimplemented!()"));
    }
}
#[cfg(test)]
mod adversarial_tests {
    use super::*;
    use bellatrix_mde::{Element, FileContent};
    fn fixture() -> Project { serde_json::from_str(include_str!("../../../../bellatrix_mde/fixtures/todo.json")).unwrap() }
    #[test]
    fn selected_function_generates_without_entire_project() {
        let result = plan(&fixture(), &Scope::Elements(vec!["fn-add".into()])).unwrap();
        assert_eq!(result.count(), 1);
        assert!(result.diagnostics.iter().any(|diagnostic| diagnostic.code == "rust.function.detached"));
    }
    #[test]
    fn literal_and_generated_manifest_must_not_collide() {
        let mut project = fixture();
        project.elements.push(Element { id: "literal-manifest".into(), kind: "fs.file".into(), name: "Cargo.toml".into(),
            label: None, description: None, properties: Default::default(), annotations: vec![], implementations: vec![],
            content: Some(FileContent::Text("[package]".into())) });
        assert!(plan(&project, &Scope::Project).is_err());
    }
    #[test]
    fn binary_bytes_are_preserved() {
        let result = plan(&fixture(), &Scope::Project).unwrap();
        let logo = result.artifacts().find(|artifact| artifact.path.to_string_lossy().ends_with("logo.bin")).unwrap();
        assert_eq!(logo.contents.as_deref(), Some(&[0, 1, 2, 3, 255, 0, 127][..]));
    }
}
#[cfg(test)]
mod completion_tests {
    use super::*;
    fn fixture() -> Project { serde_json::from_str(include_str!("../../../../bellatrix_mde/fixtures/skeleton.json")).unwrap() }
    #[test]
    fn default_generation_omits_missing_body_and_respects_root_file() {
        let result = plan(&fixture(), &Scope::Project).unwrap();
        let source = result.artifacts().find(|artifact| artifact.path.to_string_lossy() == "domain.rs").unwrap().contents.as_ref().unwrap();
        let code = String::from_utf8(source.clone()).unwrap();
        assert!(code.contains("pub struct Task")); assert!(code.contains("pub enum Status"));
        assert!(!code.contains("fn complete")); assert!(!code.contains("todo!"));
    }
    #[test]
    fn explicit_skeleton_keeps_signature_without_fabricating_body() {
        let plan = plan_with_options(&fixture(), &Scope::Project, GenerationOptions { skeleton: true }).unwrap();
        let source = plan.artifacts().find(|artifact| artifact.path.to_string_lossy() == "domain.rs").unwrap().contents.as_ref().unwrap();
        let code = String::from_utf8(source.clone()).unwrap();
        assert!(code.contains("pub fn complete(&mut self);"));
        assert!(plan.diagnostics.iter().any(|diagnostic| diagnostic.code == "rust.function.skeleton"));
    }
}
