//! Static, headless HTML/SVG renderer over a neutral documentation projection.
mod document;
mod pages;
mod svg;
use document::DocumentationEnvelope;
use serde::Serialize;
use std::collections::HashSet;

#[derive(Serialize)]
struct Artifact {
    path: String,
    contents: Option<String>,
    origin: String,
}

/// Render a version-1 documentation projection supplied by the Cordis handler.
/// Returns `{api_version:1, artifacts:[{path, contents, origin}]}`; null contents
/// denotes a directory. This renderer performs no model queries or file writes.
/// Every text value is escaped for HTML/SVG before entering an artifact.
pub fn render(input_json: &str) -> Result<String, String> {
    let envelope: DocumentationEnvelope = serde_json::from_str(input_json)
        .map_err(|error| format!("invalid documentation projection: {error}"))?;
    if envelope.api_version != 1 {
        return Err("unsupported documentation projection version".into());
    }
    let project = envelope.document;
    let mut identities = HashSet::new();
    for id in project.elements.iter().map(|element| &element.id)
        .chain(project.relations.iter().map(|relation| &relation.id))
        .chain(project.diagrams.iter().map(|diagram| &diagram.id))
        .chain(project.diagrams.iter().flat_map(|diagram| diagram.views.iter().map(|view| &view.id)))
    {
        if id.trim().is_empty() || !identities.insert(id) {
            return Err("documentation projection contains empty or duplicate identities".into());
        }
    }
    for diagram in &project.diagrams {
        if !diagram.width.is_finite() || !diagram.height.is_finite()
            || diagram.width <= 0.0 || diagram.height <= 0.0
        {
            return Err("documentation projection contains invalid diagram dimensions".into());
        }
    }
    let mut artifacts = ["assets", "elements", "diagrams"].into_iter().map(|path| Artifact {
        path: path.into(), contents: None, origin: "html.export".into(),
    }).collect::<Vec<_>>();
    artifacts.push(Artifact { path: "assets/style.css".into(), contents: Some(pages::CSS.into()), origin: "html.styles".into() });
    artifacts.push(Artifact { path: "index.html".into(), contents: Some(pages::index(&project)), origin: "html.index".into() });
    for element in &project.elements {
        artifacts.push(Artifact { path: format!("elements/{}.html", pages::safe_id(&element.id)),
            contents: Some(pages::element_page(&project, element)), origin: element.id.clone() });
    }
    for diagram in &project.diagrams {
        artifacts.push(Artifact { path: format!("diagrams/{}.svg", pages::safe_id(&diagram.id)),
            contents: Some(svg::draw(&project, diagram)), origin: diagram.id.clone() });
    }
    Ok(serde_json::json!({"api_version": 1, "artifacts": artifacts}).to_string())
}
