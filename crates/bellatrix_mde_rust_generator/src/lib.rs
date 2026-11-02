//! Graph-directed Rust/Cargo artifacts. This is not a Rust compiler.
mod cargo;
mod declarations;
mod files;

use bellatrix_mde_model::Project;
use bellatrix_mde_modeling::{Graph, Plan, Scope, analyze_generation_graph, select_elements};
use bellatrix_mde_templates::TemplatePack;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GenerationOptions {
    /// Preserve declarations without bodies as invalid-but-explicit Rust skeletons.
    pub skeleton: bool,
}

pub fn plan(project: &Project, scope: &Scope) -> Result<Plan, Box<dyn std::error::Error>> {
    plan_with_options(project, scope, GenerationOptions::default())
}

/// Built-in Rust template pack. Third-party engines can provide independent
/// packs or override individual target templates without changing graph rules.
pub fn default_templates() -> Result<TemplatePack, Box<dyn std::error::Error>> {
    Ok(TemplatePack::embedded([
        ("rust/struct.rs.tera", include_str!("../templates/struct.rs.tera")),
        ("rust/trait.rs.tera", include_str!("../templates/trait.rs.tera")),
        ("rust/function.rs.tera", include_str!("../templates/function.rs.tera")),
        ("rust/signature.rs.tera", include_str!("../templates/signature.rs.tera")),
        ("rust/raw_item.rs.tera", include_str!("../templates/raw_item.rs.tera")),
        ("rust/Cargo.toml.tera", include_str!("../templates/Cargo.toml.tera")),
    ])?)
}

pub fn plan_with_options(
    project: &Project,
    scope: &Scope,
    options: GenerationOptions,
) -> Result<Plan, Box<dyn std::error::Error>> {
    let templates = default_templates()?;
    plan_with_templates(project, scope, options, &templates)
}

/// Four separate stages: validate and select the source graph; build an
/// in-memory generation model with petgraph; render text with Tera; return
/// an immutable Plan for explicit, ownership-checked publication.
pub fn plan_with_templates(
    project: &Project,
    scope: &Scope,
    options: GenerationOptions,
    templates: &TemplatePack,
) -> Result<Plan, Box<dyn std::error::Error>> {
    Graph::new(project).validate()?;
    let selected = select_elements(project, scope)?;
    let generation_model = analyze_generation_graph(project, &selected)?;
    let mut result = Plan::new();
    files::plan_files(project, &selected, &mut result)?;
    cargo::plan_manifests(project, &selected, &mut result, templates)?;
    declarations::plan_rust(
        project, &generation_model, &selected, templates,
        &mut result, options.skeleton,
    )?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tera_is_active_and_can_be_replaced_per_target_without_graph_mutation() {
        let project: Project =
            serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap();
        let baseline = plan(&project, &Scope::Project).unwrap();
        let mut templates = default_templates().unwrap();
        assert!(templates.contains("rust/struct.rs.tera"));
        templates.override_template("rust/struct.rs.tera",
            "{{ annotations }}pub struct {{ name }} { {% for field in fields %}{{ field }} {% endfor %}}"
        ).unwrap();
        let changed = plan_with_templates(&project, &Scope::Project,
            GenerationOptions::default(), &templates).unwrap();
        let src = |plan: &Plan| String::from_utf8(
            plan.artifacts().find(|a|a.path.to_string_lossy().ends_with("task.rs"))
                .unwrap().contents.as_ref().unwrap().clone()
        ).unwrap();
        assert_ne!(src(&baseline), src(&changed));
        assert!(src(&changed).contains("pub struct TaskList"));
        assert_eq!(project.relations.iter().filter(|r|r.kind=="depends_on").count(), 1);
    }
    #[test]
    fn invalid_target_template_aborts_before_publication() {
        let project: Project =
            serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap();
        let pack = TemplatePack::embedded([("rust/struct.rs.tera", "{{ name }}")]).unwrap();
        assert!(plan_with_templates(&project, &Scope::Project,
            GenerationOptions::default(), &pack).is_err());
    }

    #[test]
    fn code_generation_uses_graph_order_and_valid_toml_without_mutating_source() {
        let project: Project =
            serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap();
        let original = project.clone();
        let result = plan(&project, &Scope::Project).unwrap();
        let file = |path: &str| String::from_utf8(
            result.artifacts().find(|a| a.path.to_string_lossy()==path)
                .unwrap().contents.as_ref().unwrap().clone()
        ).unwrap();
        let rust = file("src/task.rs");
        assert!(rust.find("pub struct Task {").unwrap()
            < rust.find("pub struct TaskList {").unwrap());
        assert!(rust.contains("pub fn add("));
        assert!(rust.contains("pub tasks: Vec<Task>"));
        let manifest = file("Cargo.toml");
        let toml: toml::Table = toml::from_str(&manifest).unwrap();
        assert_eq!(toml["package"]["name"].as_str(), Some("todo_example"));
        assert_eq!(project, original, "analysis and Tera rendering are pure");
    }

    #[test]
    fn fixture_has_real_rust_and_arbitrary_assets() {
        let project: Project =
            serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap();
        let planned = plan(&project, &Scope::Project).unwrap();
        let artifacts = planned.artifacts().collect::<Vec<_>>();
        assert!(
            artifacts
                .iter()
                .any(|a| a.path.to_string_lossy() == "Cargo.toml")
        );
        assert!(
            artifacts
                .iter()
                .any(|a| a.path.to_string_lossy() == "clippy.toml")
        );
        let f = artifacts
            .iter()
            .find(|a| a.path.to_string_lossy().ends_with("task.rs"))
            .unwrap();
        let source = String::from_utf8(f.contents.clone().unwrap()).unwrap();
        assert!(source.contains("pub struct Task"));
        assert!(!source.contains("todo!()"));
        assert!(!source.contains("unimplemented!()"));
    }
}

