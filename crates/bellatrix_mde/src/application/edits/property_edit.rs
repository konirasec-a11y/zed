use crate::ModelingSection;
use crate::uml::root::ElementRef;
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PropertyValueType {
    String,
    Boolean,
    Number,
    Choice(&'static [&'static str]),
    Reference,
    Stereotype,
    ReadOnly,
}

pub(crate) struct PropertyContract {
    pub(crate) value_type: PropertyValueType,
    pub(crate) value: Value,
}

pub(crate) const VISIBILITY: &[&str] = &["public", "private", "protected", "package"];

#[derive(Clone, Copy)]
pub(crate) struct DeclaredProperty {
    pub(crate) key: &'static str,
    pub(crate) value_type: PropertyValueType,
    pub(crate) default: fn() -> Value,
}
fn empty() -> Value {
    json!("")
}
fn public() -> Value {
    json!("public")
}
fn false_value() -> Value {
    json!(false)
}
fn one_value() -> Value {
    json!("1")
}

const COMMON_CLASSIFIER: &[DeclaredProperty] = &[
    DeclaredProperty {
        key: "stereotype",
        value_type: PropertyValueType::Stereotype,
        default: empty,
    },
    DeclaredProperty {
        key: "visibility",
        value_type: PropertyValueType::Choice(VISIBILITY),
        default: public,
    },
    DeclaredProperty {
        key: "isAbstract",
        value_type: PropertyValueType::Boolean,
        default: false_value,
    },
    DeclaredProperty {
        key: "isFinalSpecialization",
        value_type: PropertyValueType::Boolean,
        default: false_value,
    },
    DeclaredProperty {
        key: "isLeaf",
        value_type: PropertyValueType::Boolean,
        default: false_value,
    },
];
const CLASS: &[DeclaredProperty] = &[DeclaredProperty {
    key: "isActive",
    value_type: PropertyValueType::Boolean,
    default: false_value,
}];
const COMMENT: &[DeclaredProperty] = &[DeclaredProperty {
    key: "body",
    value_type: PropertyValueType::String,
    default: empty,
}];
const FIELD: &[DeclaredProperty] = &[
    DeclaredProperty {
        key: "aggregation",
        value_type: PropertyValueType::Choice(&["none", "shared", "composite"]),
        default: empty,
    },
    DeclaredProperty {
        key: "multiplicity",
        value_type: PropertyValueType::String,
        default: empty,
    },
    DeclaredProperty {
        key: "visibility",
        value_type: PropertyValueType::Choice(VISIBILITY),
        default: public,
    },
    DeclaredProperty {
        key: "type",
        value_type: PropertyValueType::String,
        default: empty,
    },
    DeclaredProperty {
        key: "defaultValue",
        value_type: PropertyValueType::Reference,
        default: empty,
    },
    DeclaredProperty {
        key: "isStatic",
        value_type: PropertyValueType::Boolean,
        default: false_value,
    },
    DeclaredProperty {
        key: "isReadOnly",
        value_type: PropertyValueType::Boolean,
        default: false_value,
    },
];
const PARAMETER: &[DeclaredProperty] = &[
    DeclaredProperty {
        key: "isOrdered",
        value_type: PropertyValueType::Boolean,
        default: false_value,
    },
    DeclaredProperty {
        key: "isUnique",
        value_type: PropertyValueType::Boolean,
        default: false_value,
    },
    DeclaredProperty {
        key: "multiplicity",
        value_type: PropertyValueType::String,
        default: one_value,
    },
    DeclaredProperty {
        key: "direction",
        value_type: PropertyValueType::ReadOnly,
        default: empty,
    },
    DeclaredProperty {
        key: "type",
        value_type: PropertyValueType::Reference,
        default: empty,
    },
];
const FUNCTION: &[DeclaredProperty] = &[
    DeclaredProperty {
        key: "isOrdered",
        value_type: PropertyValueType::ReadOnly,
        default: empty,
    },
    DeclaredProperty {
        key: "isUnique",
        value_type: PropertyValueType::ReadOnly,
        default: empty,
    },
    DeclaredProperty {
        key: "lower",
        value_type: PropertyValueType::ReadOnly,
        default: empty,
    },
    DeclaredProperty {
        key: "upper",
        value_type: PropertyValueType::ReadOnly,
        default: empty,
    },
    DeclaredProperty {
        key: "type",
        value_type: PropertyValueType::ReadOnly,
        default: empty,
    },
    DeclaredProperty {
        key: "visibility",
        value_type: PropertyValueType::Choice(VISIBILITY),
        default: public,
    },
    DeclaredProperty {
        key: "isStatic",
        value_type: PropertyValueType::Boolean,
        default: false_value,
    },
    DeclaredProperty {
        key: "isAbstract",
        value_type: PropertyValueType::Boolean,
        default: false_value,
    },
    DeclaredProperty {
        key: "isQuery",
        value_type: PropertyValueType::Boolean,
        default: false_value,
    },
];

