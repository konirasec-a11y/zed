use super::{failure, insert_mapped};
use crate::{
    Authored, InheritedStyleProperty, SourceFieldPresence, Style, StyleInheritance,
    TextAlignment, TextVerticalAlignment, View,
    mdj::{Diagnostic, MdjError},
};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

const STYLE_FIELDS: [&str; 15] = [
    "fillColor",
    "lineColor",
    "fontColor",
    "font",
    "underline",
    "wordWrap",
    "showShadow",
    "horizontalAlignment",
    "verticalAlignment",
    "showVisibility",
    "showType",
    "showProperty",
    "showMultiplicity",
    "showOperationSignature",
    "stereotypeDisplay",
];

const PARENT_STYLE_PROPERTIES: [InheritedStyleProperty; 7] = [
    InheritedStyleProperty::Fill,
    InheritedStyleProperty::Stroke,
    InheritedStyleProperty::TextColor,
    InheritedStyleProperty::FontFamily,
    InheritedStyleProperty::FontSize,
    InheritedStyleProperty::FontWeight,
    InheritedStyleProperty::FontItalic,
];

pub(super) fn take_inheritance(
    type_name: &str,
    residual: &mut BTreeMap<String, Value>,
    field_presence: &mut BTreeMap<String, SourceFieldPresence>,
) -> Option<StyleInheritance> {
    let (enabled, presence) = match residual.get("parentStyle") {
        None => (default_parent_style(type_name), None),
        Some(Value::Bool(enabled)) => (*enabled, Some(SourceFieldPresence::Value)),
        Some(Value::Null) => (false, Some(SourceFieldPresence::Null)),
        Some(_) => return None,
    };
    let source = match residual.get("_parent") {
        None => Authored::Missing,
        Some(Value::Null) => Authored::Null,
        Some(Value::Object(reference)) if reference.len() == 1 => {
            Authored::Value(reference.get("$ref")?.as_str()?.to_owned())
        }
        Some(_) => return None,
    };
    residual.remove("parentStyle");
    residual.remove("_parent");
    if let Some(presence) = presence {
        field_presence.insert("parentStyle".into(), presence);
    }
    Some(StyleInheritance {
        source,
        properties: if enabled { PARENT_STYLE_PROPERTIES.into_iter().collect() } else { Default::default() },
    })
}

fn default_parent_style(type_name: &str) -> bool {
    // Vendored constructors, not naming conventions: EdgeLabelView and
    // NodeLabelView do not inherit LabelView's default. Omission keeps the
    // constructor value; explicit null does not (core.js View.setup/load).
    matches!(type_name,
        "LabelView"
        | "ERDColumnView" | "ERDColumnCompartmentView"
        | "UMLCompartmentView" | "UMLNameCompartmentView" | "UMLListCompartmentView"
        | "UMLAttributeView" | "UMLAttributeCompartmentView"
        | "UMLOperationView" | "UMLOperationCompartmentView"
        | "UMLReceptionView" | "UMLReceptionCompartmentView"
        | "UMLTemplateParameterView" | "UMLTemplateParameterCompartmentView"
        | "UMLEnumerationLiteralView" | "UMLEnumerationLiteralCompartmentView"
        | "UMLQualifierCompartmentView" | "UMLSlotView" | "UMLSlotCompartmentView"
        | "UMLExtensionPointView" | "UMLExtensionPointCompartmentView"
        | "UMLInternalActivityView" | "UMLInternalActivityCompartmentView"
        | "UMLInternalTransitionView" | "UMLInternalTransitionCompartmentView"
        | "UMLRegionView" | "UMLDecompositionCompartmentView"
        | "UMLInteractionOperandView" | "UMLInteractionOperandCompartmentView"
        | "UMLTimeConstraintView"
        | "SysMLConstraintCompartmentItemView" | "SysMLConstraintCompartmentView"
        | "SysMLFlowPropertyCompartmentItemView" | "SysMLFlowPropertyCompartmentView"
        | "SysMLParameterCompartmentItemView" | "SysMLParameterCompartmentView"
        | "SysMLPartCompartmentItemView" | "SysMLPartCompartmentView"
        | "SysMLPortCompartmentItemView" | "SysMLPortCompartmentView"
        | "SysMLPropertyCompartmentItemView" | "SysMLPropertyCompartmentView"
        | "SysMLPropertyValueCompartmentItemView" | "SysMLPropertyValueCompartmentView"
        | "SysMLReferenceCompartmentItemView" | "SysMLReferenceCompartmentView"
        | "SysMLValueCompartmentItemView" | "SysMLValueCompartmentView"
    )
}

fn inheritance_presence(view: &View) -> Option<SourceFieldPresence> {
    let source = view.presentation.source.as_ref()?;
    let inheritance = view.presentation.style_inheritance.as_ref()?;
    let enabled = !inheritance.properties.is_empty();
    match source.field_presence.get("parentStyle").copied() {
        Some(SourceFieldPresence::Null) if !enabled => Some(SourceFieldPresence::Null),
        Some(_) => Some(SourceFieldPresence::Value),
        None if enabled != default_parent_style(&source.type_name) => Some(SourceFieldPresence::Value),
        None => None,
    }
}

