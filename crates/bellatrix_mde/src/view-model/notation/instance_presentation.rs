//! UML 2.5.1 §9.8.4: instance headings and slot rows projected from canonical identities.
use crate::uml::{
    references::{ReferenceIndex, reference_id},
    root::ElementRef,
};
use crate::{Element, Style};

pub fn format_instance_heading(
    element: &Element,
    references: &ReferenceIndex<'_>,
    style: &Style,
) -> String {
    let Some(instance) = ElementRef::Object(element).instance_specification() else {
        return element.name.clone();
    };
    if !style.uml.should_show_type() {
        return element.name.clone();
    }
    let classifiers = instance.classifiers(references.model()).unwrap_or_default();
    let names = classifiers
        .into_iter()
        .filter_map(|classifier| classifier.named().and_then(|named| named.name()))
        .collect::<Vec<_>>();
    if names.is_empty() {
        element.name.clone()
    } else {
        format!("{} : {}", element.name, names.join(", "))
    }
}

/// Display stored literal values; this does not evaluate expressions or execute the model.
pub fn collect_instance_slot_labels(
    element: &Element,
    references: &ReferenceIndex<'_>,
) -> Vec<String> {
    let Some(instance) = ElementRef::Object(element).instance_specification() else {
        return Vec::new();
    };
    instance
        .slots(references.model())
        .unwrap_or_default()
        .into_iter()
        .map(|slot| {
            use crate::uml::root::Element as _;
            let name = slot
                .defining_feature(references.model())
                .ok()
                .and_then(|feature| feature.named())
                .and_then(|named| named.name())
                .unwrap_or("[unresolved feature]");
            let values = slot
                .as_element()
                .properties()
                .get("value")
                .and_then(serde_json::Value::as_array)
                .map(|values| {
                    values
                        .iter()
                        .map(|value| {
                            reference_id(value)
                                .ok()
                                .flatten()
                                .and_then(|id| references.element(id).ok())
                                .map(format_stored_value)
                                .unwrap_or_else(|| "[unresolved value]".into())
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if values.is_empty() {
                name.to_owned()
            } else {
                format!("{name} = {}", values.join(", "))
            }
        })
        .collect()
}

fn format_stored_value(element: ElementRef<'_>) -> String {
    if element.conforms_to("LiteralNull") {
        return "null".into();
    }
    if element.conforms_to("LiteralSpecification")
        && let Some(value) = element.properties().get("value")
    {
        return if element.conforms_to("LiteralString") {
            value.to_string()
        } else {
            value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string())
        };
    }
    element
        .named()
        .and_then(|named| named.name())
        .unwrap_or("[value specification]")
        .to_owned()
}
