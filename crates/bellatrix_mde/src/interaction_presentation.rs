//! UML 2.5.1 §17.6.4.1–3: display stored guard expressions without evaluating them.
use crate::{
    Element,
    uml::{
        references::{ReferenceIndex, reference_id},
        root::ElementRef,
    },
};

pub fn format_operand_guard(element: &Element, references: &ReferenceIndex<'_>) -> String {
    let operand = ElementRef::Object(element);
    let guard = resolve_owned_reference(operand, "guard", "InteractionConstraint", references);
    let Ok(Some(guard)) = guard else {
        return if guard.is_err() {
            "[unresolved guard]".into()
        } else {
            String::new()
        };
    };
    let Ok(Some(specification)) =
        resolve_owned_reference(guard, "specification", "ValueSpecification", references)
    else {
        return "[unresolved guard]".into();
    };
    let body = specification.properties().get("body").and_then(|value| {
        value
            .as_str()
            .or_else(|| value.as_array()?.first()?.as_str())
    });
    let expression = body
        .map(str::to_owned)
        .or_else(|| {
            specification.properties().get("value").map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| value.to_string())
            })
        })
        .or_else(|| {
            specification
                .named()
                .and_then(|named| named.name())
                .map(str::to_owned)
        });
    expression
        .filter(|text| !text.is_empty())
        .map_or_else(|| "[unresolved guard]".into(), |text| format!("[{text}]"))
}

fn resolve_owned_reference<'a>(
    element: ElementRef<'a>,
    property: &str,
    metaclass: &str,
    references: &ReferenceIndex<'a>,
) -> Result<Option<ElementRef<'a>>, String> {
    if let Some(value) = element.properties().get(property) {
        let Some(id) = reference_id(value)? else {
            return Ok(None);
        };
        let target = references.element(id)?;
        return if target.conforms_to(metaclass) {
            Ok(Some(target))
        } else {
            Err("unexpected guard reference type".into())
        };
    }
    let matches = references
        .model()
        .owned_elements(element)?
        .into_iter()
        .filter(|child| child.conforms_to(metaclass))
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [target] => Ok(Some(*target)),
        _ => Err("ambiguous guard ownership".into()),
    }
}
