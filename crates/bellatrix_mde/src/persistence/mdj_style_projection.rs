use super::{failure, insert_mapped};
use crate::{
    Style, TextAlignment, TextVerticalAlignment, View,
    mdj::{Diagnostic, MdjError},
};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

const STYLE_FIELDS: [&str; 9] = [
    "fillColor",
    "lineColor",
    "fontColor",
    "font",
    "underline",
    "wordWrap",
    "showShadow",
    "horizontalAlignment",
    "verticalAlignment",
];

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
    ] {
        if let Some(value) = value {
            insert_mapped(object, field, Value::Bool(value))?;
        }
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
    if source
        .residual
        .get("parentStyle")
        .is_some_and(|value| value.as_bool() != Some(false))
    {
        diagnostics.push(Diagnostic {
            path: format!("{path}/parentStyle"),
            message: "source style inheritance is preserved but not resolved; typed style describes explicit authored values, not final inherited or computed appearance".into(),
        });
    }
}
