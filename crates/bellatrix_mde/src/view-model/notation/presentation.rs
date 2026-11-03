use crate::{Element, ModelingRelationKind, Style, UmlType};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticIdentity<'a> {
    pub type_label: &'a str,
    pub metaclass: Option<&'static str>,
}

pub fn semantic_identity(kind: &str) -> SemanticIdentity<'_> {
    let metaclass = UmlType::from_kind(kind)
        .and_then(UmlType::metaclass)
        .or_else(|| {
            let canonical = kind.strip_prefix("uml.")?;
            let mut candidate = String::new();
            for word in canonical.split('_') {
                if word.is_empty() || !word.chars().all(|character| character.is_ascii_lowercase())
                {
                    return None;
                }
                let mut characters = word.chars();
                candidate.push(characters.next()?.to_ascii_uppercase());
                candidate.extend(characters);
            }
            crate::uml::metamodel::DIRECT_GENERALIZATIONS
                .iter()
                .find(|(name, _)| *name == candidate)
                .map(|(name, _)| *name)
        });
    // Attribute and Part are recognizable presentation roles of Property, not metaclasses.
    let type_label = match kind {
        "uml.attribute" | "rust.field" => "Attribute",
        "uml.part" => "Part",
        "rust.module" => "Module",
        "rust.enum" => "Enumeration",
        "uml.data_type" => "Data type",
        "uml.primitive_type" => "Primitive type",
        "uml.association_class" => "Association class",
        "uml.execution_environment" => "Execution environment",
        "uml.template_parameter" => "Template parameter",
        "uml.nary_association" => "N-ary association",
        "uml.template" => "Template",
        "uml.subsystem" => "Subsystem",
        "uml.frame" => "Frame",
        _ => ModelingRelationKind::BASIC
            .iter()
            .chain(ModelingRelationKind::ADVANCED)
            .chain(ModelingRelationKind::PACKAGES)
            .find(|relation| relation.model_kind() == kind)
            .map(|relation| relation.label())
            .or(metaclass)
            .unwrap_or(kind),
    };
    SemanticIdentity {
        type_label,
        metaclass,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClassifierPresentationOptions {
    /// Optional active-Class notation (UML 2.5.1 §11.4.4), not semantic state.
    pub show_active_class_sides: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClassifierPresentation {
    pub name_is_italic: bool,
    pub double_sided: bool,
}

pub fn classifier_presentation(
    element: &Element,
    options: ClassifierPresentationOptions,
) -> ClassifierPresentation {
    let Some(kind) = UmlType::from_kind(&element.kind).filter(|kind| kind.is_classifier()) else {
        return ClassifierPresentation::default();
    };
    let property_is_true = |property: &str| {
        kind.has_property(property)
            && element.properties.get(property).and_then(Value::as_bool) == Some(true)
    };
    // §9.2.4 permits italic names instead of an additional {abstract} annotation.
    // UMLDI B.7.7 permits double sides only for an active Class (including subclasses).
    ClassifierPresentation {
        name_is_italic: property_is_true("isAbstract"),
        double_sided: options.show_active_class_sides
            && kind
                .metaclass()
                .is_some_and(|name| crate::uml::metamodel::conforms_to(name, "Class"))
            && property_is_true("isActive"),
    }
}

fn text(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemberPresentation {
    pub underlined: bool,
}

pub fn member_presentation(element: &Element) -> MemberPresentation {
    MemberPresentation {
        // UML 2.5.1 §9.4.4 applies to inherited Feature::isStatic, not classifiers.
        underlined: UmlType::from_kind(&element.kind)
            .is_some_and(|kind| kind.has_property("isStatic"))
            && element.properties.get("isStatic").and_then(Value::as_bool) == Some(true),
    }
}

pub fn element_display_label(element: &Element) -> String {
    display_label(element, None, &Style::default())
}

pub fn element_display_label_with_references(
    element: &Element,
    references: &crate::uml::references::ReferenceIndex<'_>,
) -> String {
    display_label(element, Some(references), &Style::default())
}

pub fn format_element_label_for_view(
    element: &Element,
    references: &crate::uml::references::ReferenceIndex<'_>,
    style: &Style,
) -> String {
    display_label(element, Some(references), style)
}

fn display_label(
    element: &Element,
    references: Option<&crate::uml::references::ReferenceIndex<'_>>,
    style: &Style,
) -> String {
    if let Some(body) = crate::uml::root::ElementRef::Object(element)
        .comment()
        .and_then(|comment| comment.body())
    {
        return body.to_owned();
    }
    if element.kind == "uml.instance_specification" && let Some(references) = references {
        return crate::format_instance_heading(element, references, style);
    }
    if element.kind == "uml.collaboration_use" {
        return if style.should_show_type() {
            type_label(element, references).map_or_else(|| element.name.clone(), |value_type| format!("{} : {value_type}", element.name))
        } else { element.name.clone() };
    }
    let kind = UmlType::from_kind(&element.kind);
    if kind == Some(UmlType::Parameter) {
        let mut label = match crate::uml::operations::direction(element) {
            Ok(direction) if direction == "in" => element.name.clone(),
            Ok(direction) => format!("{direction} {}", element.name),
            Err(_) => format!("[invalid direction] {}", element.name),
        };
        if style.should_show_type()
            && let Some(value_type) = type_label(element, references)
        {
            label.push_str(": ");
            label.push_str(&value_type);
        }
        return label;
    }
    if kind == Some(UmlType::Operation) {
        return operation_label(element, references, style);
    }
    let member = matches!(
        kind,
        Some(
            UmlType::Attribute
                | UmlType::Operation
                | UmlType::Reception
                | UmlType::Port
                | UmlType::Part
        )
    );
    if !member {
        return element.name.clone();
    }
    let prefix = match text(element.properties.get("visibility"))
        .filter(|_| style.should_show_visibility())
    {
        Some("public") => "+",
        Some("private") => "-",
        Some("protected") => "#",
        Some("package") => "~",
        _ => "",
    };
    let property = matches!(
        kind,
        Some(UmlType::Attribute | UmlType::Port | UmlType::Part)
    );
    let is_true = |key| element.properties.get(key).and_then(Value::as_bool) == Some(true);
    let derived = if property && is_true("isDerived") {
        "/"
    } else {
        ""
    };
    let mut label = format!("{prefix}{derived}{}", element.name);
    // Source declaration/signature fields may retain an earlier name. They belong
    // in their labeled inspector fields, never in the canonical semantic label.
    let type_label = type_label(element, references);
    let value_type = type_label.as_deref();
    if style.should_show_type()
        && let Some(value_type) = value_type
    {
        label.push_str(": ");
        label.push_str(value_type);
    }
    if property {
        let bound = |key| match element.properties.get(key) {
            Some(Value::Number(value)) if value.is_u64() => Some(value.to_string()),
            value => text(value).map(str::to_owned),
        };
        if let (Some(lower), Some(upper)) = (bound("lower"), bound("upper")) {
            label.push_str(" [");
            label.push_str(&lower);
            if lower != upper {
                label.push_str("..");
                label.push_str(&upper);
            }
            label.push(']');
        }
    }
    if property {
        if let Some(default) = text(element.properties.get("defaultValue")) {
            label.push_str(" = ");
            label.push_str(default);
        }
        if style.should_show_property() && is_true("isReadOnly") {
            label.push_str(" {readOnly}");
        }
    }
    label
}

fn type_label(
    element: &Element,
    references: Option<&crate::uml::references::ReferenceIndex<'_>>,
) -> Option<String> {
    let type_value = element.properties.get("type");
    match type_value {
        Some(value) if value.is_object() => {
            Some(match crate::uml::references::reference_id(value) {
                Ok(Some(id)) => {
                    let target = references.and_then(|index| {
                        let descriptor = crate::uml::root::ElementRef::Object(element)
                            .metaclass()
                            .and_then(|metaclass| {
                                crate::uml::schema::uml_property(metaclass, "type")
                                    .ok()
                                    .flatten()
                            })?;
                        index.validate(descriptor, value).ok().flatten()
                    });
                    match target {
                        Some(target) => target
                            .named()
                            .and_then(|target| target.name())
                            .map(str::to_owned)
                            .unwrap_or_else(|| format!("[unnamed Type: {id}]")),
                        None => format!("[unresolved Type: {id}]"),
                    }
                }
                _ => "[invalid Type reference]".into(),
            })
        }
        value => text(value).map(|text| format!("{text} {{type text}}")),
    }
}

fn operation_label(
    operation: &Element,
    references: Option<&crate::uml::references::ReferenceIndex<'_>>,
    style: &Style,
) -> String {
    let prefix = match text(operation.properties.get("visibility"))
        .filter(|_| style.should_show_visibility())
    {
        Some("public") => "+",
        Some("private") => "-",
        Some("protected") => "#",
        Some("package") => "~",
        _ => "",
    };
    let mut label = format!("{prefix}{}(", operation.name);
    let mut result_type = None;
    if let Some(references) = references {
        match crate::uml::operations::owned_parameters(references, &operation.id) {
            Ok(parameters) => {
                let parameters = parameters
                    .into_iter()
                    .filter(|parameter| {
                        crate::uml::operations::direction(parameter).as_deref() != Ok("return")
                    })
                    .map(|parameter| display_label(parameter, Some(references), style))
                    .collect::<Vec<_>>();
                label.push_str(&parameters.join(", "));
                result_type =
                    match crate::uml::operations::return_parameter(references, &operation.id) {
                        Ok(parameter) => {
                            parameter.and_then(|parameter| type_label(parameter, Some(references)))
                        }
                        Err(_) => Some("[invalid return Parameters]".into()),
                    };
            }
            Err(_) => label.push_str("[invalid Parameters]"),
        }
    }
    label.push(')');
    if style.should_show_type()
        && let Some(result_type) = &result_type
    {
        label.push_str(": ");
        label.push_str(result_type);
    }
    if style.should_show_property()
        && operation.properties.get("isQuery").and_then(Value::as_bool) == Some(true)
    {
        if style.should_show_type() && result_type.is_none() {
            label.push(':');
        }
        label.push_str(" {query}");
    }
    label
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn element(kind: &str, properties: Value) -> Element {
        serde_json::from_value(
            json!({"id":"member", "kind":kind,"name":"anything", "properties":properties}),
        )
        .expect("fixture")
    }

    #[test]
    fn inspector_changes_immediately_drive_classifier_presentation() {
        let project = serde_json::from_str(include_str!("../../../fixtures/todo.json")).expect("fixture");
        let mut section = crate::ModelingSection::from_model(&project);
        let options = ClassifierPresentationOptions {
            show_active_class_sides: true,
        };
        for kind in ["uml.class", "uml.component"] {
            section
                .elements
                .iter_mut()
                .find(|element| element.id == "type-list")
                .expect("class")
                .kind = kind.into();
            for active in [true, false, true] {
                for property in ["isAbstract", "isActive"] {
                    crate::set_inspected_property(
                        &mut section,
                        "type-list",
                        &format!("properties.{property}"),
                        json!(active),
                    )
                    .expect("typed inherited property");
                }
                let classifier = section
                    .elements
                    .iter()
                    .find(|element| element.id == "type-list")
                    .expect("class");
                assert_eq!(
                    classifier_presentation(classifier, options),
                    ClassifierPresentation {
                        name_is_italic: active,
                        double_sided: active
                    }
                );
            }
        }
    }

    #[test]
    fn classifier_notation_reads_strict_semantic_booleans() {
        let options = ClassifierPresentationOptions {
            show_active_class_sides: true,
        };
        for kind in [
            "uml.class",
            "rust.struct",
            "uml.component",
            "uml.association_class",
            "uml.node",
            "uml.stereotype",
        ] {
            let mut classifier = element(kind, json!({"isAbstract":true, "isActive":true}));
            let presentation = classifier_presentation(&classifier, options);
            assert_eq!(
                presentation,
                ClassifierPresentation {
                    name_is_italic: true,
                    double_sided: true
                },
                "{kind}"
            );
            classifier.name = "Renamed without changing its semantics".into();
            assert_eq!(classifier_presentation(&classifier, options), presentation);
            assert!(
                !classifier_presentation(&classifier, ClassifierPresentationOptions::default())
                    .double_sided
            );
            for value in [
                json!(false),
                json!("true"),
                json!("yes"),
                json!(1),
                json!(null),
                json!({}),
                json!([]),
            ] {
                classifier
                    .properties
                    .insert("isAbstract".into(), value.clone());
                classifier.properties.insert("isActive".into(), value);
                assert_eq!(
                    classifier_presentation(&classifier, options),
                    ClassifierPresentation::default(),
                    "{kind}"
                );
            }
            classifier.properties.clear();
            assert_eq!(
                classifier_presentation(&classifier, options),
                ClassifierPresentation::default()
            );
        }
    }

    #[test]
    fn classifier_presentation_never_grants_class_features_to_other_types() {
        let options = ClassifierPresentationOptions {
            show_active_class_sides: true,
        };
        for kind in [
            "uml.interface",
            "uml.signal",
            "uml.data_type",
            "uml.enumeration",
            "uml.artifact",
            "uml.collaboration",
        ] {
            let presentation = classifier_presentation(
                &element(kind, json!({"isAbstract":true,"isActive":true})),
                options,
            );
            assert!(presentation.name_is_italic, "{kind}");
            assert!(!presentation.double_sided, "{kind}");
        }
        for kind in [
            "uml.operation",
            "uml.attribute",
            "uml.package",
            "uml.template",
            "uml.subsystem",
            "uml.frame",
            "uml.unknown",
            "Class",
            "custom.widget",
            "",
        ] {
            assert_eq!(
                classifier_presentation(
                    &element(kind, json!({"isAbstract":true,"isActive":true})),
                    options
                ),
                ClassifierPresentation::default(),
                "{kind}"
            );
        }
    }

    #[test]
    fn renamed_members_keep_their_semantic_identity() {
        for (kind, label, metaclass) in [
            ("uml.attribute", "Attribute", "Property"),
            ("uml.part", "Part", "Property"),
            ("uml.port", "Port", "Port"),
            ("uml.constraint", "Constraint", "Constraint"),
            ("uml.operation", "Operation", "Operation"),
            ("uml.reception", "Reception", "Reception"),
            (
                "uml.template_parameter",
                "Template parameter",
                "TemplateParameter",
            ),
        ] {
            let mut member = element(kind, json!({}));
            let before = semantic_identity(&member.kind);
            member.name = "New arbitrary user name".into();
            assert_eq!(semantic_identity(&member.kind), before);
            assert_eq!(before.type_label, label);
            assert_eq!(before.metaclass, Some(metaclass));
            assert!(element_display_label(&member).contains(&member.name));
        }
    }

    #[test]
    fn stale_source_names_never_compete_with_the_canonical_name() {
        for (kind, properties) in [
            (
                "rust.field",
                json!({"declaration":"pub old_name: String", "type":"String", "visibility":"private"}),
            ),
            (
                "rust.function",
                json!({"signature":"pub fn old_name(value: String) -> bool", "visibility":"public"}),
            ),
        ] {
            let mut member = element(kind, properties.clone());
            member.name = "current_name".into();
            let label = element_display_label(&member);
            assert!(label.contains("current_name"));
            assert!(!label.contains("old_name"));
            assert!(!label.contains("pub "));
            assert_eq!(
                serde_json::to_value(&member.properties).expect("properties"),
                properties
            );
        }
    }

    #[test]
    fn hierarchy_and_inspector_share_identity_after_rename() {
        let project = serde_json::from_str(include_str!("../../../fixtures/todo.json")).expect("fixture");
        let mut section = crate::ModelingSection::from_model(&project);
        let member = section
            .elements
            .iter_mut()
            .find(|element| element.id == "field-title")
            .expect("attribute");
        member.name = "Customer wording".into();
        member.properties.insert("type".into(), json!("String"));
        member.properties.remove("declaration");
        let expected = element_display_label(member);
        let inspector = crate::inspect_element(&section, "field-title").expect("inspector");
        let owner = crate::inspect_element(&section, "type-task").expect("owner");
        assert_eq!(inspector.name, "Customer wording");
        assert_eq!(inspector.display_label, expected);
        assert_eq!(
            owner
                .attributes
                .iter()
                .find(|member| member.id == "field-title")
                .expect("owned attribute")
                .display_label,
            expected
        );
        let tree = crate::project_hierarchy(&section);
        let row = tree
            .iter()
            .find(|row| row.element_id == "field-title")
            .expect("row");
        assert_eq!(row.label, expected);
        assert_eq!(row.depth, 1);
        assert_eq!(semantic_identity(&row.kind).type_label, "Attribute");
        assert_eq!(
            semantic_identity(&inspector.kind).metaclass,
            Some("Property")
        );
    }

    #[test]
    fn canonical_relationships_resolve_official_metaclasses() {
        for (kind, label, metaclass) in [
            ("uml.usage", "Usage", "Usage"),
            ("uml.dependency", "Dependency", "Dependency"),
            ("uml.association", "Association", "Association"),
            (
                "uml.interface_realization",
                "Interface Realization",
                "InterfaceRealization",
            ),
        ] {
            let identity = semantic_identity(kind);
            assert_eq!(identity.type_label, label);
            assert_eq!(identity.metaclass, Some(metaclass));
        }
        assert_eq!(semantic_identity("uml.unknown_kind").metaclass, None);
        assert_eq!(semantic_identity("uml.frame").metaclass, None);
        assert_eq!(semantic_identity("uml.subsystem").metaclass, None);
    }

    #[test]
    fn unknown_kinds_do_not_claim_to_be_classes() {
        let identity = semantic_identity("custom.widget");
        assert_eq!(identity.type_label, "custom.widget");
        assert_eq!(identity.metaclass, None);
    }

    #[test]
    fn signatures_show_stored_values_without_inventing_types_or_visibility() {
        assert_eq!(
            element_display_label(&element("uml.attribute", json!({}))),
            "anything"
        );
        assert_eq!(
            element_display_label(&element(
                "uml.attribute",
                json!({"type":"String", "visibility":"private"})
            )),
            "-anything: String {type text}"
        );
        assert_eq!(
            element_display_label(&element(
                "uml.operation",
                json!({"signature":"save(value: String): bool", "visibility":"public"})
            )),
            "+anything()"
        );
        assert_eq!(
            element_display_label(&element("uml.operation", json!({}))),
            "anything()"
        );
        assert_eq!(
            element_display_label(&element("uml.class", json!({"type":"not-a-member"}))),
            "anything"
        );
    }
}