pub(super) fn update_inheritance_metadata(views: &mut [View]) {
    for view in views {
        if view.presentation.style_inheritance.is_some() {
            let presence = inheritance_presence(view);
            if let Some(source) = &mut view.presentation.source {
                if let Some(presence) = presence {
                    source.field_presence.insert("parentStyle".into(), presence);
                } else {
                    source.field_presence.remove("parentStyle");
                }
            }
        }
        update_inheritance_metadata(&mut view.children);
    }
}

fn export_inheritance(view: &View, object: &mut Map<String, Value>) -> Result<(), MdjError> {
    let Some(inheritance) = &view.presentation.style_inheritance else {
        return Ok(());
    };
    if !inheritance.properties.is_empty() && !inheritance.properties.iter().copied().eq(PARENT_STYLE_PROPERTIES) {
        return Err(failure(&view.id, "MDJ parentStyle requires exactly the supported color and font inheritance properties"));
    }
    match &inheritance.source {
        Authored::Missing => {}
        Authored::Null => insert_mapped(object, "_parent", Value::Null)?,
        Authored::Value(id) => insert_mapped(object, "_parent", serde_json::json!({"$ref": id}))?,
    }
    match inheritance_presence(view) {
        Some(SourceFieldPresence::Null) => insert_mapped(object, "parentStyle", Value::Null)?,
        Some(SourceFieldPresence::Value) => insert_mapped(object, "parentStyle", Value::Bool(!inheritance.properties.is_empty()))?,
        None => {}
    }
    Ok(())
}

pub(super) fn take_style(residual: &mut BTreeMap<String, Value>) -> Style {
    let mut style = Style {
        fill: take_color(residual, "fillColor"),
        stroke: take_color(residual, "lineColor"),
        text_color: take_color(residual, "fontColor"),
        font_underline: take_boolean(residual, "underline"),
        word_wrap: take_boolean(residual, "wordWrap"),
        show_shadow: take_boolean(residual, "showShadow"),
        ..Style::default()
    };
    style.uml.show_visibility = take_boolean(residual, "showVisibility");
    style.uml.show_type = take_boolean(residual, "showType");
    style.uml.show_property = take_boolean(residual, "showProperty");
    style.uml.show_multiplicity = take_boolean(residual, "showMultiplicity");
    style.uml.show_operation_signature = take_boolean(residual, "showOperationSignature");
    style.uml.stereotype_display = match residual.get("stereotypeDisplay").and_then(Value::as_str) {
        Some("none") => Some(crate::StereotypeDisplay::Hidden),
        Some("label") => Some(crate::StereotypeDisplay::Label),
        Some("icon") => Some(crate::StereotypeDisplay::Icon),
        Some("icon-label") => Some(crate::StereotypeDisplay::IconLabel),
        _ => None,
    };
    if style.uml.stereotype_display.is_some() {
        residual.remove("stereotypeDisplay");
    }
    if let Some(font) = residual
        .get("font")
        .and_then(Value::as_str)
        .and_then(read_font)
    {
        style.font_family = Some(font.family);
        style.font_size = Some(font.size);
        style.font_weight = Some(font.weight);
        style.font_italic = Some(font.italic);
        residual.remove("font");
    }
    style.text_align = match residual.get("horizontalAlignment").and_then(Value::as_i64) {
        Some(0) => Some(TextAlignment::Left),
        Some(1) => Some(TextAlignment::Right),
        Some(2) => Some(TextAlignment::Center),
        _ => None,
    };
    if style.text_align.is_some() {
        residual.remove("horizontalAlignment");
    }
    style.text_vertical_align = match residual.get("verticalAlignment").and_then(Value::as_i64) {
        Some(3) => Some(TextVerticalAlignment::Top),
        Some(4) => Some(TextVerticalAlignment::Bottom),
        Some(5) => Some(TextVerticalAlignment::Center),
        _ => None,
    };
    if style.text_vertical_align.is_some() {
        residual.remove("verticalAlignment");
    }
    style
}

fn take_color(residual: &mut BTreeMap<String, Value>, field: &str) -> Option<String> {
    let color = residual.get(field)?.as_str()?;
    let digits = color.strip_prefix('#')?;
    if !matches!(digits.len(), 6 | 8) || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let color = color.to_owned();
    residual.remove(field);
    Some(color)
}

fn take_boolean(residual: &mut BTreeMap<String, Value>, field: &str) -> Option<bool> {
    let value = residual.get(field)?.as_bool()?;
    residual.remove(field);
    Some(value)
}

struct Font {
    family: String,
    size: f64,
    weight: u16,
    italic: bool,
}

