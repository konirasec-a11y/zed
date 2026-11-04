use crate::{
    AssociationEndConvention, Element, EndpointMarkerBinding, EndpointMarkerComposition,
    EndpointMarkerGlyph, EndpointMarkerPart, EndpointMarkerPlacement, EndpointMarkerSource,
    EndpointMarkerTarget, ModelingSection, RelationshipEndpointLabels, RelationshipPresentation,
    View, relationship_presentation, relationship_style_for_notation,
};
use serde_json::Value;
#[path = "entity_relationship.rs"]
mod entity_relationship;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NavigationDeclaration {
    Unspecified,
    Navigable,
    NotNavigable,
}

pub struct ResolvedRelationshipView {
    pub kind: String,
    pub presentation: RelationshipPresentation,
    pub markers: Option<(EndpointMarkerComposition, EndpointMarkerComposition)>,
    pub diagnostics: Vec<String>,
}

fn bound_end<'a>(
    section: &'a ModelingSection,
    binding: &EndpointMarkerBinding,
) -> Result<&'a Element, String> {
    let EndpointMarkerTarget::ModelEnd { id } = &binding.target else {
        return Err("marker role/view resolution is not implemented for this notation".into());
    };
    section
        .elements
        .iter()
        .find(|element| element.id == *id)
        .ok_or_else(|| format!("marker endpoint {id} is unresolved"))
}

fn composition(navigation: NavigationDeclaration, aggregation: &str) -> EndpointMarkerComposition {
    use EndpointMarkerGlyph::*;
    let diamond = match aggregation {
        "shared" => Some(OpenDiamond),
        "composite" => Some(FilledDiamond),
        _ => None,
    };
    let glyphs = match (navigation, diamond) {
        (NavigationDeclaration::Navigable, Some(diamond)) => vec![diamond, OpenArrow],
        (_, Some(diamond)) => vec![diamond],
        (NavigationDeclaration::Navigable, None) => vec![OpenArrow],
        (NavigationDeclaration::NotNavigable, None) => vec![Cross],
        (NavigationDeclaration::Unspecified, None) => Vec::new(),
    };
    EndpointMarkerComposition {
        glyphs: glyphs
            .into_iter()
            .enumerate()
            .map(|(index, glyph)| EndpointMarkerPart {
                glyph,
                placement: if index == 0 {
                    EndpointMarkerPlacement::Endpoint
                } else {
                    EndpointMarkerPlacement::AfterPrevious
                },
            })
            .collect(),
    }
}

fn resolve_marker(
    section: &ModelingSection,
    binding: &EndpointMarkerBinding,
    aggregation_binding: &EndpointMarkerBinding,
) -> Result<EndpointMarkerComposition, String> {
    if let EndpointMarkerSource::Authored { composition } = &binding.marker {
        return Ok(composition.clone());
    }
    let end = bound_end(section, binding)?;
    let aggregation_end = bound_end(section, aggregation_binding)?;
    let navigation = if end.kind == "mdj.UMLAssociationEnd" {
        // These are authored interchange values in the document contract. They
        // are neither a Boolean UML assertion nor copied provenance state.
        match end.properties.get("navigable") {
            None => NavigationDeclaration::Unspecified,
            Some(Value::String(value)) if value == "unspecified" => {
                NavigationDeclaration::Unspecified
            }
            Some(Value::String(value)) if value == "navigable" => NavigationDeclaration::Navigable,
            Some(Value::String(value)) if value == "notNavigable" => {
                NavigationDeclaration::NotNavigable
            }
            _ => {
                return Err(format!(
                    "endpoint {} has an unsupported navigation declaration",
                    end.id
                ));
            }
        }
    } else if end.kind == "uml.attribute" {
        let references = crate::uml::references::ReferenceIndex::new(
            crate::uml::root::Model::from_section(section),
        );
        match crate::uml::associations::navigation_knowledge(&references, &end.id)? {
            crate::NavigationKnowledge::Known(true) => NavigationDeclaration::Navigable,
            crate::NavigationKnowledge::Known(false) => NavigationDeclaration::NotNavigable,
            crate::NavigationKnowledge::Unverified => NavigationDeclaration::Unspecified,
        }
    } else {
        return Err(format!("endpoint {} has no marker projection", end.id));
    };
    if end
        .properties
        .get("ownerAttribute")
        .is_some_and(|value| !value.is_null())
        || aggregation_end
            .properties
            .get("ownerAttribute")
            .is_some_and(|value| !value.is_null())
    {
        return Err(format!(
            "endpoint {} requires ownerAttribute presentation resolution",
            end.id
        ));
    }
    let aggregation = match aggregation_end.properties.get("aggregation") {
        None => "none",
        Some(Value::String(value)) if matches!(value.as_str(), "none" | "shared" | "composite") => {
            value
        }
        _ => {
            return Err(format!(
                "endpoint {} has an unsupported aggregation encoding",
                aggregation_end.id
            ));
        }
    };
    Ok(composition(navigation, aggregation))
}

