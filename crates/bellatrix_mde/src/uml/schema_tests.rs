use super::*;
use crate::{
    Format, ModelingSection, Project, PropertyEditor,
    commands::{self, Command},
    inspect_element,
};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn contract(
    metaclass: &str,
    name: &str,
) -> Result<&'static PropertyDescriptor, Box<dyn std::error::Error>> {
    uml_property(metaclass, name)?.ok_or_else(|| format!("missing {metaclass}::{name}").into())
}

#[test]
fn pinned_sources_resolve_typed_inheritance_and_expose_normative_defects() -> TestResult {
    assert_eq!(metamodel::SOURCES.len(), 8);
    for (document, kind, count) in [
        (Document::Uml, ClassifierKind::Class, 242),
        (Document::UmlDi, ClassifierKind::Class, 33),
        (Document::PrimitiveTypes, ClassifierKind::Primitive, 5),
        (Document::StandardProfile, ClassifierKind::Stereotype, 33),
    ] {
        assert_eq!(
            metamodel::CLASSIFIERS
                .iter()
                .filter(|classifier| classifier.id.document == document && classifier.kind == kind)
                .count(),
            count
        );
    }
    for descriptor in metamodel::CLASSIFIERS {
        assert_eq!(
            classifier(descriptor.id).map(|found| found.id),
            Some(descriptor.id)
        );
    }
    let class = XmiId::new(Document::Uml, "Class");
    let unknown = XmiId::new(Document::Uml, "MissingClassifier");
    assert_eq!(
        conforms_to(unknown, class),
        Err(LookupError::UnknownClassifier)
    );
    assert_eq!(
        conforms_to(class, unknown),
        Err(LookupError::UnknownClassifier)
    );
    assert!(matches!(
        property(unknown, "visibility"),
        Err(LookupError::UnknownClassifier)
    ));
    let mut inherited = inherited_properties(class)?;
    let inherited_ids = inherited
        .properties
        .iter()
        .map(|property| property.id)
        .collect::<Vec<_>>();
    inherited.properties.clear();
    assert_eq!(
        inherited_properties(class)?
            .properties
            .iter()
            .map(|property| property.id)
            .collect::<Vec<_>>(),
        inherited_ids,
    );
    let visibility = contract("Class", "visibility")?;
    assert_eq!(
        visibility.id,
        XmiId::new(Document::Uml, "PackageableElement-visibility")
    );
    assert_eq!((visibility.lower, visibility.upper), ("0", "1"));
    assert_eq!(
        effective_default(visibility)?.and_then(DefaultValue::scalar_json),
        Some(json!("public"))
    );
    assert_eq!(contract("NamedElement", "visibility")?.default, None);
    assert_eq!(validate_scalar(visibility, &Value::Null)?, Ok(()));
    let abstract_flag = contract("Component", "isAbstract")?;
    assert_eq!(
        abstract_flag.property_type.target,
        Some(XmiId::new(Document::PrimitiveTypes20131001, "Boolean"))
    );
    assert_ne!(
        abstract_flag.property_type.target,
        Some(XmiId::new(Document::PrimitiveTypes, "Boolean"))
    );
    assert_eq!(
        validate_scalar(abstract_flag, &Value::Null)?,
        Err(ValueError::Required)
    );
    assert_eq!(
        effective_default(contract("Parameter", "direction")?)?.and_then(DefaultValue::scalar_json),
        Some(json!("in"))
    );
    let attribute = contract("Class", "ownedAttribute")?;
    assert_eq!(attribute.aggregation, Aggregation::Composite);
    assert!(attribute.subsets.iter().any(
        |reference| reference.target == Some(XmiId::new(Document::Uml, "Classifier-attribute"))
    ));
    assert_eq!(
        attribute
            .opposites
            .first()
            .and_then(|reference| reference.target),
        Some(XmiId::new(Document::Uml, "Property-class"))
    );
    assert!(matches!(
        scalar_kind(attribute),
        Err(UnsupportedContract::Collection)
    ));
    let diagram = XmiId::new(Document::UmlDi, "UMLClassDiagram");
    assert!(conforms_to(
        diagram,
        XmiId::new(Document::Di, "DiagramElement")
    )?);
    assert!(!conforms_to(
        XmiId::new(Document::Uml, "Class"),
        XmiId::new(Document::Uml20131001, "Class")
    )?);
    let icon = property(diagram, "isIcon")?.ok_or("missing inherited icon contract")?;
    assert_eq!(
        effective_default(icon)?.and_then(DefaultValue::scalar_json),
        Some(json!(false))
    );
    let bounds =
        property(XmiId::new(Document::UmlDi, "UMLShape"), "bounds")?.ok_or("missing DI bounds")?;
    assert_eq!(bounds.id.document, Document::Di);
    assert_eq!(
        bounds.property_type.target,
        Some(XmiId::new(Document::Dc, "Bounds"))
    );
    let base = property(
        XmiId::new(Document::StandardProfile, "Auxiliary"),
        "base_Class",
    )?
    .ok_or("missing StandardProfile base")?;
    assert_eq!(
        base.property_type.target,
        Some(XmiId::new(Document::Uml, "Class"))
    );
    let opposite = base
        .opposites
        .first()
        .and_then(|reference| reference.target)
        .and_then(property_by_id)
        .ok_or("missing extension end")?;
    assert_eq!((opposite.lower, opposite.upper), ("0", "1"));
    assert_eq!(opposite.aggregation, Aggregation::Composite);
    assert!(
        metamodel::UNRESOLVED_REFERENCES
            .iter()
            .all(|reference| matches!(
                reference.reference.resolution,
                ReferenceResolution::MissingIdentifier | ReferenceResolution::AmbiguousIdentifier
            ))
    );
    assert!(
        metamodel::UNRESOLVED_REFERENCES
            .iter()
            .any(|reference| reference.owner.fragment == "UML_mofTag0"
                && reference.reference.target.is_none())
    );
    assert!(
        metamodel::AMBIGUOUS_DEFINITIONS
            .iter()
            .any(|definition| definition.id == XmiId::new(Document::Uml, "State-isConsistentWith"))
    );
    assert!(
        metamodel::UNEVALUATED_CONSTRAINTS
            .iter()
            .any(|constraint| constraint.id
                == XmiId::new(Document::Uml, "Property-subsetting_rules"))
    );
    let operation = operation_by_id(XmiId::new(
        Document::Uml,
        "MultiplicityElement-includesMultiplicity",
    ))
    .ok_or("missing operation")?;
    let parameters = operation
        .parameters
        .iter()
        .map(|reference| {
            reference
                .target
                .and_then(parameter_by_id)
                .ok_or("unresolved parameter")
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        parameters
            .iter()
            .map(|parameter| parameter.direction)
            .collect::<Vec<_>>(),
        [ParameterDirection::Return, ParameterDirection::In]
    );
    assert_eq!(
        parameters
            .get(1)
            .and_then(|parameter| parameter.parameter_type)
            .and_then(|reference| reference.target),
        Some(XmiId::new(Document::Uml, "MultiplicityElement"))
    );
    let condition = operation
        .body_conditions
        .first()
        .and_then(|reference| reference.target)
        .ok_or("missing body condition")?;
    let specification = metamodel::UNEVALUATED_CONSTRAINTS
        .iter()
        .find(|constraint| constraint.id == condition)
        .and_then(|constraint| constraint.specification)
        .and_then(specification_by_id)
        .ok_or("missing source specification")?;
    assert_eq!(specification.languages, &["OCL"]);
    assert!(
        specification
            .bodies
            .iter()
            .any(|body| body.contains("self.lowerBound() <= M.lowerBound()"))
    );
    assert!(documentation_for(operation.id).any(|comment| {
        comment
            .body
            .is_some_and(|body| body.contains("includes all the cardinalities"))
    }));
    assert!(operation_by_id(XmiId::new(Document::Uml, "State-isConsistentWith")).is_none());
    let inventory: Value = serde_json::from_str(include_str!("metamodel.inventory.json"))?;
    assert_eq!(inventory["sources"].as_array().map(Vec::len), Some(8));
    assert!(
        inventory["not_executable_from_extraction"]
            .as_array()
            .is_some_and(|items| items.iter().any(|value| value
                .as_str()
                .is_some_and(|text| text.contains("not interpreted"))))
    );
    for descriptor in metamodel::PROPERTIES {
        assert_eq!(
            property_by_id(descriptor.id).map(|found| found.id),
            Some(descriptor.id)
        );
        assert!(
            descriptor
                .property_type
                .target
                .and_then(classifier)
                .is_some(),
            "unresolved Type: {:?}",
            descriptor.id
        );
    }
    Ok(())
}

fn fixture() -> serde_json::Result<Project> {
    serde_json::from_value(
        json!({"id": "project", "name": "Contracts", "root_model": "root",
        "elements": [
            {"id":"root", "kind":"uml.model", "name":"Root"},
            {"id":"class", "kind":"uml.class", "name":"Box"},
            {"id":"attribute", "kind":"uml.attribute", "name":"item", "properties":{"type":"String", "defaultValue":"empty", "aggregation":"none"}},
            {"id":"operation", "kind":"uml.operation", "name":"get", "properties":{"isOrdered":false,"type":"LegacyResult"}}
        ],
        "relations":[
            {"id":"owns-class", "kind":"owns", "from":"root", "to":"class"},
            {"id":"owns-attribute", "kind":"owns", "from":"class", "to":"attribute"},
            {"id":"owns-operation", "kind":"owns", "from":"class", "to":"operation"}
        ]}),
    )
}

fn assert_legacy_contract_warnings(
    diagnostics: Vec<crate::StructuralDiagnostic>,
    type_warning: &str,
) {
    assert!(
        diagnostics
            .iter()
            .all(|issue| issue.severity == crate::StructuralSeverity::Warning),
        "{diagnostics:?}"
    );
    let mut actual = diagnostics
        .iter()
        .map(|issue| {
            (
                issue.subject_id.as_str(),
                issue.code,
                issue.message.as_str(),
            )
        })
        .collect::<Vec<_>>();
    let mut expected = vec![
        (
            "attribute",
            "MDE_PROPERTY_CONTRACT_UNVERIFIED",
            "Property-defaultValue: text declaration is not a resolved UML reference",
        ),
        (
            "attribute",
            "MDE_PROPERTY_CONTRACT_UNVERIFIED",
            type_warning,
        ),
        (
            "operation",
            "MDE_PROPERTY_CONTRACT_UNVERIFIED",
            "Operation-isOrdered: a stored value does not establish its UML derivation",
        ),
        (
            "operation",
            "MDE_PROPERTY_CONTRACT_UNVERIFIED",
            "Operation-type: a stored value does not establish its UML derivation",
        ),
    ];
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}

fn edit(project: &Project, id: &str, key: &str, value: Value) -> Result<Project, String> {
    commands::apply(
        project,
        &Command::SetProperty {
            element_id: id.into(),
            path: format!("properties.{key}"),
            value,
        },
        0,
    )
}

#[test]
fn generated_contracts_drive_canonical_inspection_commands_validation_and_roundtrip() -> TestResult
{
    let original = fixture()?;
    let section = ModelingSection::from_model(&original);
    let class = inspect_element(&section, "class").ok_or("missing Class inspector")?;
    assert!(
        class
            .properties
            .iter()
            .any(|field| field.path == "properties.visibility" && field.value == json!("public"))
    );
    let attribute = inspect_element(&section, "attribute").ok_or("missing Property inspector")?;
    assert!(
        attribute
            .properties
            .iter()
            .any(|field| field.path == "properties.visibility" && field.value.is_null())
    );
    assert!(
        attribute
            .properties
            .iter()
            .any(|field| field.path == "properties.aggregation"
                && field.editor == PropertyEditor::Choice(&["none", "shared", "composite"]))
    );
    assert!(
        !original
            .element("class")
            .ok_or("missing class")?
            .properties
            .contains_key("visibility")
    );
    let changed = edit(&original, "class", "isAbstract", json!(true))?;
    assert!(edit(&changed, "class", "isAbstract", json!("true")).is_err());
    assert!(edit(&changed, "attribute", "aggregation", json!("other")).is_err());
    assert!(edit(&changed, "operation", "isOrdered", json!(true)).is_err());
    assert!(contract("Operation", "isOrdered")?.is_read_only);
    assert!(contract("Property", "isComposite")?.is_derived);
    assert!(!contract("Property", "isComposite")?.is_read_only);
    assert!(validate_edit("Property", "isComposite", &json!(true)).is_err());
    let changed = commands::apply(
        &changed,
        &Command::CreateDiagram {
            kind: crate::StructuralDiagramKind::Class,
            name: "Types".into(),
            model_elements: vec![],
        },
        1,
    )?;
    let diagram_id = changed.diagrams.last().ok_or("missing diagram")?.id.clone();
    let changed = commands::apply(
        &changed,
        &Command::CreateElement {
            diagram_id,
            owner_id: Some("root".into()),
            kind: crate::commands::ModelingElementKind::PrimitiveType,
            name: Some("String".into()),
            x: 0.0,
            y: 0.0,
        },
        2,
    )?;
    let type_id = changed.elements.last().ok_or("missing Type")?.id.clone();
    let changed = commands::apply(
        &changed,
        &Command::AddMember {
            owner_id: "class".into(),
            kind: crate::commands::ModelingMemberKind::Attribute,
        },
        3,
    )?;
    let property_id = changed
        .elements
        .last()
        .ok_or("missing new Property")?
        .id
        .clone();
    let inspected =
        inspect_element(&ModelingSection::from_model(&changed), "attribute").ok_or("inspector")?;
    assert!(
        inspected
            .properties
            .iter()
            .any(|field| field.path == "properties.type"
                && field.editor == PropertyEditor::Reference
                && field.value.is_null())
    );
    assert!(
        inspected
            .properties
            .iter()
            .any(|field| field.path == "legacy.type"
                && field.value == json!("String")
                && !field.editable)
    );
    assert!(
        inspected
            .reference_choices
            .iter()
            .any(|choice| choice.id == type_id && choice.label.contains("Root::String"))
    );
    assert!(
        !inspected
            .reference_choices
            .iter()
            .any(|choice| choice.id == "root")
    );
    assert!(edit(&changed, "attribute", "type", json!("String")).is_err());
    let mut conflicting_text = changed.clone();
    conflicting_text
        .elements
        .iter_mut()
        .find(|element| element.id == "attribute")
        .ok_or("Property")?
        .properties
        .insert("typeText".into(), json!("Different authored text"));
    let conflicting_before = conflicting_text.clone();
    assert!(
        edit(
            &conflicting_text,
            "attribute",
            "type",
            json!({"$ref":type_id})
        )
        .is_err()
    );
    assert_eq!(conflicting_text, conflicting_before);
    let changed = edit(&changed, "attribute", "type", json!({"$ref":type_id}))?;
    let changed = edit(&changed, &property_id, "type", json!({"$ref":"class"}))?;
    assert_eq!(
        changed
            .element("attribute")
            .ok_or("Property")?
            .properties
            .get("typeText"),
        Some(&json!("String"))
    );
    let before_invalid = changed.clone();
    for invalid in [
        json!({"$ref":"missing"}),
        json!({"$ref":"root"}),
        json!([]),
        json!({"href":"library#String"}),
        json!({"$ref":type_id,"extra":true}),
    ] {
        assert!(edit(&changed, "attribute", "type", invalid).is_err());
        assert_eq!(changed, before_invalid);
    }
    assert!(validate_edit("Operation", "type", &json!("String")).is_err());
    let operation = inspect_element(&ModelingSection::from_model(&changed), "operation")
        .ok_or("Operation inspector")?;
    assert!(
        operation
            .properties
            .iter()
            .any(|field| field.path == "properties.type"
                && !field.editable
                && field.editor == PropertyEditor::ReadOnly)
    );
    assert!(edit(&changed, "operation", "type", json!({"$ref":type_id})).is_err());
    assert!(
        commands::apply(
            &changed,
            &Command::DeleteElement {
                element_id: type_id.clone()
            },
            4
        )
        .is_err()
    );
    assert_eq!(changed, before_invalid);
    let changed = commands::apply(
        &changed,
        &Command::Rename {
            element_id: type_id.clone(),
            name: "Text".into(),
        },
        4,
    )?;
    let section = ModelingSection::from_model(&changed);
    let references = super::super::references::ReferenceIndex::new(
        super::super::root::Model::from_section(&section),
    );
    let label = crate::element_display_label_with_references(
        changed.element("attribute").ok_or("Property")?,
        &references,
    );
    assert!(label.contains(": Text"));
    assert!(!label.contains("String"));
    assert_eq!(
        inspect_element(&section, "attribute")
            .ok_or("Property inspector")?
            .display_label,
        label
    );
    assert_eq!(
        inspect_element(&section, "class")
            .ok_or("Class inspector")?
            .attributes
            .iter()
            .find(|member| member.id == "attribute")
            .ok_or("member")?
            .display_label,
        label
    );
    assert_eq!(
        crate::project_hierarchy(&section)
            .iter()
            .find(|row| row.element_id == "attribute")
            .ok_or("tree row")?
            .label,
        label
    );
    let cleared = edit(&changed, "attribute", "type", Value::Null)?;
    assert!(
        !cleared
            .element("attribute")
            .ok_or("Property")?
            .properties
            .contains_key("type")
    );
    assert_eq!(
        cleared
            .element("attribute")
            .ok_or("Property")?
            .properties
            .get("typeText"),
        Some(&json!("String"))
    );
    let mut dangling = changed.clone();
    dangling.elements.retain(|element| element.id != type_id);
    assert!(
        crate::validate_structural_model(&dangling)
            .iter()
            .any(|issue| issue.code == "UML_TYPED_ELEMENT_TYPE" && issue.subject_id == "attribute")
    );
    let mut wrong_type = changed.clone();
    wrong_type
        .elements
        .iter_mut()
        .find(|element| element.id == type_id)
        .ok_or("Type")?
        .kind = "uml.comment".into();
    assert!(
        crate::validate_structural_model(&wrong_type)
            .iter()
            .any(|issue| issue.code == "UML_TYPED_ELEMENT_TYPE" && issue.subject_id == "attribute")
    );
    let mut duplicate_type = changed.clone();
    duplicate_type
        .elements
        .push(changed.element(&type_id).ok_or("Type")?.clone());
    let references = super::super::references::ReferenceIndex::new(
        super::super::root::Model::from_project(&duplicate_type),
    );
    assert!(
        references
            .validate(contract("Property", "type")?, &json!({"$ref":type_id}))
            .is_err()
    );
    dangling.root_model = None;
    assert!(
        crate::validate_structural_model(&dangling)
            .iter()
            .any(|issue| issue.code == "UML_TYPED_ELEMENT_TYPE")
    );
    let changed = edit(&changed, "attribute", "defaultValue", json!("[]"))?;
    let changed = edit(&changed, "attribute", "visibility", json!("private"))?;
    let changed = edit(&changed, "attribute", "visibility", Value::Null)?;
    assert!(edit(&changed, "class", "isAbstract", Value::Null).is_err());
    assert!(uses_legacy_text_adapter(
        contract("Property", "type")?,
        &json!("Vec<String>")
    ));
    for format in [Format::Json, Format::Protobuf] {
        let decoded = crate::decode(&crate::encode(&changed, format)?, format)?;
        assert_eq!(decoded, changed);
        assert_legacy_contract_warnings(
            super::super::root::Model::from_project(&decoded).validate(),
            "typeText: no normative property or registered extension contract",
        );
        assert_eq!(
            decoded
                .element("attribute")
                .ok_or("missing attribute")?
                .properties
                .get("type"),
            Some(&json!({"$ref":type_id}))
        );
    }
    let mut malformed = changed.clone();
    malformed
        .elements
        .iter_mut()
        .find(|element| element.id == "class")
        .ok_or("missing Class")?
        .properties
        .insert("isAbstract".into(), json!("true"));
    assert!(
        crate::validate_structural_model(&malformed)
            .iter()
            .any(|issue| issue.subject_id == "class" && issue.code == "UML_PROPERTY_VALUE")
    );
    assert_legacy_contract_warnings(
        crate::validate_structural_model(&changed),
        "typeText: no normative property or registered extension contract",
    );
    assert_legacy_contract_warnings(
        crate::validate_structural_model(&original),
        "TypedElement-type: text declaration is not a resolved UML reference",
    );
    assert_eq!(
        original
            .element("attribute")
            .ok_or("missing original Property")?
            .properties
            .get("type"),
        Some(&json!("String"))
    );
    Ok(())
}

#[test]
fn unbounded_scalar_edits_preserve_exact_values_and_reject_invalid_typed_literals() -> TestResult {
    let mut project = fixture()?;
    let large = "1234567890".repeat(35);
    for (id, kind) in [
        ("integer", "uml.literal_integer"),
        ("unlimited", "uml.literal_unlimited_natural"),
    ] {
        project.elements.push(serde_json::from_value(
            json!({"id":id,"kind":kind,"name":id,"properties":{"value":"0"}}),
        )?);
        project.relations.push(serde_json::from_value(
            json!({"id":format!("owns-{id}"),"kind":"owns","from":"root","to":id}),
        )?);
    }
    project.elements.push(serde_json::from_value(json!({"id":"string", "kind":"uml.literal_string", "name":"string", "properties":{"value":null}}))?);
    project.relations.push(serde_json::from_value(
        json!({"id":"owns-string", "kind":"owns", "from":"root", "to":"string"}),
    )?);
    let project = edit(&project, "string", "value", json!("text"))?;
    let project = edit(&project, "string", "value", Value::Null)?;
    let edited = edit(&project, "integer", "value", json!(format!("-{large}")))?;
    let edited = edit(&edited, "unlimited", "value", json!(large))?;
    assert!(edit(&edited, "unlimited", "value", json!("-1")).is_err());
    assert!(edit(&edited, "integer", "value", json!("1.5")).is_err());
    assert!(edit(&edited, "integer", "value", json!("*")).is_err());
    let edited = edit(&edited, "unlimited", "value", json!("*"))?;
    for format in [Format::Json, Format::Protobuf] {
        let decoded = crate::decode(&crate::encode(&edited, format)?, format)?;
        assert_eq!(decoded, edited);
        assert_legacy_contract_warnings(
            super::super::root::Model::from_project(&decoded).validate(),
            "TypedElement-type: text declaration is not a resolved UML reference",
        );
        assert_eq!(
            decoded
                .element("integer")
                .ok_or("missing literal")?
                .properties
                .get("value"),
            Some(&json!(format!("-{large}")))
        );
    }
    Ok(())
}
