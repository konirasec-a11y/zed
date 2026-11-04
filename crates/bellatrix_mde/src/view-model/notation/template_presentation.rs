use crate::{Element, ModelingSection, template_parametered_element};

pub fn template_parameter_label(section: &ModelingSection, parameter: &Element) -> String {
    let references =
        crate::uml::references::ReferenceIndex::new(crate::uml::root::Model::from_section(section));
    template_parameter_label_with_references(section, parameter, &references)
}

pub fn template_parameter_label_with_references(
    section: &ModelingSection,
    parameter: &Element,
    references: &crate::uml::references::ReferenceIndex<'_>,
) -> String {
    crate::element_display_label_with_references(
        template_parametered_element(section, parameter).unwrap_or(parameter),
        references,
    )
}
