//! Graph-directed Rust/Cargo artifacts. This is not a Rust compiler.
mod cargo;
mod declarations;
mod files;

use bellatrix_mde_model::Project;
use bellatrix_mde_modeling::{Graph, Plan, Scope, select_elements};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GenerationOptions {
    /// Preserve declarations without bodies as invalid-but-explicit Rust skeletons.
    pub skeleton: bool,
}

pub fn plan(project: &Project, scope: &Scope) -> Result<Plan, Box<dyn std::error::Error>> {
    plan_with_options(project, scope, GenerationOptions::default())
}

pub fn plan_with_options(
    project: &Project,
    scope: &Scope,
    options: GenerationOptions,
) -> Result<Plan, Box<dyn std::error::Error>> {
    Graph::new(project).validate()?;
    let selected = select_elements(project, scope)?;
    let mut result = Plan::new();
    files::plan_files(project, &selected, &mut result)?;
    cargo::plan_manifests(project, &selected, &mut result)?;
    declarations::plan_rust(project, &selected, &mut result, options.skeleton)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
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
