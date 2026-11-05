use crate::{Authored, Geometry, LayoutExtent, StereotypeDisplay, View};

pub(super) fn project(view: &View, parent: Option<&View>) -> Option<Geometry> {
    let source = view.presentation.source.as_ref()?;
    if source.format != "mdj"
        || ["left", "top", "width", "height"]
            .iter()
            .any(|field| source.residual.contains_key(*field))
    {
        return None;
    }
    let Geometry::Layout { bounds } = &view.geometry else {
        return None;
    };
    let x = *bounds.x.value()?;
    let y = *bounds.y.value()?;
    if matches!(
        source.type_name.as_str(),
        "UMLClassView" | "UMLStereotypeView" | "UMLMetaClassView"
    ) && source
        .residual
        .get("autoResize")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
        && !source.residual.contains_key("stereotypeDisplay")
        && matches!(
            view.style.uml.stereotype_display,
            None | Some(StereotypeDisplay::Hidden | StereotypeDisplay::Label)
        )
    {
        let (width, height) = compartment_bounds(view, x, y)?;
        return rectangle(
            x,
            y,
            extent(&bounds.width, width)?,
            extent(&bounds.height, height)?,
        );
    }
    let parent = parent?;
    let parent_source = parent.presentation.source.as_ref()?;
    if parent_source.format != "mdj"
        || !matches!(
            (parent_source.type_name.as_str(), source.type_name.as_str()),
            ("UMLNameCompartmentView", "LabelView")
                | ("UMLAttributeCompartmentView", "UMLAttributeView")
                | ("UMLOperationCompartmentView", "UMLOperationView")
                | ("UMLReceptionCompartmentView", "UMLReceptionView")
        )
    {
        return None;
    }
    let Geometry::Rect {
        x: left,
        y: top,
        width,
        height,
    } = parent.geometry
    else {
        return None;
    };
    let label_height = bounds.height.value()?.resolved()?;
    // StarUML's UMLCompartmentView.arrange assigns width minus two 5px
    // paddings. Reuse the stored placement and height, not guessed text metrics.
    if x != left + 5.0 || y < top + 5.0 || y + label_height > top + height - 5.0 {
        return None;
    }
    rectangle(x, y, extent(&bounds.width, width - 10.0)?, label_height)
}

fn compartment_bounds(view: &View, x: f64, y: f64) -> Option<(f64, f64)> {
    let source = view.presentation.source.as_ref()?;
    let mut compartments = Vec::new();
    for (field, kind) in [
        ("nameCompartment", "UMLNameCompartmentView"),
        ("attributeCompartment", "UMLAttributeCompartmentView"),
        ("operationCompartment", "UMLOperationCompartmentView"),
        ("receptionCompartment", "UMLReceptionCompartmentView"),
    ] {
        let Some(binding) = source.residual.get(field) else {
            if field == "nameCompartment" {
                return None;
            }
            continue;
        };
        let reference = binding.as_object().filter(|object| object.len() == 1)?;
        let id = reference.get("$ref")?.as_str()?;
        let compartment = view.children.iter().find(|child| child.id == id)?;
        let compartment_source = compartment.presentation.source.as_ref()?;
        if compartment_source.format != "mdj"
            || compartment_source.type_name != kind
            || compartment_source.residual.contains_key("visible")
        {
            return None;
        }
        if compartment.presentation.visible != Some(false) {
            compartments.push(compartment);
        }
    }
    if compartments.is_empty()
        || view.children.iter().any(|child| {
            child.presentation.visible != Some(false)
                && !compartments
                    .iter()
                    .any(|compartment| compartment.id == child.id)
        })
    {
        return None;
    }
    let mut width = 0.0_f64;
    let mut bottom = y;
    for compartment in compartments {
        let Geometry::Rect {
            x: left,
            y: top,
            width: compartment_width,
            height: compartment_height,
        } = compartment.geometry
        else {
            return None;
        };
        rectangle(left, top, compartment_width, compartment_height)?;
        if left != x || top != bottom || compartment_width <= 0.0 || compartment_height <= 0.0 {
            return None;
        }
        width = width.max(compartment_width);
        bottom += compartment_height;
    }
    // The saved compartments already establish this content envelope. They are
    // not necessarily fresh font-measured minima, so do not apply another
    // StarUML arrange pass (including its inclusive-width +1) or persist it.
    Some((width, bottom - y))
}

fn extent(authored: &Authored<LayoutExtent>, content: f64) -> Option<f64> {
    match authored {
        Authored::Missing | Authored::Value(LayoutExtent::Unresolved { .. }) => Some(content),
        Authored::Value(extent) => extent.resolved(),
        Authored::Null => None,
    }
}

fn rectangle(x: f64, y: f64, width: f64, height: f64) -> Option<Geometry> {
    ([x, y, width, height, x + width, y + height]
        .iter()
        .all(|value| value.is_finite())
        && width >= 0.0
        && height >= 0.0)
        .then_some(Geometry::Rect {
            x,
            y,
            width,
            height,
        })
}
