//! UML 2.5.1 Element and NamedElement graph queries over one immutable snapshot.
//! The XMI operation bodies define the closure and the ordered Namespace chain.

use super::{
    references::ReferenceIndex,
    root::{Element, ElementRef},
    schema::{self, Aggregation},
};
use serde_json::Value;
use std::collections::{HashSet, VecDeque};

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ElementGraphError {
    #[error("Invalid UML graph at {subject_id}: {message}")]
    Invalid { subject_id: String, message: String },
    #[error("Unsupported UML graph representation at {subject_id}: {message}")]
    Unsupported { subject_id: String, message: String },
}

impl ElementGraphError {
    fn invalid(subject_id: &str, message: impl Into<String>) -> Self {
        Self::Invalid {
            subject_id: subject_id.into(),
            message: message.into(),
        }
    }
    fn unsupported(subject_id: &str, message: impl Into<String>) -> Self {
        Self::Unsupported {
            subject_id: subject_id.into(),
            message: message.into(),
        }
    }
}

fn populated(value: &Value) -> bool {
    !value.is_null() && !value.as_array().is_some_and(Vec::is_empty)
}

fn element<'a>(index: &ReferenceIndex<'a>, id: &str) -> Result<ElementRef<'a>, ElementGraphError> {
    index
        .element(id)
        .map_err(|error| ElementGraphError::invalid(id, error))
}

fn named_element<'a>(
    index: &ReferenceIndex<'a>,
    id: &str,
) -> Result<ElementRef<'a>, ElementGraphError> {
    let element = element(index, id)?;
    if element.named().is_none() {
        return Err(ElementGraphError::invalid(id, "expected a NamedElement"));
    }
    Ok(element)
}

fn ensure_derived_ownership(element: ElementRef<'_>) -> Result<(), ElementGraphError> {
    for name in ["owner", "ownedElement"] {
        if element.properties().get(name).is_some_and(populated) {
            return Err(ElementGraphError::unsupported(
                element.id(),
                format!("{name} must be derived from canonical ownership"),
            ));
        }
    }
    Ok(())
}

fn ensure_canonical_ownership(element: ElementRef<'_>) -> Result<(), ElementGraphError> {
    ensure_derived_ownership(element)?;
    let Some(classifier) = element.metaclass().and_then(schema::uml_classifier) else {
        return Err(ElementGraphError::unsupported(
            element.id(),
            "unknown UML metaclass",
        ));
    };
    let properties = schema::inherited_properties(classifier.id)
        .map_err(|error| ElementGraphError::unsupported(element.id(), error.to_string()))?;
    if !properties.unresolved.is_empty() {
        return Err(ElementGraphError::unsupported(
            element.id(),
            "incomplete metaclass inheritance",
        ));
    }
    for property in properties.properties {
        if property.aggregation == Aggregation::Composite
            && !property.is_derived
            && element
                .properties()
                .get(property.name)
                .is_some_and(populated)
        {
            return Err(ElementGraphError::unsupported(
                element.id(),
                format!("{} requires a canonical ownership adapter", property.name),
            ));
        }
    }
    Ok(())
}

/// Returns a set in deterministic snapshot traversal order; UML does not order this result.
pub fn all_owned_elements<'a>(
    index: &ReferenceIndex<'a>,
    id: &str,
) -> Result<Vec<ElementRef<'a>>, ElementGraphError> {
    let root = element(index, id)?;
    let mut seen = HashSet::from([root.id()]);
    let mut pending = VecDeque::from([root]);
    let mut result = Vec::new();
    while let Some(owner) = pending.pop_front() {
        ensure_canonical_ownership(owner)?;
        index
            .explicit_owned_elements(owner.id())
            .map_err(|error| ElementGraphError::invalid(owner.id(), error))?;
        for child in index
            .model()
            .owned_elements(owner)
            .map_err(|error| ElementGraphError::invalid(owner.id(), error))?
        {
            element(index, child.id())?;
            if !seen.insert(child.id()) {
                return Err(ElementGraphError::invalid(
                    child.id(),
                    "cycle or duplicate in UML ownership",
                ));
            }
            result.push(child);
            pending.push_back(child);
        }
    }
    Ok(result)
}

