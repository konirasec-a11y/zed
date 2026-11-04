use super::{failure, insert_mapped, mdj};
use crate::{
    Authored, InterfaceConnection, InterfaceConnectionRole, InterfaceIconBinding, Project, View,
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
            if object.type_name == "UMLInterfaceView" {
                let mut compartment = Authored::Missing;
                if let Some(source) = &mut view.presentation.source {
                    match source.residual.get("nameCompartment") {
                        None => {}
                        Some(Value::Null) => {
                            compartment = Authored::Null;
                            source.residual.remove("nameCompartment");
                        }
                        Some(value) => {
                            if let Some(id) = reference(value) {
                                compartment = Authored::Value(id.to_owned());
                                source.residual.remove("nameCompartment");
                            }
                        }
                    }
                }
                view.presentation.interface_icon = Some(InterfaceIconBinding {
                    label_compartment: compartment,
                });
            }
            let role = match object.type_name {
                "UMLDependencyView" => Some(InterfaceConnectionRole::Required),
                "UMLInterfaceRealizationView" => Some(InterfaceConnectionRole::Provided),
                _ => None,
            };
            if let Some(role) = role {
                view.presentation.interface_connection = Some(InterfaceConnection {
                    role,
                    target_unverified: object
                        .properties
                        .get("head")
                        .is_some_and(|value| !value.is_null() && reference(value).is_none()),
                });
            }
        }
        bind_views(index, &mut view.children);
    }
}

fn reference(value: &Value) -> Option<&str> {
    value
        .as_object()
        .filter(|object| object.len() == 1)?
        .get("$ref")?
        .as_str()
        .filter(|value| !value.is_empty())
}

pub(super) fn export(view: &View, object: &mut Map<String, Value>) -> Result<(), mdj::MdjError> {
    let Some(binding) = &view.presentation.interface_icon else {
        return Ok(());
    };
    if view.presentation.kind.as_deref() != Some("UMLInterfaceView") {
        return Err(failure(
            &view.id,
            "interface icon binding has no inverse for this MDJ view type",
        ));
    }
    match &binding.label_compartment {
        Authored::Missing => {}
        Authored::Null => insert_mapped(object, "nameCompartment", Value::Null)?,
        Authored::Value(id) => {
            insert_mapped(object, "nameCompartment", serde_json::json!({"$ref": id}))?
        }
    }
    Ok(())
}
