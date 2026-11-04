use super::mdj;
use crate::{LabelOptionsSource, Project, SubjectLabelBinding, SubjectLabelRole, View};
use serde_json::Value;

pub(super) fn bind(index: &mdj::Index<'_>, project: &mut Project) {
    for diagram in &mut project.diagrams {
        bind_views(index, &mut diagram.views);
    }
}

fn bind_views(index: &mdj::Index<'_>, views: &mut [View]) {
    for view in views {
        if let Some(object) = index.object(&view.id) {
            let role = match object.type_name {
                "UMLAttributeView" => Some(SubjectLabelRole::Attribute),
                "UMLOperationView" => Some(SubjectLabelRole::Operation),
                _ => None,
            };
            if let Some(role) = role {
                view.presentation.label_binding = Some(SubjectLabelBinding {
                    role,
                    options_source: options_source(index, object, role)
                        .unwrap_or_else(|reason| LabelOptionsSource::Unresolved { reason }),
                });
            }
        }
        bind_views(index, &mut view.children);
    }
}

fn parent<'index, 'document>(
    index: &'index mdj::Index<'document>,
    object: &mdj::Object<'document>,
) -> Result<Option<&'index mdj::Object<'document>>, String> {
    let Some(value) = object
        .properties
        .get("_parent")
        .filter(|value| !value.is_null())
    else {
        return Ok(None);
    };
    let identifier = value
        .as_object()
        .filter(|reference| reference.len() == 1)
        .and_then(|reference| reference.get("$ref"))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("label options parent of {} is malformed", object.id))?;
    index
        .object(identifier)
        .map(Some)
        .ok_or_else(|| format!("label options parent {identifier} is unresolved"))
}

fn options_source(
    index: &mdj::Index<'_>,
    object: &mdj::Object<'_>,
    role: SubjectLabelRole,
) -> Result<LabelOptionsSource, String> {
    // The vendored member Views read options from the visual grandparent,
    // independently of parentStyle. Explicit references, not tree proximity,
    // determine that source (elements.js UMLAttributeView/UMLOperationView).
    let Some(parent) = parent(index, object)? else {
        return Ok(LabelOptionsSource::Defaults);
    };
    let Some(owner) = self::parent(index, parent)? else {
        return Ok(LabelOptionsSource::Defaults);
    };
    if !matches!(
        owner.type_name,
        "UMLClassifierView"
            | "UMLClassView"
            | "UMLInterfaceView"
            | "UMLSignalView"
            | "UMLDataTypeView"
            | "UMLPrimitiveTypeView"
            | "UMLEnumerationView"
            | "UMLArtifactView"
            | "UMLComponentView"
            | "UMLNodeView"
            | "UMLUseCaseView"
            | "UMLActorView"
            | "UMLInformationItemView"
            | "UMLStereotypeView"
    ) {
        return Err(format!(
            "label options on {} have an unsupported view contract",
            owner.id
        ));
    }
    for field in [
        "showVisibility",
        "showType",
        "showProperty",
        "showMultiplicity",
    ] {
        if owner
            .properties
            .get(field)
            .is_some_and(|value| !value.is_boolean())
        {
            return Err(format!(
                "label option {field} on {} is unsupported",
                owner.id
            ));
        }
    }
    if role == SubjectLabelRole::Operation
        && owner
            .properties
            .get("showOperationSignature")
            .is_some_and(|value| !value.is_boolean())
    {
        return Err(format!(
            "label option showOperationSignature on {} is unsupported",
            owner.id
        ));
    }
    if owner
        .properties
        .get("stereotypeDisplay")
        .is_some_and(|value| !matches!(value.as_str(), Some("none" | "label" | "icon" | "icon-label")))
    {
        return Err(format!(
            "stereotype label display on {} is unsupported",
            owner.id
        ));
    }
    Ok(LabelOptionsSource::View {
        view_id: owner.id.to_owned(),
    })
}
