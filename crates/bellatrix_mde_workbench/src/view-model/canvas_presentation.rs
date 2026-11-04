use bellatrix_design_system::Palette;
use bellatrix_mde_cordis::Geometry;
use bellatrix_mde_cordis::presentation_contract::{RenderStyle, UmlNotation};
use bellatrix_mde_cordis::projection::{ModelingProjection, View};

pub(super) struct CanvasBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub(super) fn resolve_bounds(
    geometry: &Geometry,
    zoom: f32,
    pan: (f32, f32),
) -> Option<CanvasBounds> {
    let (x, y, width, height) = match geometry {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => (*x, *y, *width, *height),
        Geometry::Layout { bounds } => (
            *bounds.x.value()?,
            *bounds.y.value()?,
            bounds.width.value()?.resolved()?,
            bounds.height.value()?.resolved()?,
        ),
        _ => return None,
    };
    if ![x, y, width, height].iter().all(|value| value.is_finite())
        || width < 0.0
        || height < 0.0
        || !zoom.is_finite()
        || zoom <= 0.0
    {
        return None;
    }
    let zoom = f64::from(zoom);
    let values = [
        x * zoom + f64::from(pan.0),
        y * zoom + f64::from(pan.1),
        width * zoom,
        height * zoom,
    ];
    if values.iter().any(|value| {
        !value.is_finite()
            || !(*value as f32).is_finite()
            || (*value != 0.0 && *value as f32 == 0.0)
    }) {
        return None;
    }
    let [x, y, width, height] = values.map(|value| value as f32);
    if !(x + width).is_finite() || !(y + height).is_finite() {
        return None;
    }
    Some(CanvasBounds {
        x,
        y,
        width,
        height,
    })
}

pub(super) fn render_style(view: &View, palette: Palette) -> RenderStyle {
    let mut style = view.render_style.clone();
    let matches_color = |color: Option<&str>, expected: &str| {
        color.is_some_and(|color| {
            color.eq_ignore_ascii_case(expected)
                || color.eq_ignore_ascii_case(&format!("{expected}ff"))
        })
    };
    // Neutral palette defaults are paint inputs, not the editor's theme. Keep
    // authored colors in the document and retain non-default color accents.
    let themed_surface = style.fill.is_none()
        || matches_color(style.fill.as_deref(), "#ffffff")
        || matches_color(style.fill.as_deref(), "#e2e2e2");
    if themed_surface {
        let fill = if matches!(&view.geometry, Geometry::Path { .. }) {
            palette.canvas
        } else if matches_color(style.fill.as_deref(), "#e2e2e2") {
            palette.panel_alt
        } else {
            palette.panel
        };
        style.fill = Some(format!("#{fill:06x}"));
        if matches_color(style.text_color.as_deref(), "#000000") {
            style.text_color = None;
        }
    }
    if (themed_surface || matches!(&view.geometry, Geometry::Path { .. }))
        && matches_color(style.stroke.as_deref(), "#000000")
    {
        style.stroke = None;
    }
    style
}

pub(super) fn ordered_views<'a>(section: &ModelingProjection, views: &'a [View]) -> Vec<&'a View> {
    let mut siblings = views.iter().collect::<Vec<_>>();
    siblings.sort_by_key(|view| view.z_index);
    let mut pending = siblings
        .into_iter()
        .rev()
        .map(|view| (view, false))
        .collect::<Vec<_>>();
    let mut ordered = Vec::new();
    while let Some((view, native_parent)) = pending.pop() {
        if !visible_on_canvas(view) {
            continue;
        }
        if !native_parent || !view.presentation.internal {
            ordered.push(view);
        }
        let native_parent = if view.presentation.internal {
            native_parent
        } else {
            uses_native_contents(section, view)
        };
        let mut children = view.children.iter().collect::<Vec<_>>();
        children.sort_by_key(|child| child.z_index);
        pending.extend(children.into_iter().rev().map(|child| (child, native_parent)));
    }
    ordered
}

fn uses_native_contents(section: &ModelingProjection, view: &View) -> bool {
    // Imported compartments remain in the document, but supported semantic
    // figures own their title and member rows just like newly created figures.
    let Some(element) = section
        .elements
        .iter()
        .find(|element| Some(element.id.as_str()) == view.subject.as_deref())
    else {
        return false;
    };
    if !matches!(element.kind.as_str(), "uml.class" | "uml.interface")
        || view.figure.notation != UmlNotation::ClassifierRectangle
        || view.entity_box.is_some()
    {
        return false;
    }
    let mut pending = view.children.iter().collect::<Vec<_>>();
    while let Some(child) = pending.pop() {
        if !child.presentation.internal || !child.presentation.visible {
            continue;
        }
        if child.figure.notation == UmlNotation::Label
            && let Some(text) = child
                .rendered_label
                .text
                .as_deref()
                .filter(|text| !text.is_empty())
        {
            let native_member = view.interface_icon.is_none()
                && child.subject.as_deref().is_some_and(|subject| {
                    section.elements.iter().any(|member| {
                        member.id == subject
                            && (element.template_parameters.iter().any(|id| id == subject)
                                && member.template_label == text
                                || element.members.iter().any(|id| id == subject)
                                    && view.member_labels.get(subject).map(String::as_str)
                                        == Some(text)
                                    && match member.kind.as_str() {
                                        "uml.attribute" => view.render_style.show_attributes,
                                        "uml.operation" => view.render_style.show_operations,
                                        "uml.reception" => view.render_style.show_receptions,
                                        _ => false,
                                    })
                    })
                });
            let native_header = view.presentation.name_label.as_deref() == Some(child.id.as_str())
                || view.figure.header.keyword.as_deref() == Some(text)
                || view.figure.header.applied_stereotype.as_deref() == Some(text);
            if !native_member && !native_header {
                return false;
            }
        }
        pending.extend(child.children.iter());
    }
    true
}

pub(super) fn has_authored_contents(section: &ModelingProjection, view: &View) -> bool {
    !uses_native_contents(section, view)
        && view.children.iter().any(|child| child.presentation.internal)
}

fn visible_on_canvas(view: &View) -> bool {
    view.presentation.visible
}

pub(super) fn minimum_size(
    section: &bellatrix_mde_cordis::projection::ModelingProjection,
    view: &View,
) -> (f64, f64) {
    if view.draft.is_some()
        || view.presentation.internal
        || has_authored_contents(section, view)
        || matches!(
            view.presentation.kind.as_deref(),
            Some("presentation.label" | "presentation.compartment" | "presentation.rectangle")
        )
    {
        return (1.0, 1.0);
    }
    section
        .elements
        .iter()
        .find(|element| Some(element.id.as_str()) == view.subject.as_deref())
        .map(|element| {
            (
                element.minimum_size.0.max(80.0),
                element
                    .minimum_size
                    .1
                    .max(48.0)
                    .max(element.minimum_header_height)
                    .max(element.minimum_interface_height.unwrap_or(0.0)),
            )
        })
        .unwrap_or((80.0, 48.0))
}
