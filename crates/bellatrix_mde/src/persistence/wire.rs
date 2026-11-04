//! A typed protobuf envelope. Extensible properties and geometry retain JSON values.
use super::io::ModelIoError;
use crate::{Annotation, Diagram, Element, FileContent, Implementation, Project, Relation};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use prost::Message;

#[derive(Clone, PartialEq, Message)]
struct WireProject {
    #[prost(uint32, tag = "1")]
    version: u32,
    #[prost(string, tag = "2")]
    id: String,
    #[prost(string, tag = "3")]
    name: String,
    #[prost(message, repeated, tag = "4")]
    elements: Vec<WireElement>,
    #[prost(message, repeated, tag = "5")]
    relations: Vec<WireRelation>,
    #[prost(message, repeated, tag = "6")]
    diagrams: Vec<WireDiagram>,
    #[prost(string, optional, tag = "7")]
    root_model: Option<String>,
    // Retired fields are decoded only to reject their presence without losing user data.
    #[prost(bytes, optional, tag = "8")]
    legacy_explorer_parents_json: Option<Vec<u8>>,
    #[prost(bytes, optional, tag = "9")]
    legacy_explorer_diagram_occurrences_json: Option<Vec<u8>>,
    #[prost(message, repeated, tag = "10")]
    hierarchy: Vec<WireHierarchyPlacement>,
    #[prost(bytes, optional, tag = "11")]
    mdj_provenance_json: Option<Vec<u8>>,
    #[prost(bytes, optional, tag = "12")]
    composition_json: Option<Vec<u8>>,
}
#[derive(Clone, PartialEq, Message)]
struct WireHierarchyPlacement {
    #[prost(string, tag = "1")]
    subject_id: String,
    #[prost(string, optional, tag = "2")]
    parent_id: Option<String>,
    #[prost(message, optional, tag = "3")]
    occurrence: Option<WireViewOccurrence>,
}
#[derive(Clone, PartialEq, Message)]
struct WireViewOccurrence {
    #[prost(string, tag = "1")]
    diagram_id: String,
    #[prost(string, tag = "2")]
    view_id: String,
    #[prost(string, repeated, tag = "3")]
    member_path: Vec<String>,
}
#[derive(Clone, PartialEq, Message)]
struct WireElement {
    #[prost(string, tag = "1")]
    id: String,
    #[prost(string, tag = "2")]
    kind: String,
    #[prost(string, tag = "3")]
    name: String,
    #[prost(string, optional, tag = "4")]
    label: Option<String>,
    #[prost(string, optional, tag = "5")]
    description: Option<String>,
    #[prost(bytes, tag = "6")]
    properties_json: Vec<u8>,
    #[prost(message, repeated, tag = "7")]
    annotations: Vec<WireAnnotation>,
    #[prost(message, repeated, tag = "8")]
    implementations: Vec<WireImplementation>,
    #[prost(message, optional, tag = "9")]
    content: Option<WireContent>,
}
#[derive(Clone, PartialEq, Message)]
struct WireAnnotation {
    #[prost(string, tag = "1")]
    language: String,
    #[prost(string, tag = "2")]
    source: String,
}
#[derive(Clone, PartialEq, Message)]
struct WireImplementation {
    #[prost(string, tag = "1")]
    language: String,
    #[prost(string, tag = "2")]
    source: String,
}
#[derive(Clone, PartialEq, Message)]
struct WireContent {
    #[prost(bool, tag = "1")]
    text: bool,
    #[prost(bytes, tag = "2")]
    bytes: Vec<u8>,
}
#[derive(Clone, PartialEq, Message)]
struct WireRelation {
    #[prost(string, tag = "1")]
    id: String,
    #[prost(string, tag = "2")]
    kind: String,
    #[prost(string, tag = "3")]
    from: String,
    #[prost(string, tag = "4")]
    to: String,
    #[prost(bytes, tag = "5")]
    properties_json: Vec<u8>,
}
#[derive(Clone, PartialEq, Message)]
struct WireDiagram {
    #[prost(string, tag = "1")]
    id: String,
    #[prost(string, tag = "2")]
    name: String,
    #[prost(double, tag = "3")]
    width: f64,
    #[prost(double, tag = "4")]
    height: f64,
    #[prost(message, repeated, tag = "5")]
    views: Vec<WireView>,
    #[prost(string, optional, tag = "6")]
    uml_di_type: Option<String>,
    #[prost(string, repeated, tag = "7")]
    model_elements: Vec<String>,
    #[prost(string, optional, tag = "8")]
    interaction_kind: Option<String>,
}
#[derive(Clone, PartialEq, Message)]
struct WireView {
    #[prost(string, tag = "1")]
    id: String,
    #[prost(string, optional, tag = "2")]
    subject: Option<String>,
    #[prost(string, optional, tag = "3")]
    label: Option<String>,
    #[prost(bytes, tag = "4")]
    geometry_json: Vec<u8>,
    #[prost(bytes, tag = "5")]
    style_json: Vec<u8>,
    #[prost(sint32, tag = "6")]
    z_index: i32,
    #[prost(bytes, tag = "7")]
    draft_json: Vec<u8>,
    #[prost(bytes, optional, tag = "8")]
    timing_json: Option<Vec<u8>>,
    #[prost(bytes, optional, tag = "9")]
    endpoints_json: Option<Vec<u8>>,
    #[prost(message, repeated, tag = "10")]
    children: Vec<WireView>,
    #[prost(bytes, optional, tag = "11")]
    presentation_json: Option<Vec<u8>>,
}

