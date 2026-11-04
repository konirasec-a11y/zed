use crate::{Diagram, ModelingSection, RelationshipNotation, View};
use bellatrix_mde_contracts::protocol::{
    command::TextAlignment,
    presentation::{RenderShadow, RenderStyle},
    projection::{EntityBoxProjection, RenderedLabel},
};

pub(super) fn project(
    section: &ModelingSection,
    diagram: &Diagram,
    view: &View,
    label: &mut RenderedLabel,
    style: &mut RenderStyle,
) -> Option<EntityBoxProjection> {
    let owners = diagram
        .all_views()
        .filter(|owner| {
            owner
                .presentation
                .entity_box
                .as_ref()
                .and_then(|binding| binding.name_label.value())
                .is_some_and(|id| id == &view.id)
        })
        .collect::<Vec<_>>();
    if !owners.is_empty() {
        label.bound = true;
        match owners.as_slice() {
            [owner] if owner.children.iter().any(|child| child.id == view.id) => {
                if let Some(subject) = owner
                    .subject
                    .as_deref()
                    .and_then(|id| section.elements.iter().find(|element| element.id == id))
                {
                    label.text = Some(subject.name.clone());
                    style.font_weight = Some(700);
                    style.font_italic = Some(false);
                    style.text_align.get_or_insert(TextAlignment::Center);
                } else {
                    label.diagnostics.push(format!(
                        "View {}: entity name subject is unresolved; preserved text is a fallback",
                        view.id
                    ));
                }
            }
            _ => label.diagnostics.push(format!(
                "View {}: entity name role is ambiguous or outside its owner",
                view.id
            )),
        }
    }
    project_relationship_label(section, diagram, view, label);
    view.presentation.entity_box.as_ref().map(|binding| {
        if style.show_shadow == Some(true) {
            style.shadow = Some(RenderShadow {
                offset_x: 7.0,
                offset_y: 7.0,
                blur_radius: 0.0,
                color: "#c0c0c033".into(),
            });
        }
        let mut projected = EntityBoxProjection::default();
        match binding.name_label.value().and_then(|id| view.children.iter().find(|child| child.id == *id)) {
            Some(_) => {},
            None => projected.diagnostics.push(format!("View {}: entity name label binding is unresolved", view.id)),
        }
        match binding.column_compartment.value().and_then(|id| view.children.iter().find(|child| child.id == *id)) {
            Some(compartment) if super::visible_on_canvas(compartment) && !compartment.children.is_empty() => {
                projected.diagnostics.push(format!("View {}: populated entity columns and measured column dividers are not rendered", view.id));
            }
            Some(_) => {},
            None => projected.diagnostics.push(format!("View {}: entity column compartment binding is unresolved", view.id)),
        }
        projected
    })
}

fn project_relationship_label(
    section: &ModelingSection,
    diagram: &Diagram,
    view: &View,
    label: &mut RenderedLabel,
) {
    let mut bindings = Vec::new();
    for owner in diagram.all_views().filter(|owner| {
        owner.presentation.relationship_notation == Some(RelationshipNotation::EntityRelationship)
    }) {
        let Some(labels) = &owner.presentation.relationship_labels else {
            continue;
        };
        for (role, id) in [
            (0, &labels.name),
            (1, &labels.source_name),
            (2, &labels.target_name),
        ] {
            if id.value().is_some_and(|id| id == &view.id) {
                bindings.push((owner, role));
            }
        }
    }
    if bindings.is_empty() {
        return;
    }
    if label.bound || bindings.len() != 1 {
        label.bound = true;
        label.diagnostics.push(format!(
            "View {}: relationship label role is ambiguous",
            view.id
        ));
        return;
    }
    label.bound = true;
    let (owner, role) = bindings[0];
    let result = (|| {
        if !owner.children.iter().any(|child| child.id == view.id) {
            return Err("relationship label is not a direct child".to_owned());
        }
        let subject = if role == 0 {
            owner.subject.as_deref()
        } else {
            owner
                .presentation
                .relationship_endpoints
                .as_ref()
                .and_then(|ends| {
                    let binding = if role == 1 {
                        &ends.source
                    } else {
                        &ends.target
                    };
                    match &binding.target {
                        crate::EndpointMarkerTarget::ModelEnd { id } => Some(id.as_str()),
                        _ => None,
                    }
                })
        };
        let element = subject
            .and_then(|id| section.elements.iter().find(|element| element.id == id))
            .ok_or("relationship label subject is unresolved")?;
        Ok(element.name.clone())
    })();
    match result {
        Ok(text) => {
            if role == 0 {
                label.visible = Some(!text.is_empty());
            }
            label.text = Some(text);
        }
        Err(error) => label.diagnostics.push(format!(
            "View {}: {error}; preserved text is a fallback",
            view.id
        )),
    }
}
