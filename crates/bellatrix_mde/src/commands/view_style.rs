use crate::{Geometry, ModelingSection, RelationshipRouting, Style, View};

fn view<'a>(
    section: &'a ModelingSection,
    diagram_id: &str,
    view_id: &str,
) -> Result<&'a View, String> {
    section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?
        .views
        .iter()
        .find(|view| view.id == view_id)
        .ok_or_else(|| "view not found in diagram".into())
}

fn view_mut<'a>(
    section: &'a mut ModelingSection,
    diagram_id: &str,
    view_id: &str,
) -> Result<&'a mut View, String> {
    section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?
        .views
        .iter_mut()
        .find(|view| view.id == view_id)
        .ok_or_else(|| "view not found in diagram".into())
}

pub(super) fn set_style(
    section: &mut ModelingSection,
    diagram_id: &str,
    view_id: &str,
    style: &Style,
) -> Result<(), String> {
    let current = view(section, diagram_id, view_id)?;
    if current.style.routing != style.routing {
        return Err("change relationship routing with SetViewRouting".into());
    }
    for color in [&style.fill, &style.stroke, &style.text_color]
        .into_iter()
        .flatten()
    {
        let digits = color
            .strip_prefix('#')
            .ok_or("color must be #RRGGBB or #RRGGBBAA")?;
        if !matches!(digits.len(), 6 | 8) || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("color must be #RRGGBB or #RRGGBBAA".into());
        }
    }
    for (name, value, positive) in [
        ("stroke width", style.stroke_width, true),
        ("font size", style.font_size, true),
        ("corner radius", style.corner_radius, false),
    ] {
        if let Some(value) = value
            && (!value.is_finite() || value < 0.0 || positive && value == 0.0)
        {
            return Err(format!(
                "{name} must be finite and {}",
                if positive { "positive" } else { "nonnegative" }
            ));
        }
    }
    if style
        .opacity
        .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
    {
        return Err("opacity must be between zero and one".into());
    }
    if style
        .font_family
        .as_ref()
        .is_some_and(|value| value.trim().is_empty() || value.chars().any(char::is_control))
    {
        return Err("font family must be nonblank and contain no control characters".into());
    }
    if style
        .dash_pattern
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err("dash lengths must be finite and positive".into());
    }
    if !style.dash_pattern.is_empty()
        && section.relations.iter().any(|relation| {
            Some(relation.id.as_str()) == current.subject.as_deref()
                && relation.kind.starts_with("uml.")
        })
    {
        return Err("UML relationship dashing is defined by its semantic notation".into());
    }
    view_mut(section, diagram_id, view_id)?.style = style.clone();
    Ok(())
}

pub(super) fn set_routing(
    section: &mut ModelingSection,
    diagram_id: &str,
    view_id: &str,
    routing: RelationshipRouting,
) -> Result<(), String> {
    let current = view(section, diagram_id, view_id)?;
    if !matches!(current.geometry, Geometry::Path { .. }) {
        return Err("routing requires a relationship Path View".into());
    }
    let relation = section
        .relations
        .iter()
        .find(|relation| Some(relation.id.as_str()) == current.subject.as_deref())
        .ok_or("view does not represent a relationship")?;
    let geometry = crate::relationships::route_for_routing(section, diagram_id, relation, routing)?;
    let current = view_mut(section, diagram_id, view_id)?;
    current.geometry = geometry;
    current.style.routing = Some(routing);
    Ok(())
}

pub(super) fn reorder(
    section: &mut ModelingSection,
    diagram_id: &str,
    view_id: &str,
    order: super::ViewOrder,
) -> Result<(), String> {
    let diagram = section
        .diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?;
    let mut ordered: Vec<_> = diagram.views.iter().enumerate().collect();
    ordered.sort_by_key(|(_, view)| view.z_index);
    let position = ordered
        .iter()
        .position(|(_, view)| view.id == view_id)
        .ok_or("view not found in diagram")?;
    let destination = match order {
        super::ViewOrder::BringToFront => ordered.len().saturating_sub(1),
        super::ViewOrder::SendToBack => 0,
        super::ViewOrder::BringForward => (position + 1).min(ordered.len().saturating_sub(1)),
        super::ViewOrder::SendBackward => position.saturating_sub(1),
    };
    if position == destination {
        return Ok(());
    }
    let moved = ordered.remove(position);
    ordered.insert(destination, moved);
    // Normalize the entire local stack so duplicate or extreme imported indices remain reorderable.
    let indices: Vec<_> = ordered.into_iter().map(|(index, _)| index).collect();
    for (rank, index) in indices.into_iter().enumerate() {
        let rank = i32::try_from(rank).map_err(|_| "too many views to reorder")?;
        diagram.views[index].z_index = rank;
    }
    Ok(())
}