pub(crate) fn fields_for(kind: &str) -> impl Iterator<Item = &DeclaredProperty> {
    let semantic_type = crate::UmlType::from_kind(kind);
    COMMON_CLASSIFIER
        .iter()
        .chain(CLASS.iter())
        .filter(move |field| {
            semantic_type.is_some_and(|kind| {
                // The legacy editor template alias is not a normative metaclass.
                if kind == crate::UmlType::Template {
                    return COMMON_CLASSIFIER
                        .iter()
                        .any(|declared| declared.key == field.key);
                }
                if field.key == "stereotype" {
                    kind.is_classifier()
                } else {
                    kind.has_property(field.key)
                }
            })
        })
        .chain(
            match kind {
                "rust.field" | "uml.attribute" | "uml.property" | "uml.part" | "uml.port" => FIELD,
                "uml.comment" | "uml.note" => COMMENT,
                "rust.function" | "uml.operation" => FUNCTION,
                "uml.parameter" => PARAMETER,
                _ => &[],
            }
            .iter(),
        )
}

pub(crate) fn relationship_fields(
    element: ElementRef<'_>,
) -> impl Iterator<Item = DeclaredProperty> {
    const COMMON: &[DeclaredProperty] = &[
        DeclaredProperty {
            key: "name",
            value_type: PropertyValueType::String,
            default: empty,
        },
        DeclaredProperty {
            key: "sourceMultiplicity",
            value_type: PropertyValueType::String,
            default: empty,
        },
        DeclaredProperty {
            key: "targetMultiplicity",
            value_type: PropertyValueType::String,
            default: empty,
        },
        DeclaredProperty {
            key: "sourceRole",
            value_type: PropertyValueType::String,
            default: empty,
        },
        DeclaredProperty {
            key: "targetRole",
            value_type: PropertyValueType::String,
            default: empty,
        },
        DeclaredProperty {
            key: "stereotype",
            value_type: PropertyValueType::Stereotype,
            default: empty,
        },
    ];
    fn no_aggregation() -> Value {
        json!("none")
    }
    const ENDS: &[DeclaredProperty] = &[
        DeclaredProperty {
            key: "end1Aggregation",
            value_type: PropertyValueType::Choice(&["none", "shared", "composite"]),
            default: no_aggregation,
        },
        DeclaredProperty {
            key: "end2Aggregation",
            value_type: PropertyValueType::Choice(&["none", "shared", "composite"]),
            default: no_aggregation,
        },
    ];
    COMMON
        .iter()
        .copied()
        .filter(move |definition| match definition.key {
            "name" => element.conforms_to("NamedElement"),
            "sourceMultiplicity" | "targetMultiplicity" | "sourceRole" | "targetRole" => {
                element.conforms_to("Association")
            }
            _ => true,
        })
        .chain(
            ENDS.iter()
                .copied()
                .filter(move |_| element.metaclass() == Some("Association")),
        )
}

