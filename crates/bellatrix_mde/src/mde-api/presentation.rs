use crate::api_contract::{API_VERSION, error_response};
use crate::mde_api_document::decode_model_request;
use crate::uml::{references::ReferenceIndex, root::Model};
use serde_json::json;

/// Produce a read-only documentation projection for the Cordis export handler.
///
/// Input is exactly `{api_version: 1, model}`. Output is a versioned `document`
/// projection containing display text, links and flattened diagram primitives.
/// It is not a model document and cannot be used for persistence or mutation.
/// UML labels and relationship stroke patterns are resolved here; the renderer
/// receives no UML query objects and does not decide model semantics.
///
/// Graph integrity is required, matching the existing documentation export gate.
/// This does not certify UML conformance. Explicit empty labels are preserved;
/// unresolved layout remains an explicit layout primitive, never invented bounds.
///
/// # Errors
/// Uses the versioned document errors and `GRAPH_INTEGRITY`. No partial projection
/// is returned. Original model identifiers remain opaque strings.
///
/// # Effects and determinism
/// No I/O, mutation, clocks, callbacks or retained state. Text is not HTML-escaped;
/// the renderer must escape all text for its target format.
pub fn project_documentation(input_json: &str) -> Result<String, String> {
    let model = decode_model_request(input_json)?;
    crate::Graph::new(&model)
        .validate()
        .map_err(|error| error_response("GRAPH_INTEGRITY", error.to_string()))?;
    let references = ReferenceIndex::new(Model::from_project(&model));
    let elements = model
        .elements
        .iter()
        .map(|element| {
            let properties = element
                .properties
                .iter()
                .map(|(key, value)| (key.clone(), value.to_string()))
                .collect::<std::collections::BTreeMap<_, _>>();
            json!({
                "id": element.id,
                "name": element.name,
                "label": element.label.as_deref().unwrap_or(&element.name),
                "kind": element.kind,
                "description": element.description,
                "properties": properties,
                "annotations": element.annotations,
                "implementations": element.implementations,
            })
        })
        .collect::<Vec<_>>();
    let relations = model
        .relations
        .iter()
        .map(|relation| {
            json!({
                "id": relation.id, "kind": relation.kind, "from": relation.from, "to": relation.to,
            })
        })
        .collect::<Vec<_>>();
    let diagrams = model.diagrams.iter().map(|diagram| {
        let views = diagram.all_views().map(|view| {
            let label = view.label.clone().or_else(|| {
                view.subject.as_deref().and_then(|subject| model.element(subject))
                    .map(|element| element.label.clone().unwrap_or_else(|| {
                        crate::format_element_label_for_view(element, &references, &view.style)
                    }))
            }).unwrap_or_default();
            let notation_mode = view.draft.as_ref().map(|draft| {
                crate::resolve_relationship_style(&draft.notation_kind, &draft.properties).line_mode
            }).or_else(|| {
                view.subject.as_ref().and_then(|subject| {
                    model.relations.iter().find(|relation| &relation.id == subject)
                        .map(|relation| crate::resolve_relationship_style(&relation.kind, &relation.properties).line_mode)
                })
            }).unwrap_or(crate::LineMode::Solid);
            let geometry = match &view.geometry {
                crate::Geometry::Layout { .. } => json!({"kind": "layout"}),
                geometry => json!(geometry),
            };
            json!({
                "id": view.id, "subject": view.subject, "label": label,
                "geometry": geometry, "z_index": view.z_index,
                "style": {
                    "fill": view.style.fill, "stroke": view.style.stroke,
                    "stroke_width": view.style.stroke_width, "opacity": view.style.opacity,
                    "font_family": view.style.font_family, "font_size": view.style.font_size,
                    "text_color": view.style.text_color, "text_align": view.style.text_align,
                    "text_vertical_align": view.style.text_vertical_align,
                    "corner_radius": view.style.corner_radius, "show_shadow": view.style.show_shadow,
                    "dash_pattern": view.style.resolve_stroke_pattern(notation_mode),
                },
            })
        }).collect::<Vec<_>>();
        let page_subjects = diagram.views.iter().filter_map(|view| view.subject.as_deref()).collect::<Vec<_>>();
        json!({"id": diagram.id, "name": diagram.name, "width": diagram.width,
            "height": diagram.height, "page_subjects": page_subjects, "views": views})
    }).collect::<Vec<_>>();
    Ok(json!({"api_version": API_VERSION, "document": {
        "name": model.name, "elements": elements, "relations": relations, "diagrams": diagrams,
    }})
    .to_string())
}