#[cfg(test)]
mod adversarial_tests {
    use super::*;
    use bellatrix_mde_model::{Element, FileContent};
    #[test]
    fn selected_function_generates_without_entire_project() {
        let project: Project =
            serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap();
        let result = plan(&project, &Scope::Elements(vec!["fn-add".into()])).unwrap();
        assert_eq!(result.count(), 1);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "rust.function.detached")
        );
    }
    #[test]
    fn literal_and_generated_manifest_must_not_collide() {
        let mut project: Project =
            serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap();
        project.elements.push(Element {
            id: "literal-manifest".into(),
            kind: "fs.file".into(),
            name: "Cargo.toml".into(),
            label: None,
            description: None,
            properties: Default::default(),
            annotations: vec![],
            implementations: vec![],
            content: Some(FileContent::Text("[package]".into())),
        });
        assert!(plan(&project, &Scope::Project).is_err());
    }
    #[test]
    fn binary_bytes_are_preserved() {
        let project: Project =
            serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/todo.json")).unwrap();
        let result = plan(&project, &Scope::Project).unwrap();
        let logo = result
            .artifacts()
            .find(|a| a.path.to_string_lossy().ends_with("logo.bin"))
            .unwrap();
        assert_eq!(
            logo.contents.as_deref(),
            Some(&[0, 1, 2, 3, 255, 0, 127][..])
        );
    }
}

#[cfg(test)]
mod completion_tests {
    use super::*;
    #[test]
    fn default_generation_omits_missing_body_and_respects_root_file() {
        let project: Project =
            serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/skeleton.json")).unwrap();
        let result = plan(&project, &Scope::Project).unwrap();
        let source = result
            .artifacts()
            .find(|a| a.path.to_string_lossy() == "domain.rs")
            .unwrap()
            .contents
            .as_ref()
            .unwrap();
        let code = String::from_utf8(source.clone()).unwrap();
        assert!(code.contains("pub struct Task"));
        assert!(code.contains("pub enum Status"));
        assert!(!code.contains("fn complete"));
        assert!(!code.contains("todo!"));
    }
    #[test]
    fn explicit_skeleton_keeps_signature_without_fabricating_body() {
        let project: Project =
            serde_json::from_str(include_str!("../../bellatrix_mde_model/fixtures/skeleton.json")).unwrap();
        let plan = plan_with_options(
            &project,
            &Scope::Project,
            GenerationOptions { skeleton: true },
        )
        .unwrap();
        let source = plan
            .artifacts()
            .find(|a| a.path.to_string_lossy() == "domain.rs")
            .unwrap()
            .contents
            .as_ref()
            .unwrap();
        let code = String::from_utf8(source.clone()).unwrap();
        assert!(code.contains("pub fn complete(&mut self);"));
        assert!(
            plan.diagnostics
                .iter()
                .any(|d| d.code == "rust.function.skeleton")
        );
    }
}
