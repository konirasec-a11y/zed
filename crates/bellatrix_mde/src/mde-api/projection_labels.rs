use crate::uml::{
    operations, references::ReferenceIndex, root::ElementRef, schema, value_specifications,
};
use crate::{Element, LabelOptionsSource, ModelingSection, SubjectLabelRole, UmlStyle, View};
use bellatrix_mde_contracts::protocol::{
    command::TextAlignment, presentation::RenderStyle, projection::RenderedLabel,
};
use serde_json::Value;

pub(super) fn project(
    section: &ModelingSection,
    diagram: &crate::Diagram,
    view: &View,
    references: &ReferenceIndex<'_>,
    style: &mut RenderStyle,
) -> RenderedLabel {
    let mut rendered = RenderedLabel {
        bound: view.presentation.label_binding.is_some(),
        visible: None,
        text: view.label.clone(),
        diagnostics: Vec::new(),
    };
    let name_owners = diagram
        .all_views()
        .filter(|owner| {
            !owner.presentation.internal
                && owner.presentation.name_label.as_deref() == Some(view.id.as_str())
        })
        .collect::<Vec<_>>();
    if !name_owners.is_empty() {
        rendered.bound = true;
        match name_owners.as_slice() {
            [owner] => {
                if let Some(element) = owner.subject.as_deref().and_then(|subject| {
                    section
                        .elements
                        .iter()
                        .find(|element| element.id == subject)
                }) {
                    rendered.text = Some(element.name.clone());
                    style.font_weight = Some(700);
                    style.text_align.get_or_insert(TextAlignment::Center);
                } else {
                    rendered
                        .diagnostics
                        .push(format!("View {}: name subject is unresolved", view.id));
                }
            }
            _ => rendered
                .diagnostics
                .push(format!("View {}: name label has multiple owners", view.id)),
        }
        return rendered;
    }
    let Some(binding) = &view.presentation.label_binding else {
        return rendered;
    };
    // Opaque members retain authored labels without acquiring canonical UML semantics.
    // This depends on their contract, never on retained source provenance.
    if rendered.text.is_some()
        && section.elements.iter().any(|element| {
            view.subject.as_deref() == Some(element.id.as_str())
                && matches!(
                    (binding.role, element.kind.as_str()),
                    (SubjectLabelRole::Attribute, "mdj.UMLAttribute")
                        | (SubjectLabelRole::Operation, "mdj.UMLOperation")
                )
        })
    {
        return rendered;
    }
    let result = (|| {
        let element = view
            .subject
            .as_deref()
            .and_then(|subject| {
                section
                    .elements
                    .iter()
                    .find(|element| element.id == subject)
            })
            .ok_or_else(|| {
                "bound label subject is unresolved; preserved text is only a fallback".to_owned()
            })?;
        let expected = match binding.role {
            SubjectLabelRole::Attribute => "Property",
            SubjectLabelRole::Operation => "Operation",
        };
        if !ElementRef::Object(element).conforms_to(expected) {
            return Err(format!("bound label requires a {expected} subject"));
        }
        let defaults = UmlStyle::default();
        let options = match &binding.options_source {
            LabelOptionsSource::Defaults => &defaults,
            LabelOptionsSource::View { view_id } => {
                &diagram
                    .all_views()
                    .find(|source| source.id == *view_id)
                    .ok_or_else(|| format!("label options view {view_id} is unresolved"))?
                    .style
                    .uml
            }
            LabelOptionsSource::Unresolved { reason } => return Err(reason.clone()),
        };
        let label = member_label(section, element, binding.role, references, options)?;
        let is_static = schema::read_effective_boolean(ElementRef::Object(element), "isStatic")?;
        let is_abstract = if binding.role == SubjectLabelRole::Operation {
            Some(schema::read_effective_boolean(
                ElementRef::Object(element),
                "isAbstract",
            )?)
        } else {
            None
        };
        Ok((label, is_static, is_abstract))
    })();
    match result {
        Ok((text, is_static, is_abstract)) => {
            rendered.text = Some(text);
            style.font_underline = Some(is_static);
            if let Some(is_abstract) = is_abstract {
                style.font_weight = Some(400);
                style.font_italic = Some(is_abstract);
            }
            style.text_align.get_or_insert(TextAlignment::Left);
        }
        Err(error) => rendered
            .diagnostics
            .push(format!("View {}: {error}", view.id)),
    }
    rendered
}

