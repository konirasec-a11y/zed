use bellatrix_mde::{Geometry, View};

pub(super) struct CanvasBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub(super) fn resolve_layout_bounds(
    geometry: &Geometry,
    zoom: f32,
    pan: (f32, f32),
) -> Option<CanvasBounds> {
    let Geometry::Layout { bounds } = geometry else {
        return None;
    };
    let (x, y, width, height) = (
        *bounds.x.value()?,
        *bounds.y.value()?,
        *bounds.width.value()?,
        *bounds.height.value()?,
    );
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

pub(super) enum ImportedShape {
    Rectangle,
    Label,
}

pub(super) fn imported_shape(view: &View) -> Option<ImportedShape> {
    match view.presentation.kind.as_deref()? {
        "RectangleView" | "UMLClassView" => Some(ImportedShape::Rectangle),
        "LabelView" | "NodeLabelView" | "EdgeLabelView" => Some(ImportedShape::Label),
        _ => None,
    }
}

pub(super) fn ordered_views(views: &[View]) -> Vec<&View> {
    let mut siblings = views.iter().collect::<Vec<_>>();
    siblings.sort_by_key(|view| view.z_index);
    let mut pending = siblings.into_iter().rev().collect::<Vec<_>>();
    let mut ordered = Vec::new();
    while let Some(view) = pending.pop() {
        if !visible_in_preview(view) {
            continue;
        }
        ordered.push(view);
        let mut children = view.children.iter().collect::<Vec<_>>();
        children.sort_by_key(|child| child.z_index);
        pending.extend(children.into_iter().rev());
    }
    ordered
}

fn visible_in_preview(view: &View) -> bool {
    let Some(source) = view
        .presentation
        .source
        .as_ref()
        .filter(|source| source.format == "mdj")
    else {
        return true;
    };
    match source.residual.get("visible") {
        // StarUML View initializes visible=true (core.js:1492); load preserves it
        // for an absent property. This derived default must not enter the document.
        None => true,
        Some(serde_json::Value::Bool(visible)) => *visible,
        // Reader.read and Element.load retain null, which is falsy when drawing.
        Some(serde_json::Value::Null) => false,
        // Other values are not interpreted as the missing-property default.
        Some(_) => false,
    }
}