fn encode_view(view: &crate::View) -> Result<WireView, ModelIoError> {
    Ok(WireView {
        id: view.id.clone(),
        subject: view.subject.clone(),
        label: view.label.clone(),
        geometry_json: serde_json::to_vec(&view.geometry)?,
        style_json: serde_json::to_vec(&view.style)?,
        z_index: view.z_index,
        draft_json: serde_json::to_vec(&view.draft)?,
        timing_json: view.timing.as_ref().map(serde_json::to_vec).transpose()?,
        endpoints_json: view
            .endpoints
            .as_ref()
            .map(serde_json::to_vec)
            .transpose()?,
        children: view
            .children
            .iter()
            .map(encode_view)
            .collect::<Result<_, _>>()?,
        presentation_json: if view.presentation.is_empty() {
            None
        } else {
            Some(serde_json::to_vec(&view.presentation)?)
        },
    })
}

fn decode_view(view: WireView) -> Result<crate::View, ModelIoError> {
    Ok(crate::View {
        id: view.id,
        subject: view.subject,
        label: view.label,
        geometry: if view.geometry_json.is_empty() {
            Default::default()
        } else {
            serde_json::from_slice(&view.geometry_json)?
        },
        style: serde_json::from_slice(&view.style_json)?,
        z_index: view.z_index,
        draft: if view.draft_json.is_empty() {
            None
        } else {
            serde_json::from_slice(&view.draft_json)?
        },
        timing: view
            .timing_json
            .as_deref()
            .map(serde_json::from_slice)
            .transpose()?,
        endpoints: view
            .endpoints_json
            .as_deref()
            .map(serde_json::from_slice)
            .transpose()?,
        children: view
            .children
            .into_iter()
            .map(decode_view)
            .collect::<Result<_, _>>()?,
        presentation: view
            .presentation_json
            .as_deref()
            .map(serde_json::from_slice)
            .transpose()?
            .unwrap_or_default(),
    })
}

