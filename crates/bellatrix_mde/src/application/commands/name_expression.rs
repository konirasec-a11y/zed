use crate::{Element, ModelingSection, UmlType};

#[derive(Clone, Debug, PartialEq, Eq)]
struct NameExpression {
    name: String,
    stereotype: Option<String>,
    visibility: Option<&'static str>,
}

fn parse(expression: &str) -> Result<NameExpression, String> {
    let mut rest = expression.trim();
    let stereotype = if let Some(value) = rest.strip_prefix("<<") {
        let (text, next) = value.split_once(">>").ok_or("unclosed UML stereotype")?;
        let text = text.trim();
        if text.is_empty() {
            return Err("empty UML stereotype".into());
        }
        rest = next.trim_start();
        Some(text.to_owned())
    } else if let Some(value) = rest.strip_prefix('«') {
        let (text, next) = value.split_once('»').ok_or("unclosed UML stereotype")?;
        let text = text.trim();
        if text.is_empty() {
            return Err("empty UML stereotype".into());
        }
        rest = next.trim_start();
        Some(text.to_owned())
    } else {
        None
    };
    let visibility = match rest.chars().next() {
        Some('+') => Some("public"),
        Some('#') => Some("protected"),
        Some('-') => Some("private"),
        Some('~') => Some("package"),
        _ => None,
    };
    if visibility.is_some() {
        rest = rest[1..].trim_start();
    }
    let name = rest.trim();
    if name.contains("<<") || name.contains(">>") {
        return Err("UML name expression has malformed stereotype".into());
    }
    Ok(NameExpression {
        name: name.into(),
        stereotype,
        visibility,
    })
}

pub(super) fn apply(
    section: &mut ModelingSection,
    id: &str,
    expression: &str,
) -> Result<(), String> {
    let parsed = parse(expression)?;
    let element = section
        .elements
        .iter_mut()
        .find(|element| element.id == id)
        .ok_or("element not found")?;
    if !UmlType::from_kind(&element.kind).is_some_and(UmlType::is_classifier) {
        return Err("UML name expression requires a classifier".into());
    }
    element.name = parsed.name;
    if let Some(stereotype) = parsed.stereotype {
        element
            .properties
            .insert("stereotype".into(), serde_json::json!(stereotype));
    } else if element
        .properties
        .get("stereotype")
        .is_some_and(serde_json::Value::is_string)
    {
        element.properties.remove("stereotype");
    }
    if let Some(visibility) = parsed.visibility {
        element
            .properties
            .insert("visibility".into(), serde_json::json!(visibility));
    }
    Ok(())
}

pub fn format(element: &Element) -> String {
    let stereotype = element
        .properties
        .get("stereotype")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map_or_else(String::new, |value| format!("<< {value} >> "));
    let visibility = element
        .properties
        .get("visibility")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| match value {
            "protected" => Some("# "),
            "private" => Some("- "),
            "package" => Some("~ "),
            _ => None,
        })
        .unwrap_or("");
    format!("{stereotype}{visibility}{}", element.name)
}
