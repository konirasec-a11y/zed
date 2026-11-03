use crate::{Element, ModelingRelationKind, UmlType};
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

fn text(value: Option<&Value>) -> Option<&str> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub fn element_display_label(element: &Element) -> String {
    let member = matches!(
        UmlType::from_kind(&element.kind),
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
    let prefix = match text(element.properties.get("visibility")) {
        Some("public") => "+",
        Some("private") => "-",
        Some("protected") => "#",
        Some("package") => "~",
        _ => "",
    };
    let mut label = format!("{prefix}{}", element.name);
    // Source declaration/signature fields may retain an earlier name. They belong
    // in their labeled inspector fields, never in the canonical semantic label.
    if let Some(value_type) = text(element.properties.get("type")) {
        label.push_str(": ");
        label.push_str(value_type);
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
        let project = serde_json::from_str(include_str!("../fixtures/todo.json")).expect("fixture");
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
        assert_eq!(row.label, "Customer wording");
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
            "-anything: String"
        );
        assert_eq!(
            element_display_label(&element(
                "uml.operation",
                json!({"signature":"save(value: String): bool", "visibility":"public"})
            )),
            "+anything"
        );
        assert_eq!(
            element_display_label(&element("uml.operation", json!({}))),
            "anything"
        );
        assert_eq!(
            element_display_label(&element("uml.class", json!({"type":"not-a-member"}))),
            "anything"
        );
    }
}
