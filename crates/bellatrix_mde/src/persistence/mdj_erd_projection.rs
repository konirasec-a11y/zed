use super::{failure, insert_mapped, mdj};
use crate::{
    Authored, EndpointMarkerBinding, EndpointMarkerSource, EndpointMarkerTarget, EntityBoxBinding,
    Project, RelationshipEndpointPresentation, RelationshipNotation, View,
};
use serde_json::{Map, Value};

pub(super) fn bind(index: &mdj::Index<'_>, project: &mut Project) {
    for diagram in &mut project.diagrams {
        bind_views(index, &mut diagram.views);
    }
}

fn bind_views(index: &mdj::Index<'_>, views: &mut [View]) {
    for view in views {
        if let Some(object) = index.object(&view.id) {
            match object.type_name {
                "ERDEntityView" => {
                    let name_label = take_reference(view, "nameLabel");
                    let column_compartment = take_reference(view, "columnCompartment");
                    view.presentation.entity_box = Some(EntityBoxBinding {
                        name_label,
                        column_compartment,
                    });
                }
                "ERDRelationshipView" => {
                    view.presentation.relationship_notation =
                        Some(RelationshipNotation::EntityRelationship);
                    if let Some(relationship) = view.subject.as_deref().and_then(|id| index.object(id))
                        .filter(|object| object.type_name == "ERDRelationship")
                    {
                        let ends = ["end1", "end2"].map(|field| {
                            relationship.properties.get(field)
                                .filter(|end| end.get("_type").and_then(Value::as_str) == Some("ERDRelationshipEnd"))
                                .and_then(|end| end.get("_id")).and_then(Value::as_str)
                        });
                        if let [Some(source), Some(target)] = ends {
                            let binding = |id: &str| EndpointMarkerBinding {
                                target: EndpointMarkerTarget::ModelEnd { id: id.to_owned() },
                                marker: EndpointMarkerSource::Model,
                            };
                            view.presentation.relationship_endpoints = Some(RelationshipEndpointPresentation {
                                source: binding(source),
                                target: binding(target),
                            });
                        }
                    }
                }
                _ => {}
            }
        }
        bind_views(index, &mut view.children);
    }
}

fn take_reference(view: &mut View, field: &str) -> Authored<String> {
    let Some(source) = &mut view.presentation.source else { return Authored::Missing; };
    let value = match source.residual.get(field) {
        None => return Authored::Missing,
        Some(Value::Null) => Authored::Null,
        Some(value) => match value.as_object().filter(|object| object.len() == 1)
            .and_then(|object| object.get("$ref")).and_then(Value::as_str)
            .filter(|id| !id.is_empty())
        {
            Some(id) => Authored::Value(id.to_owned()),
            None => return Authored::Missing,
        },
    };
    source.residual.remove(field);
    value
}

pub(super) fn export(view: &View, object: &mut Map<String, Value>) -> Result<(), mdj::MdjError> {
    let Some(binding) = &view.presentation.entity_box else { return Ok(()); };
    if view.presentation.kind.as_deref() != Some("ERDEntityView") {
        return Err(failure(&view.id, "entity box binding has no inverse for this MDJ view type"));
    }
    for (field, value) in [("nameLabel", &binding.name_label), ("columnCompartment", &binding.column_compartment)] {
        match value {
            Authored::Missing => {},
            Authored::Null => insert_mapped(object, field, Value::Null)?,
            Authored::Value(id) => insert_mapped(object, field, serde_json::json!({"$ref": id}))?,
        }
    }
    Ok(())
}