pub(crate) fn normalize_property_contract(
    element: ElementRef<'_>,
    key: &str,
    value_type: PropertyValueType,
    value: Value,
) -> PropertyContract {
    use crate::uml::schema::{self, ScalarKind};
    use crate::uml::types::StandardPrimitiveType;
    let mut contract = PropertyContract { value_type, value };
    if key == "name" && matches!(element, ElementRef::Object(_)) {
        // Objects edit the authoritative name projection through the name path.
        contract.value_type = PropertyValueType::ReadOnly;
        return contract;
    }
    if key == "navigable" && element.conforms_to("Association") {
        // UML navigability belongs to individual ends; a single Boolean cannot
        // represent Association::navigableOwnedEnd or classifier-owned ends.
        contract.value_type = PropertyValueType::ReadOnly;
        return contract;
    }
    if key == "stereotype" {
        contract.value_type = match crate::StereotypeValue::read(Some(&contract.value)) {
            crate::StereotypeValue::Null
            | crate::StereotypeValue::Text(_)
            | crate::StereotypeValue::Reference(_) => PropertyValueType::Stereotype,
            _ => PropertyValueType::ReadOnly,
        };
        return contract;
    }
    let Some(metaclass) = element.metaclass() else {
        return contract;
    };
    let Ok(Some(descriptor)) = schema::uml_property(metaclass, key) else {
        return contract;
    };
    if (key == "ownedParameter" && element.conforms_to("Operation"))
        || (key == "operation" && element.conforms_to("Parameter"))
        || descriptor.is_read_only
        || descriptor.is_derived
    {
        contract.value_type = PropertyValueType::ReadOnly;
        return contract;
    }
    if !matches!(
        crate::contract_edits::field_shape(metaclass, key),
        Ok(Some(_))
    ) {
        // The normative schema can expose fields missing from the generated
        // concrete contract. Offering an editor would lose or reject that edit.
        contract.value_type = PropertyValueType::ReadOnly;
        return contract;
    }
    if key == "type" && (element.conforms_to("Property") || element.conforms_to("Parameter")) {
        contract.value_type = PropertyValueType::Reference;
        if contract.value.is_string() {
            contract.value = Value::Null;
        }
        return contract;
    }
    if schema::uses_legacy_text_adapter(descriptor, &contract.value) {
        return contract;
    }
    let kind = match schema::scalar_kind(descriptor) {
        Ok(kind) => kind,
        Err(schema::UnsupportedContract::Reference)
            if (contract.value.is_null() || contract.value.is_object())
                && matches!(descriptor.upper, "0" | "1") =>
        {
            // Clearing or replacing a reference must not lock subsequent edits.
            contract.value_type = PropertyValueType::Reference;
            return contract;
        }
        Err(_) => return contract,
    };
    contract.value_type = match kind {
        ScalarKind::Enumeration(literals) => PropertyValueType::Choice(literals),
        ScalarKind::Primitive(StandardPrimitiveType::Boolean) => PropertyValueType::Boolean,
        ScalarKind::Primitive(
            StandardPrimitiveType::Integer
            | StandardPrimitiveType::UnlimitedNatural
            | StandardPrimitiveType::String,
        ) => PropertyValueType::String,
        ScalarKind::Primitive(StandardPrimitiveType::Real) => PropertyValueType::Number,
    };
    if !element.properties().contains_key(key) {
        match schema::effective_default(descriptor) {
            Ok(Some(default)) => {
                if let Some(value) = default.scalar_json() {
                    contract.value = value;
                }
            }
            Ok(None) if descriptor.lower == "0" && key != "body" => contract.value = Value::Null,
            _ => {}
        }
    }
    contract
}

fn authored_value_type(value: &Value) -> PropertyValueType {
    match value {
        Value::String(_) => PropertyValueType::String,
        Value::Bool(_) => PropertyValueType::Boolean,
        Value::Number(_) => PropertyValueType::Number,
        _ => PropertyValueType::ReadOnly,
    }
}

pub(crate) fn navigation_contract(
    references: &crate::uml::references::ReferenceIndex<'_>,
    id: &str,
) -> Result<PropertyContract, String> {
    use crate::uml::associations;
    let knowledge = associations::navigation_knowledge(references, id)?;
    let editable = associations::owning_association(references, id)?.is_some();
    Ok(PropertyContract {
        value_type: if editable {
            PropertyValueType::Choice(&["navigable", "notNavigable", "unverified"])
        } else {
            PropertyValueType::ReadOnly
        },
        value: json!(match knowledge {
            crate::NavigationKnowledge::Known(true) => "navigable",
            crate::NavigationKnowledge::Known(false) => "notNavigable",
            crate::NavigationKnowledge::Unverified => "unverified",
        }),
    })
}