fn read_font(encoded: &str) -> Option<Font> {
    // StarUML Font.__read uses '?' to retain environment-dependent defaults.
    // Project only complete fonts, never manufacture those missing values.
    let mut fields = encoded.split(';');
    let family = fields.next()?;
    let encoded_size = fields.next()?;
    let encoded_flags = fields.next()?;
    if fields.next().is_some() || !supported_family(family) {
        return None;
    }
    let size = encoded_size.parse::<f64>().ok()?;
    // This custom field is a string, not a JSON number. Keep alternate spelling
    // in residue so an unrelated edit cannot rewrite "13.00" into "13".
    if !size.is_finite() || !(1.0..=400.0).contains(&size) || encoded_size != size.to_string() {
        return None;
    }
    let (weight, italic) = match encoded_flags {
        "0" => (400, false),
        "1" => (700, false),
        "2" => (400, true),
        "3" => (700, true),
        _ => return None,
    };
    Some(Font {
        family: family.into(),
        size,
        weight,
        italic,
    })
}

fn supported_family(family: &str) -> bool {
    family != "?"
        && !family.trim().is_empty()
        && !family
            .chars()
            .any(|character| character.is_control() || character == ';')
}

pub(super) fn export_style(view: &View, object: &mut Map<String, Value>) -> Result<(), MdjError> {
    export_inheritance(view, object)?;
    let style = &view.style;
    for (field, value) in [
        ("fillColor", &style.fill),
        ("lineColor", &style.stroke),
        ("fontColor", &style.text_color),
    ] {
        if let Some(value) = value {
            insert_mapped(object, field, Value::String(value.clone()))?;
        }
    }
    for (field, value) in [
        ("underline", style.font_underline),
        ("wordWrap", style.word_wrap),
        ("showShadow", style.show_shadow),
        ("showVisibility", style.uml.show_visibility),
        ("showType", style.uml.show_type),
        ("showProperty", style.uml.show_property),
        ("showMultiplicity", style.uml.show_multiplicity),
        ("showOperationSignature", style.uml.show_operation_signature),
    ] {
        if let Some(value) = value {
            insert_mapped(object, field, Value::Bool(value))?;
        }
    }
    if let Some(display) = style.uml.stereotype_display {
        let value = match display {
            crate::StereotypeDisplay::Hidden => "none",
            crate::StereotypeDisplay::Label => "label",
            crate::StereotypeDisplay::Icon => "icon",
            crate::StereotypeDisplay::IconLabel => "icon-label",
        };
        insert_mapped(object, "stereotypeDisplay", Value::String(value.into()))?;
    }
    if let Some(alignment) = style.text_align {
        let value = match alignment {
            TextAlignment::Left => 0,
            TextAlignment::Right => 1,
            TextAlignment::Center => 2,
        };
        insert_mapped(object, "horizontalAlignment", Value::from(value))?;
    }
    if let Some(alignment) = style.text_vertical_align {
        let value = match alignment {
            TextVerticalAlignment::Top => 3,
            TextVerticalAlignment::Bottom => 4,
            TextVerticalAlignment::Center => 5,
        };
        insert_mapped(object, "verticalAlignment", Value::from(value))?;
    }
    if style.font_family.is_some()
        || style.font_size.is_some()
        || style.font_weight.is_some()
        || style.font_italic.is_some()
    {
        let (Some(family), Some(size), Some(weight), Some(italic)) = (
            style.font_family.as_deref(),
            style.font_size,
            style.font_weight,
            style.font_italic,
        ) else {
            return Err(failure(
                &view.id,
                "MDJ font editing requires family, size, weight and italic together; implicit font defaults cannot be invented",
            ));
        };
        let flags = match (weight, italic) {
            (400, false) => 0,
            (700, false) => 1,
            (400, true) => 2,
            (700, true) => 3,
            _ => {
                return Err(failure(
                    &view.id,
                    "MDJ font supports only normal (400) or bold (700) weight",
                ));
            }
        };
        if !supported_family(family) || !size.is_finite() || !(1.0..=400.0).contains(&size) {
            return Err(failure(
                &view.id,
                "font cannot be represented by the supported MDJ style projection",
            ));
        }
        insert_mapped(
            object,
            "font",
            Value::String(format!("{family};{size};{flags}")),
        )?;
    }
    Ok(())
}

pub(super) fn diagnose(view: &View, path: &str, diagnostics: &mut Vec<Diagnostic>) {
    let Some(source) = &view.presentation.source else {
        return;
    };
    for field in STYLE_FIELDS {
        if source.residual.contains_key(field) {
            diagnostics.push(Diagnostic {
                path: format!("{path}/{field}"),
                message: "authored style retained as source residue: null, partial or unsupported encoding; no style default was substituted".into(),
            });
        }
    }
    for field in ["parentStyle", "_parent"] {
        if source.residual.contains_key(field) {
            diagnostics.push(Diagnostic {
                path: format!("{path}/{field}"),
                message: "style inheritance encoding is unsupported and retained as source residue; no inheritance was inferred from containment".into(),
            });
        }
    }
}