pub(super) fn encode(project: &Project) -> Result<Vec<u8>, ModelIoError> {
    let mut elements = Vec::new();
    for element in &project.elements {
        let content = element
            .content
            .as_ref()
            .map(|value| {
                let text = matches!(value, FileContent::Text(_));
                Ok::<_, ModelIoError>(WireContent {
                    text,
                    bytes: value.bytes()?,
                })
            })
            .transpose()?;
        elements.push(WireElement {
            id: element.id.clone(),
            kind: element.kind.clone(),
            name: element.name.clone(),
            label: element.label.clone(),
            description: element.description.clone(),
            properties_json: serde_json::to_vec(&element.properties)?,
            annotations: element
                .annotations
                .iter()
                .map(|a| WireAnnotation {
                    language: a.language.clone(),
                    source: a.source.clone(),
                })
                .collect(),
            implementations: element
                .implementations
                .iter()
                .map(|i| WireImplementation {
                    language: i.language.clone(),
                    source: i.source.clone(),
                })
                .collect(),
            content,
        });
    }
    let relations = project
        .relations
        .iter()
        .map(|r| {
            Ok::<_, ModelIoError>(WireRelation {
                id: r.id.clone(),
                kind: r.kind.clone(),
                from: r.from.clone(),
                to: r.to.clone(),
                properties_json: serde_json::to_vec(&r.properties)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let diagrams = project
        .diagrams
        .iter()
        .map(|d| {
            Ok::<_, ModelIoError>(WireDiagram {
                id: d.id.clone(),
                name: d.name.clone(),
                width: d.width,
                height: d.height,
                uml_di_type: Some(d.uml_di_type.clone()),
                model_elements: d.model_elements.clone(),
                interaction_kind: d.interaction_kind.clone(),
                views: d
                    .views
                    .iter()
                    .map(encode_view)
                    .collect::<Result<Vec<_>, _>>()?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(WireProject {
        version: project.schema_version,
        composition_json: Some(serde_json::to_vec(&project.composition)?),
        root_model: project.root_model.clone(),
        mdj_provenance_json: project
            .mdj_provenance
            .as_ref()
            .map(serde_json::to_vec)
            .transpose()?,
        legacy_explorer_parents_json: None,
        legacy_explorer_diagram_occurrences_json: None,
        hierarchy: project
            .hierarchy
            .iter()
            .map(|(subject_id, placement)| WireHierarchyPlacement {
                subject_id: subject_id.clone(),
                parent_id: placement.parent_id.clone(),
                occurrence: placement
                    .occurrence
                    .as_ref()
                    .map(|occurrence| WireViewOccurrence {
                        diagram_id: occurrence.diagram_id.clone(),
                        view_id: occurrence.view_id.clone(),
                        member_path: occurrence.member_path.clone(),
                    }),
            })
            .collect(),
        id: project.id.clone(),
        name: project.name.clone(),
        elements,
        relations,
        diagrams,
    }
    .encode_to_vec())
}

pub(super) fn decode(bytes: &[u8]) -> Result<Project, ModelIoError> {
    let data = WireProject::decode(bytes)?;
    if data.legacy_explorer_parents_json.is_some()
        || data.legacy_explorer_diagram_occurrences_json.is_some()
    {
        return Err(ModelIoError::Hierarchy(
            "Legacy Explorer placement fields require explicit conversion before import".into(),
        ));
    }
    let mut hierarchy = std::collections::BTreeMap::new();
    for entry in data.hierarchy {
        let placement = crate::HierarchyPlacement {
            parent_id: entry.parent_id,
            occurrence: entry.occurrence.map(|occurrence| crate::ViewOccurrence {
                diagram_id: occurrence.diagram_id,
                view_id: occurrence.view_id,
                member_path: occurrence.member_path,
            }),
        };
        if hierarchy.insert(entry.subject_id, placement).is_some() {
            return Err(ModelIoError::Hierarchy(
                "Duplicate hierarchy subject".into(),
            ));
        }
    }

    let elements = data
        .elements
        .into_iter()
        .map(|e| {
            let content = e
                .content
                .map(|c| -> Result<_, ModelIoError> {
                    if c.text {
                        Ok(FileContent::Text(String::from_utf8(c.bytes)?))
                    } else {
                        Ok(FileContent::Base64(STANDARD.encode(c.bytes)))
                    }
                })
                .transpose()?;
            Ok::<_, ModelIoError>(Element {
                id: e.id,
                kind: e.kind,
                name: e.name,
                label: e.label,
                description: e.description,
                properties: serde_json::from_slice(&e.properties_json)?,
                annotations: e
                    .annotations
                    .into_iter()
                    .map(|a| Annotation {
                        language: a.language,
                        source: a.source,
                    })
                    .collect(),
                implementations: e
                    .implementations
                    .into_iter()
                    .map(|i| Implementation {
                        language: i.language,
                        source: i.source,
                    })
                    .collect(),
                content,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let relations = data
        .relations
        .into_iter()
        .map(|r| {
            Ok::<_, ModelIoError>(Relation {
                id: r.id,
                kind: r.kind,
                from: r.from,
                to: r.to,
                properties: serde_json::from_slice(&r.properties_json)?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let diagrams = data
        .diagrams
        .into_iter()
        .map(|d| {
            Ok::<_, ModelIoError>(Diagram {
                id: d.id,
                name: d.name,
                width: d.width,
                height: d.height,
                uml_di_type: d
                    .uml_di_type
                    .filter(|value| !value.trim().is_empty())
                    .ok_or(ModelIoError::DiagramType)?,
                model_elements: d.model_elements,
                interaction_kind: d.interaction_kind,
                views: d
                    .views
                    .into_iter()
                    .map(decode_view)
                    .collect::<Result<Vec<_>, _>>()?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Project {
        schema_version: data.version,
        composition: data
            .composition_json
            .as_deref()
            .map(|bytes| {
                let mut deserializer = serde_json::Deserializer::from_slice(bytes);
                let composition =
                    crate::contracts::composition::deserialize_composition(&mut deserializer)?;
                deserializer.end()?;
                Ok::<_, serde_json::Error>(composition)
            })
            .transpose()?
            .unwrap_or_default(),
        root_model: data.root_model,
        mdj_provenance: data
            .mdj_provenance_json
            .as_deref()
            .map(serde_json::from_slice)
            .transpose()?,
        hierarchy,
        id: data.id,
        name: data.name,
        elements,
        relations,
        diagrams,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uml_metadata_and_future_type_names_round_trip_in_json_and_wire() -> Result<(), ModelIoError>
    {
        for type_name in ["UMLCompositeStructureDiagram", "UMLFutureDiagram"] {
            let project: Project = serde_json::from_value(serde_json::json!({
                "id": "project", "name": "Typed", "diagrams": [{
                    "id": "diagram", "name": "Typed", "uml_di_type": type_name,
                    "model_elements": ["context"]
                }]
            }))?;
            assert_eq!(project.schema_version, 1);
            assert_eq!(decode(&encode(&project)?)?, project);
            assert_eq!(
                serde_json::from_slice::<Project>(&serde_json::to_vec(&project)?)?,
                project
            );
        }
        Ok(())
    }
}