fn member_label(
    section: &ModelingSection,
    element: &Element,
    role: SubjectLabelRole,
    references: &ReferenceIndex<'_>,
    options: &UmlStyle,
) -> Result<String, String> {
    let mut label = String::new();
    if options.should_show_stereotype() {
        let stereotype = crate::stereotype_presentation(section, element);
        if matches!(
            stereotype,
            crate::StereotypePresentation::UnverifiedReference { label: None, .. }
                | crate::StereotypePresentation::Unsupported(_)
        ) {
            return Err("stereotype label is unresolved".into());
        }
        if let Some(stereotype) = stereotype.label().filter(|label| !label.is_empty()) {
            label.push_str(&format!("«{stereotype}»"));
        }
    }
    if options.should_show_visibility() {
        label.push_str(match element.properties.get("visibility") {
            None | Some(Value::Null) => "",
            Some(Value::String(value)) if value == "public" => "+",
            Some(Value::String(value)) if value == "private" => "-",
            Some(Value::String(value)) if value == "protected" => "#",
            Some(Value::String(value)) if value == "package" => "~",
            _ => return Err("visibility notation is unresolved".into()),
        });
    }
    if role == SubjectLabelRole::Attribute && explicit_true(element, "isDerived") {
        label.push('/');
    }
    label.push_str(&element.name);
    let mut properties = Vec::new();
    match role {
        SubjectLabelRole::Attribute => {
            append_type_and_multiplicity(&mut label, element, references, options, true)?;
            append_default(&mut label, element);
            for (property, text) in [
                ("isID", "id"),
                ("isReadOnly", "readOnly"),
                ("isOrdered", "ordered"),
                ("isUnique", "unique"),
            ] {
                if explicit_true(element, property) {
                    properties.push(text);
                }
            }
        }
        SubjectLabelRole::Operation => {
            label.push('(');
            if options.show_operation_signature != Some(false) {
                let mut parameters = Vec::new();
                for parameter in operations::owned_parameters(references, &element.id)? {
                    let direction = operations::direction(parameter)?;
                    if direction == "return" {
                        continue;
                    }
                    let mut parameter_label = match direction.as_str() {
                        "in" => String::new(),
                        "out" | "inout" => format!("{direction} "),
                        _ => {
                            return Err(format!(
                                "Parameter {} has unresolved direction",
                                parameter.id
                            ));
                        }
                    };
                    parameter_label.push_str(&parameter.name);
                    append_type_and_multiplicity(
                        &mut parameter_label,
                        parameter,
                        references,
                        options,
                        true,
                    )?;
                    append_default(&mut parameter_label, parameter);
                    parameters.push(parameter_label);
                }
                label.push_str(&parameters.join(", "));
            }
            label.push(')');
            let result = if options.show_operation_signature != Some(false)
                || options.should_show_property()
            {
                operations::return_parameter(references, &element.id)?
            } else {
                None
            };
            if options.show_operation_signature != Some(false)
                && let Some(result) = result
            {
                let mut result_label = String::new();
                append_type_and_multiplicity(
                    &mut result_label,
                    result,
                    references,
                    options,
                    false,
                )?;
                if !result_label.is_empty() {
                    label.push_str(": ");
                    label.push_str(&result_label);
                }
            }
            if explicit_true(element, "isQuery") {
                properties.push("query");
            }
            if let Some(result) = result {
                if explicit_true(result, "isOrdered") {
                    properties.push("ordered");
                }
                if explicit_true(result, "isUnique") {
                    properties.push("unique");
                }
            }
        }
    }
    if options.should_show_property() && !properties.is_empty() {
        label.push_str(" {");
        label.push_str(&properties.join(", "));
        label.push('}');
    }
    Ok(label)
}

fn append_type_and_multiplicity(
    label: &mut String,
    element: &Element,
    references: &ReferenceIndex<'_>,
    options: &UmlStyle,
    type_separator: bool,
) -> Result<(), String> {
    if options.should_show_type() {
        let value_type = match element.properties.get("type") {
            None | Some(Value::Null) => None,
            Some(Value::String(text)) => Some(text.clone()).filter(|text| !text.is_empty()),
            Some(value) => {
                let id = crate::uml::references::reference_id(value)?
                    .ok_or("type identity is unresolved")?;
                let target = references.element(id)?;
                Some(
                    target
                        .named()
                        .and_then(|target| target.name())
                        .ok_or("type name is unresolved")?
                        .to_owned(),
                )
            }
        };
        if let Some(value_type) = value_type {
            if type_separator {
                label.push_str(": ");
            }
            // A textual declaration remains text, not a resolved UML Type.
            label.push_str(&value_type);
        }
    }
    if options.show_multiplicity != Some(false) {
        let notation = if value_specifications::has_bound_references(ElementRef::Object(element)) {
            let bounds =
                value_specifications::multiplicity(references, ElementRef::Object(element))
                    .map_err(|error| error.to_string())?;
            let lower = bounds.lower_bound().to_string();
            let upper = bounds.upper_bound().to_string();
            Some(if lower == upper {
                lower
            } else {
                format!("{lower}..{upper}")
            })
        } else {
            element
                .properties
                .get("multiplicity")
                .and_then(Value::as_str)
                .filter(|text| !text.is_empty())
                .map(str::to_owned)
        };
        if let Some(notation) = notation {
            label.push('[');
            label.push_str(&notation);
            label.push(']');
        }
    }
    Ok(())
}

fn explicit_true(element: &Element, property: &str) -> bool {
    element.properties.get(property).and_then(Value::as_bool) == Some(true)
}

fn append_default(label: &mut String, element: &Element) {
    if let Some(default) = element
        .properties
        .get("defaultValue")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
    {
        label.push_str(" = ");
        label.push_str(default);
    }
}