pub(crate) fn property_contract(
    section: &ModelingSection,
    id: &str,
    path: &str,
) -> Result<PropertyContract, String> {
    if path == "authoring.navigation" {
        return navigation_contract(
            &crate::uml::references::ReferenceIndex::new(crate::uml::root::Model::from_section(
                section,
            )),
            id,
        );
    }
    let element = if let Some(relation) = section.relations.iter().find(|relation| {
        relation.id == id
            && (crate::uml::root::relationship_metaclass(&relation.kind).is_some()
                || relation.kind.starts_with("uml.")
                || section.diagrams.iter().any(|diagram| {
                    diagram
                        .views
                        .iter()
                        .any(|view| view.subject.as_deref() == Some(id))
                }))
    }) {
        ElementRef::Relationship(relation)
    } else {
        ElementRef::Object(
            section
                .elements
                .iter()
                .find(|element| element.id == id)
                .ok_or("element not found")?,
        )
    };
    let read_only = || PropertyContract {
        value_type: PropertyValueType::ReadOnly,
        value: Value::Null,
    };
    let Some(key) = path.strip_prefix("properties.") else {
        if matches!(element, ElementRef::Object(_))
            && matches!(path, "name" | "label" | "description")
        {
            return Ok(PropertyContract {
                value_type: PropertyValueType::String,
                value: Value::Null,
            });
        }
        if matches!(path, "id" | "kind" | "from" | "to")
            || path.starts_with("uml.")
            || path.starts_with("legacy.")
        {
            return Ok(read_only());
        }
        return Err("property is not declared".into());
    };
    let (value_type, value) = match element {
        ElementRef::Object(object) => {
            if let Some(definition) =
                fields_for(&object.kind).find(|definition| definition.key == key)
            {
                (
                    definition.value_type,
                    object.properties.get(key).cloned().unwrap_or_else(|| {
                        if key == "body" && element.comment().is_some() {
                            json!(object.name)
                        } else {
                            (definition.default)()
                        }
                    }),
                )
            } else if let Some(value) = object.properties.get(key) {
                (authored_value_type(value), value.clone())
            } else {
                return Err("property is not declared".into());
            }
        }
        ElementRef::Relationship(relation) => {
            if matches!(key, "end1Aggregation" | "end2Aggregation") {
                if element.metaclass() != Some("Association") {
                    return Err("diamond editing requires an Association".into());
                }
                let references = crate::uml::references::ReferenceIndex::new(
                    crate::uml::root::Model::from_section(section),
                );
                let ends = crate::uml::associations::member_ends(&references, &relation.id)?;
                let end = ends
                    .get(if key == "end1Aggregation" { 1 } else { 0 })
                    .ok_or("Association member end is missing")?;
                return Ok(PropertyContract {
                    value_type: PropertyValueType::Choice(&["none", "shared", "composite"]),
                    // Keep an invalid authored value editable so the user can repair it.
                    value: end
                        .properties
                        .get("aggregation")
                        .cloned()
                        .unwrap_or_else(|| json!("none")),
                });
            }
            if let Some(definition) =
                relationship_fields(element).find(|definition| definition.key == key)
            {
                (
                    definition.value_type,
                    relation
                        .properties
                        .get(key)
                        .cloned()
                        .unwrap_or_else(definition.default),
                )
            } else if let Some(value) = relation.properties.get(key) {
                (authored_value_type(value), value.clone())
            } else {
                return Err("property is not declared".into());
            }
        }
    };
    let mut contract = normalize_property_contract(element, key, value_type, value);
    if matches!(element, ElementRef::Object(_)) {
        if key == "multiplicity" && element.conforms_to("MultiplicityElement") {
            let references = crate::uml::references::ReferenceIndex::new(
                crate::uml::root::Model::from_section(section),
            );
            match crate::uml::value_specifications::multiplicity(&references, element) {
                Ok(bounds) => {
                    contract.value_type = PropertyValueType::String;
                    let lower = bounds.lower_bound().to_string();
                    let upper = bounds.upper_bound().to_string();
                    contract.value = json!(if lower == upper {
                        lower
                    } else {
                        format!("{lower}..{upper}")
                    });
                }
                Err(error) => {
                    contract.value_type = PropertyValueType::ReadOnly;
                    contract.value = json!(error.to_string());
                }
            }
        }
    }
    Ok(contract)
}