/// Resolves presentation from the current document contract only. Opaque source
/// objects remain opaque; drawing them creates no canonical UML relationship.
pub fn resolve_relationship_view(
    section: &ModelingSection,
    view: &View,
) -> Option<ResolvedRelationshipView> {
    if view.presentation.relationship_notation == Some(crate::RelationshipNotation::EntityRelationship) {
        return Some(entity_relationship::resolve(section, view));
    }
    let subject = view.subject.as_deref()?;
    let relation = section
        .relations
        .iter()
        .find(|relation| relation.id == subject);
    let (kind, presentation) = if let Some(relation) = relation {
        (relation.kind.clone(), relationship_presentation(relation))
    } else {
        let object = section
            .elements
            .iter()
            .find(|element| element.id == subject && element.kind == "mdj.UMLAssociation")?;
        let endpoints = view.presentation.relationship_endpoints.as_ref()?;
        let labels = |binding: &EndpointMarkerBinding| {
            let end = bound_end(section, binding).ok();
            RelationshipEndpointLabels {
                role: end
                    .filter(|end| !end.name.is_empty())
                    .map(|end| end.name.clone()),
                multiplicity: end
                    .and_then(|end| end.properties.get("multiplicity"))
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned),
            }
        };
        (
            "view.association".into(),
            RelationshipPresentation {
                name: (!object.name.is_empty()).then(|| object.name.clone()),
                source_labels: labels(&endpoints.source),
                target_labels: labels(&endpoints.target),
                label: "association".into(),
                notation: "view.association".into(),
                style: relationship_style_for_notation("uml.association"),
                source_role: "source",
                target_role: "target",
            },
        )
    };
    let mut resolved = ResolvedRelationshipView {
        kind,
        presentation,
        markers: None,
        diagnostics: Vec::new(),
    };
    // Child label views own their text, placement and visibility. Do not also
    // paint generated labels at unrelated offsets over the authored subviews.
    if view.children.iter().any(|child| {
        matches!(
            child.presentation.kind.as_deref(),
            Some("EdgeLabelView" | "LabelView")
        )
    }) {
        resolved.presentation.name = None;
        resolved.presentation.source_labels = RelationshipEndpointLabels::default();
        resolved.presentation.target_labels = RelationshipEndpointLabels::default();
    }
    if let Some(endpoints) = &view.presentation.relationship_endpoints {
        let convention = view.style.uml.association_end_convention;
        let (source_aggregation, target_aggregation) =
            if convention == Some(AssociationEndConvention::Uml) {
                (&endpoints.target, &endpoints.source)
            } else {
                (&endpoints.source, &endpoints.target)
            };
        let mut marker =
            |side: &str, binding: &EndpointMarkerBinding, aggregation: &EndpointMarkerBinding| {
                if convention.is_none() && matches!(binding.marker, EndpointMarkerSource::Model) {
                    resolved.diagnostics.push(format!(
                        "{side} model endpoint marker needs an explicit notation convention"
                    ));
                    return EndpointMarkerComposition::default();
                }
                match resolve_marker(section, binding, aggregation) {
                    Ok(marker) => marker,
                    Err(message) => {
                        resolved.diagnostics.push(format!("{side}: {message}"));
                        EndpointMarkerComposition::default()
                    }
                }
            };
        let source = marker("source", &endpoints.source, source_aggregation);
        let target = marker("target", &endpoints.target, target_aggregation);
        resolved.markers = Some((source, target));
    }
    Some(resolved)
}
