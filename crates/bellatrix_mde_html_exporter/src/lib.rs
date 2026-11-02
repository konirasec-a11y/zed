//! Static, headless, escaped HTML documentation and diagram SVG output.
mod pages;
mod svg;
use bellatrix_mde::{Project, Graph, Plan};

pub fn plan(project: &Project) -> Result<Plan, Box<dyn std::error::Error>> {
    Graph::new(project).validate()?;
    let mut result = Plan::new();
    result.directory("assets", "html.export")?;
    result.directory("elements", "html.export")?;
    result.directory("diagrams", "html.export")?;
    result.file("assets/style.css", pages::CSS.as_bytes().to_vec(), "html.styles")?;
    result.file("index.html", pages::index(project).into_bytes(), "html.index")?;
    for element in &project.elements {
        result.file(format!("elements/{}.html", pages::safe_id(&element.id)),
            pages::element_page(project, element).into_bytes(), &element.id)?;
    }
    for diagram in &project.diagrams {
        result.file(format!("diagrams/{}.svg", pages::safe_id(&diagram.id)),
            svg::draw(project, diagram).into_bytes(), &diagram.id)?;
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Project { serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json")).unwrap() }
    #[test]
    fn html_escapes_user_supplied_text() {
        let mut project = fixture(); project.name = "<script>alert('x')</script>".into();
        let plan = plan(&project).unwrap();
        let html = plan.artifacts().find(|artifact| artifact.path.to_string_lossy() == "index.html").unwrap().contents.as_ref().unwrap();
        let html = String::from_utf8(html.clone()).unwrap();
        assert!(!html.contains("<script>")); assert!(html.contains("&lt;script&gt;"));
    }
    #[test]
    fn cubic_routes_are_emitted() {
        let project = fixture(); assert!(svg::draw(&project, &project.diagrams[0]).contains(" C "));
    }
}
#[cfg(test)]
mod rich_diagram_tests {
    use super::*;
    fn fixture() -> Project { serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/skeleton.json")).unwrap() }
    #[test]
    fn svg_preserves_shape_color_dash_and_bezier() {
        let project = fixture(); let svg = svg::draw(&project, &project.diagrams[0]);
        assert!(svg.contains("<ellipse")); assert!(svg.contains("stroke-dasharray=\"3,5\""));
        assert!(svg.contains(" C ")); assert!(svg.contains("href=\"../elements/id-"));
    }
    #[test]
    fn exported_html_has_navigation_and_no_inline_scripts() {
        let files = plan(&fixture()).unwrap();
        let html = files.artifacts().find(|artifact| artifact.path.to_string_lossy() == "index.html").unwrap().contents.as_ref().unwrap();
        let html = String::from_utf8(html.clone()).unwrap();
        assert!(html.contains("Project documentation")); assert!(html.contains("Task structure")); assert!(!html.contains("<script"));
    }
}
#[cfg(test)]
mod relationship_tests {
    use super::*;
    #[test]
    fn documentation_links_semantic_graph_relations() {
        let project: Project = serde_json::from_str(include_str!("../../bellatrix_mde/fixtures/todo.json")).unwrap();
        let html = pages::index(&project);
        assert!(html.contains("<h2>Relationships</h2>")); assert!(html.contains("depends_on"));
        assert!(html.contains("elements/id-")); assert!(html.contains("TaskList"));
    }
}
