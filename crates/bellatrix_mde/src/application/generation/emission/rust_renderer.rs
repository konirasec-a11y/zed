//! Pure Rust/Cargo rendering of prepared emission tasks. No source graph access.
#[path = "rust_renderer/cargo_manifest.rs"]
mod cargo;
#[path = "rust_renderer/declarations.rs"]
mod declarations;
#[path = "rust_renderer/emission.rs"]
mod emission;
use crate::template_pack::TemplatePack;

pub fn default_templates() -> Result<TemplatePack, Box<dyn std::error::Error>> {
    Ok(TemplatePack::embedded([
        (
            "rust/struct.rs.tera",
            include_str!("rust_renderer/templates/struct.rs.tera"),
        ),
        (
            "rust/trait.rs.tera",
            include_str!("rust_renderer/templates/trait.rs.tera"),
        ),
        (
            "rust/function.rs.tera",
            include_str!("rust_renderer/templates/function.rs.tera"),
        ),
        (
            "rust/signature.rs.tera",
            include_str!("rust_renderer/templates/signature.rs.tera"),
        ),
        (
            "rust/raw_item.rs.tera",
            include_str!("rust_renderer/templates/raw_item.rs.tera"),
        ),
        (
            "rust/Cargo.toml.tera",
            include_str!("rust_renderer/templates/Cargo.toml.tera"),
        ),
    ])?)
}

/// Render the kernel's versioned emission projection with the bundled templates.
/// The returned artifact manifest is not a validated publication plan. Cordis
/// sends it back through the kernel's path/collision gate before publication.
/// No I/O, source mutation, graph traversal or UML validation occurs here.
pub fn render(input_json: &str) -> Result<String, String> {
    let templates = default_templates().map_err(|error| error.to_string())?;
    render_with_templates(input_json, &templates)
}

/// Render prepared tasks with caller-supplied templates and no source model.
pub fn render_with_templates(input_json: &str, templates: &TemplatePack) -> Result<String, String> {
    let request: emission::Request =
        serde_json::from_str(input_json).map_err(|error| error.to_string())?;
    if request.api_version != 1 {
        return Err("unsupported Rust emission projection version".into());
    }
    let emission = request.emission;
    let mut rendered = emission::Rendered {
        api_version: 1,
        artifacts: emission.files,
        diagnostics: Vec::new(),
    };
    cargo::render_manifests(&emission.manifests, &mut rendered, templates)
        .map_err(|error| error.to_string())?;
    declarations::render_rust(&emission.units, templates, &mut rendered, emission.skeleton)
        .map_err(|error| error.to_string())?;
    serde_json::to_string(&rendered).map_err(|error| error.to_string())
}
