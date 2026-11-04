use super::{ResolvedRelationshipView, bound_end};
use crate::{
    EndpointMarkerBinding, EndpointMarkerComposition, EndpointMarkerGlyph, EndpointMarkerPart,
    EndpointMarkerPlacement, EndpointMarkerSource, LineMode, ModelingSection,
    RelationshipEndpointLabels, RelationshipMarker, RelationshipPresentation, RelationshipStyle,
    View,
};
use serde_json::Value;

pub(super) fn resolve(section: &ModelingSection, view: &View) -> ResolvedRelationshipView {
    let mut diagnostics = Vec::new();
    let object = view
        .subject
        .as_deref()
        .and_then(|id| section.elements.iter().find(|element| element.id == id));
    let line_mode = match object.map(|object| object.properties.get("identifying")) {
        Some(None | Some(Value::Bool(true))) => LineMode::Solid,
        Some(Some(Value::Bool(false))) => LineMode::Dotted,
        _ => {
            diagnostics.push(format!(
                "View {}: entity relationship identifying declaration is unresolved",
                view.id
            ));
            LineMode::Solid
        }
    };
    let markers = view
        .presentation
        .relationship_endpoints
        .as_ref()
        .map(|endpoints| {
            let mut resolve =
                |side: &str, binding: &EndpointMarkerBinding| match marker(section, binding) {
                    Ok(marker) => marker,
                    Err(error) => {
                        diagnostics.push(format!("View {} {side}: {error}", view.id));
                        EndpointMarkerComposition::default()
                    }
                };
            (
                resolve("source", &endpoints.source),
                resolve("target", &endpoints.target),
            )
        });
    if markers.is_none() {
        diagnostics.push(format!(
            "View {}: entity relationship end identities are unresolved",
            view.id
        ));
    }
    ResolvedRelationshipView {
        kind: "view.entity_relationship".into(),
        presentation: RelationshipPresentation {
            // The imported child labels retain their own placement and text.
            name: None,
            source_labels: RelationshipEndpointLabels::default(),
            target_labels: RelationshipEndpointLabels::default(),
            label: "entity relationship".into(),
            notation: "view.entity_relationship".into(),
            style: RelationshipStyle {
                source_marker: RelationshipMarker::None,
                target_marker: RelationshipMarker::None,
                line_mode,
                middle_marker: RelationshipMarker::None,
            },
            source_role: "source",
            target_role: "target",
        },
        markers,
        diagnostics,
    }
}

fn marker(
    section: &ModelingSection,
    binding: &EndpointMarkerBinding,
) -> Result<EndpointMarkerComposition, String> {
    if let EndpointMarkerSource::Authored { composition } = &binding.marker {
        return Ok(composition.clone());
    }
    let end = bound_end(section, binding)?;
    // Cardinality remains in the identified end. The notation's omission
    // default is not materialized as semantic UML multiplicity or persisted.
    let glyph = match end.properties.get("cardinality") {
        None => EndpointMarkerGlyph::CrowfootOne,
        Some(Value::String(value)) => match value.as_str() {
            "1" => EndpointMarkerGlyph::CrowfootOne,
            "1..*" => EndpointMarkerGlyph::CrowfootMany,
            "0..1" => EndpointMarkerGlyph::CrowfootZeroOne,
            "0..*" => EndpointMarkerGlyph::CrowfootZeroMany,
            _ => {
                return Err(format!(
                    "endpoint {} has unsupported cardinality notation",
                    end.id
                ));
            }
        },
        _ => {
            return Err(format!(
                "endpoint {} has malformed cardinality notation",
                end.id
            ));
        }
    };
    Ok(EndpointMarkerComposition {
        glyphs: vec![EndpointMarkerPart {
            glyph,
            placement: EndpointMarkerPlacement::Endpoint,
        }],
    })
}