fn namespace<'a>(
    index: &ReferenceIndex<'a>,
    current: ElementRef<'a>,
    expose_template_parameter: bool,
) -> Result<Option<ElementRef<'a>>, ElementGraphError> {
    ensure_derived_ownership(current)?;
    if current.properties().get("namespace").is_some_and(populated) {
        return Err(ElementGraphError::unsupported(
            current.id(),
            "namespace must be derived from canonical ownership",
        ));
    }
    let model = index.model();
    let owner = model
        .owner(current)
        .map_err(|error| ElementGraphError::invalid(current.id(), error))?;
    if let Some(owner) = owner {
        element(index, owner.id())?;
        ensure_canonical_ownership(owner)?;
        index
            .explicit_owned_elements(owner.id())
            .map_err(|error| ElementGraphError::invalid(owner.id(), error))?;
        if expose_template_parameter && owner.conforms_to("TemplateParameter") {
            if owner.properties().get("signature").is_some_and(populated) {
                return Err(ElementGraphError::unsupported(
                    owner.id(),
                    "signature must use canonical ownership",
                ));
            }
            let signature = model
                .owner(owner)
                .map_err(|error| ElementGraphError::invalid(owner.id(), error))?
                .ok_or_else(|| {
                    ElementGraphError::invalid(owner.id(), "TemplateParameter has no signature")
                })?;
            element(index, signature.id())?;
            ensure_canonical_ownership(signature)?;
            if signature
                .properties()
                .get("template")
                .is_some_and(populated)
                || signature
                    .properties()
                    .get("classifier")
                    .is_some_and(populated)
            {
                return Err(ElementGraphError::unsupported(
                    signature.id(),
                    "template must use canonical ownership",
                ));
            }
            if !signature.conforms_to("TemplateSignature") {
                return Err(ElementGraphError::invalid(
                    owner.id(),
                    "TemplateParameter owner must be a TemplateSignature",
                ));
            }
            let template = model
                .owner(signature)
                .map_err(|error| ElementGraphError::invalid(signature.id(), error))?
                .ok_or_else(|| {
                    ElementGraphError::invalid(signature.id(), "TemplateSignature has no template")
                })?;
            element(index, template.id())?;
            ensure_canonical_ownership(template)?;
            if !template.conforms_to("TemplateableElement") {
                return Err(ElementGraphError::invalid(
                    template.id(),
                    "TemplateSignature owner must be a TemplateableElement",
                ));
            }
            return Ok(template.namespace().map(|_| template));
        }
    }
    let named = current
        .named()
        .ok_or_else(|| ElementGraphError::invalid(current.id(), "expected a NamedElement"))?;
    model
        .namespace(named)
        .map(|namespace| namespace.map(|namespace| namespace.as_element()))
        .map_err(|error| ElementGraphError::unsupported(current.id(), error))
}

/// Nearest Namespace first, as ordered by NamedElement::allNamespaces.
pub fn all_namespaces<'a>(
    index: &ReferenceIndex<'a>,
    id: &str,
) -> Result<Vec<ElementRef<'a>>, ElementGraphError> {
    let mut current = named_element(index, id)?;
    let mut visited = HashSet::from([current.id()]);
    let mut result = Vec::new();
    while let Some(parent) = namespace(index, current, true)? {
        if !visited.insert(parent.id()) {
            return Err(ElementGraphError::invalid(
                parent.id(),
                "cycle in UML namespace ownership",
            ));
        }
        result.push(parent);
        current = parent;
    }
    Ok(result)
}

/// Follows namespace only while each immediate namespace is a Package (§7.8.9).
pub fn all_owning_packages<'a>(
    index: &ReferenceIndex<'a>,
    id: &str,
) -> Result<Vec<ElementRef<'a>>, ElementGraphError> {
    let mut current = named_element(index, id)?;
    let mut visited = HashSet::from([current.id()]);
    let mut result = Vec::new();
    while let Some(parent) = namespace(index, current, false)? {
        if !parent.conforms_to("Package") {
            break;
        }
        if !visited.insert(parent.id()) {
            return Err(ElementGraphError::invalid(
                parent.id(),
                "cycle in UML namespace ownership",
            ));
        }
        result.push(parent);
        current = parent;
    }
    Ok(result)
}

/// Computes the inverse of canonical Dependency client endpoints, including subclasses.
pub fn client_dependencies<'a>(
    index: &ReferenceIndex<'a>,
    id: &str,
) -> Result<Vec<ElementRef<'a>>, ElementGraphError> {
    let subject = named_element(index, id)?;
    if subject
        .properties()
        .get("clientDependency")
        .is_some_and(populated)
    {
        return Err(ElementGraphError::unsupported(
            id,
            "clientDependency must be derived from Dependency clients",
        ));
    }
    let mut result = Vec::new();
    for dependency in index
        .model()
        .elements()
        .filter(|element| element.conforms_to("Dependency"))
    {
        element(index, dependency.id())?;
        let ElementRef::Relationship(relation) = dependency else {
            return Err(ElementGraphError::unsupported(
                dependency.id(),
                "Dependency object clients require a canonical relationship adapter",
            ));
        };
        if relation.properties.get("client").is_some_and(populated)
            || relation.properties.get("supplier").is_some_and(populated)
        {
            return Err(ElementGraphError::unsupported(
                dependency.id(),
                "Dependency client and supplier must use canonical endpoints",
            ));
        }
        named_element(index, &relation.from)?;
        named_element(index, &relation.to)?;
        if relation.from == id {
            result.push(dependency);
        }
    }
    Ok(result)
}
