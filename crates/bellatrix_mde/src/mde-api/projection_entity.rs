use crate::{Diagram, ModelingSection, RelationshipNotation, View};
use bellatrix_mde_protocol::{
    command::TextAlignment,
    presentation::RenderStyle,
    projection::{EntityBoxProjection, RenderedLabel},
};

pub(super) fn project(
    section: &ModelingSection,
    diagram: &Diagram,
    view: &View,
    label: &mut RenderedLabel,
    style: &mut RenderStyle,
) -> Option<EntityBoxProjection> {
    if view.presentation.relationship_notation == Some(RelationshipNotation::EntityRelationship) {
        if let Some(subject) = view.subject.as_deref().and_then(|id| section.elements.iter().find(|element| element.id == id)) {
            match subject.properties.get("identifying") {
                None | Some(serde_json::Value::Bool(true)) => style.stroke_pattern = Some(Vec::new()),
                Some(serde_json::Value::Bool(false)) => style.stroke_pattern = Some(vec![3.0]),
                _ => {},
            }
        }
    }
    let owners = diagram.all_views().filter(|owner| {
        owner.presentation.entity_box.as_ref().and_then(|binding| binding.name_label.value())
            .is_some_and(|id| id == &view.id)
    }).collect::<Vec<_>>();
    if !owners.is_empty() {
        label.bound = true;
        match owners.as_slice() {
            [owner] if owner.children.iter().any(|child| child.id == view.id) => {
                if let Some(subject) = owner.subject.as_deref().and_then(|id| section.elements.iter().find(|element| element.id == id)) {
                    label.text = Some(subject.name.clone());
                    style.font_weight = Some(700);
                    style.font_italic = Some(false);
                    style.text_align.get_or_insert(TextAlignment::Center);
                } else {
                    label.diagnostics.push(format!("View {}: entity name subject is unresolved; preserved text is a fallback", view.id));
                }
            }
            _ => label.diagnostics.push(format!("View {}: entity name role is ambiguous or outside its owner", view.id)),
        }
    }
    view.presentation.entity_box.as_ref().map(|binding| {
        let mut projected = EntityBoxProjection::default();
        match binding.name_label.value().and_then(|id| view.children.iter().find(|child| child.id == *id)) {
            Some(_) => {},
            None => projected.diagnostics.push(format!("View {}: entity name label binding is unresolved", view.id)),
        }
        match binding.column_compartment.value().and_then(|id| view.children.iter().find(|child| child.id == *id)) {
            Some(compartment) if super::visible_in_preview(compartment) && !compartment.children.is_empty() => {
                projected.diagnostics.push(format!("View {}: populated entity columns and measured column dividers are not rendered", view.id));
            }
            Some(_) => {},
            None => projected.diagnostics.push(format!("View {}: entity column compartment binding is unresolved", view.id)),
        }
        projected
    })
}
