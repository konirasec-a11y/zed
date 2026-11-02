//! A typed protobuf envelope. Extensible properties and geometry retain JSON values.
use super::io::ModelIoError;
use crate::{Annotation, Diagram, Element, FileContent, Implementation, Project, Relation, View};
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
                uml_di_type: d.uml_di_type.clone(),
                model_elements: d.model_elements.clone(),
                views: d
                    .views
                    .iter()
                    .map(|v| {
                        Ok::<_, ModelIoError>(WireView {
                            id: v.id.clone(),
                            subject: v.subject.clone(),
                            label: v.label.clone(),
                            geometry_json: serde_json::to_vec(&v.geometry)?,
                            style_json: serde_json::to_vec(&v.style)?,
                            z_index: v.z_index,
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(WireProject {
        version: project.schema_version,
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
                uml_di_type: d.uml_di_type,
                model_elements: d.model_elements,
                views: d
                    .views
                    .into_iter()
                    .map(|v| {
                        Ok::<_, ModelIoError>(View {
                            id: v.id,
                            subject: v.subject,
                            label: v.label,
                            geometry: serde_json::from_slice(&v.geometry_json)?,
                            style: serde_json::from_slice(&v.style_json)?,
                            z_index: v.z_index,
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Project {
        schema_version: data.version,
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
    fn legacy_json_has_no_uml_metadata_and_keeps_schema_version() -> Result<(), ModelIoError> {
        let project: Project = serde_json::from_value(serde_json::json!({
            "id": "project", "name": "Legacy", "diagrams": [{"id": "diagram", "name": "Legacy"}]
        }))?;
        assert_eq!(project.schema_version, 1);
        for diagram in &project.diagrams {
            assert_eq!(diagram.uml_di_type, None);
            assert!(diagram.model_elements.is_empty());
            let json = serde_json::to_value(diagram)?;
            assert!(json.get("uml_di_type").is_none());
            assert!(json.get("model_elements").is_none());
        }
        assert_eq!(decode(&encode(&project)?)?, project);
        Ok(())
    }

    #[test]
    fn original_wire_tags_decode_without_new_metadata() -> Result<(), ModelIoError> {
        // Version 1 envelope with id/name and a Diagram containing only its
        // original tags 1/2. No tag 6/7 exists inside this legacy Diagram.
        let legacy = [
            0x08, 0x01, 0x12, 0x01, b'p', 0x1a, 0x01, b'P', 0x32, 0x06, 0x0a, 0x01, b'd', 0x12,
            0x01, b'D',
        ];
        let project = decode(&legacy)?;
        assert_eq!(project.schema_version, 1);
        assert_eq!(project.diagrams.len(), 1);
        for diagram in &project.diagrams {
            assert_eq!(diagram.uml_di_type, None);
            assert!(diagram.model_elements.is_empty());
        }
        Ok(())
    }

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
