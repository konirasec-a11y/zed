use crate::{
    Element as ElementRecord, ModelingSection, Project, Relation, StructuralDiagnostic,
    StructuralSeverity, UmlType,
};
use serde_json::Value;
use std::collections::HashSet;

/// A borrowed identity in the canonical model. UML Relationships are Elements too.
#[derive(Clone, Copy, Debug)]
pub enum ElementRef<'a> {
    Object(&'a ElementRecord),
    Relationship(&'a Relation),
}

#[derive(Clone, Copy, Debug)]
pub struct NamedElementRef<'a>(ElementRef<'a>);

#[derive(Clone, Copy, Debug)]
pub struct NamespaceRef<'a>(NamedElementRef<'a>);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VisibilityKind {
    Public,
    Private,
    Protected,
    Package,
}

/// UML §7.8.6: identity and derived composite ownership, independent of storage layout.
pub trait Element<'a>: Copy {
    fn as_element(self) -> ElementRef<'a>;
    fn id(self) -> &'a str {
        self.as_element().id()
    }
    fn owner(self, model: Model<'a>) -> Result<Option<ElementRef<'a>>, String> {
        model.owner(self)
    }
    fn owned_elements(self, model: Model<'a>) -> Result<Vec<ElementRef<'a>>, String> {
        model.owned_elements(self)
    }
    fn owned_comments(self, model: Model<'a>) -> Result<Vec<CommentRef<'a>>, String> {
        Ok(self
            .owned_elements(model)?
            .into_iter()
            .filter_map(ElementRef::comment)
            .collect())
    }
}

/// UML §7.8.9: a name is optional; derived namespace/name data is never stored twice.
pub trait NamedElement<'a>: Element<'a> {
    fn name(self) -> Option<&'a str>;
    fn visibility(self) -> Option<VisibilityKind>;
    fn namespace(self, model: Model<'a>) -> Result<Option<NamespaceRef<'a>>, String> {
        model.namespace(self)
    }
    fn qualified_name(self, model: Model<'a>) -> Result<Option<String>, String> {
        model.qualified_name(self)
    }
}

pub trait Namespace<'a>: NamedElement<'a> {
    fn owned_members(self, model: Model<'a>) -> Result<Vec<NamedElementRef<'a>>, String> {
        Ok(self
            .owned_elements(model)?
            .into_iter()
            .filter_map(ElementRef::named)
            .collect())
    }
}

pub trait Relationship<'a>: Element<'a> {
    fn related_elements(self, model: Model<'a>) -> Result<Vec<ElementRef<'a>>, String>;
}

pub trait DirectedRelationship<'a>: Relationship<'a> {
    fn sources(self, model: Model<'a>) -> Result<Vec<ElementRef<'a>>, String>;
    fn targets(self, model: Model<'a>) -> Result<Vec<ElementRef<'a>>, String>;
}

#[derive(Clone, Copy, Debug)]
pub struct RelationshipRef<'a>(&'a Relation);
#[derive(Clone, Copy, Debug)]
pub struct DirectedRelationshipRef<'a>(RelationshipRef<'a>);
#[derive(Clone, Copy, Debug)]
pub struct CommentRef<'a>(&'a ElementRecord);

impl<'a> Element<'a> for ElementRef<'a> {
    fn as_element(self) -> ElementRef<'a> {
        self
    }
}
impl<'a> Element<'a> for NamedElementRef<'a> {
    fn as_element(self) -> ElementRef<'a> {
        self.0
    }
}
impl<'a> Element<'a> for NamespaceRef<'a> {
    fn as_element(self) -> ElementRef<'a> {
        self.0.0
    }
}
impl<'a> Element<'a> for RelationshipRef<'a> {
    fn as_element(self) -> ElementRef<'a> {
        ElementRef::Relationship(self.0)
    }
}
impl<'a> Element<'a> for DirectedRelationshipRef<'a> {
    fn as_element(self) -> ElementRef<'a> {
        self.0.as_element()
    }
}
impl<'a> Element<'a> for CommentRef<'a> {
    fn as_element(self) -> ElementRef<'a> {
        ElementRef::Object(self.0)
    }
}
impl<'a> NamedElement<'a> for NamedElementRef<'a> {
    fn name(self) -> Option<&'a str> {
        NamedElementRef::name(self)
    }
    fn visibility(self) -> Option<VisibilityKind> {
        NamedElementRef::visibility(self)
    }
}
impl<'a> NamedElement<'a> for NamespaceRef<'a> {
    fn name(self) -> Option<&'a str> {
        self.0.name()
    }
    fn visibility(self) -> Option<VisibilityKind> {
        self.0.visibility()
    }
}
impl<'a> Namespace<'a> for NamespaceRef<'a> {}
impl<'a> Relationship<'a> for RelationshipRef<'a> {
    fn related_elements(self, model: Model<'a>) -> Result<Vec<ElementRef<'a>>, String> {
        let mut related = Vec::new();
        for id in [&self.0.from, &self.0.to] {
            let element = model
                .element(id)
                .ok_or_else(|| format!("unknown related Element {id}"))?;
            if !related
                .iter()
                .any(|existing: &ElementRef<'a>| existing.id() == element.id())
            {
                related.push(element);
            }
        }
        Ok(related)
    }
}
impl<'a> Relationship<'a> for DirectedRelationshipRef<'a> {
    fn related_elements(self, model: Model<'a>) -> Result<Vec<ElementRef<'a>>, String> {
        self.0.related_elements(model)
    }
}
impl<'a> DirectedRelationship<'a> for DirectedRelationshipRef<'a> {
    fn sources(self, model: Model<'a>) -> Result<Vec<ElementRef<'a>>, String> {
        model
            .element(&self.0.0.from)
            .map(|element| vec![element])
            .ok_or_else(|| "unknown source Element".into())
    }
    fn targets(self, model: Model<'a>) -> Result<Vec<ElementRef<'a>>, String> {
        model
            .element(&self.0.0.to)
            .map(|element| vec![element])
            .ok_or_else(|| "unknown target Element".into())
    }
}
impl<'a> CommentRef<'a> {
    pub fn body(self) -> Option<&'a str> {
        self.0.properties.get("body").and_then(Value::as_str)
    }
    pub fn annotated_elements(self, model: Model<'a>) -> Result<Vec<ElementRef<'a>>, String> {
        let mut annotated = HashSet::new();
        model
            .relationships
            .iter()
            .filter(|relation| {
                relation.kind == "uml.note_link"
                    && relation.to == self.0.id
                    && annotated.insert(relation.from.as_str())
            })
            .map(|relation| {
                model
                    .element(&relation.from)
                    .ok_or_else(|| format!("unknown annotated Element {}", relation.from))
            })
            .collect()
    }
}

impl VisibilityKind {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "public" => Self::Public,
            "private" => Self::Private,
            "protected" => Self::Protected,
            "package" => Self::Package,
            _ => return None,
        })
    }
}

pub fn relationship_metaclass(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "uml.association" | "uml.directed_association" => "Association",
        "uml.generalization" => "Generalization",
        "uml.dependency" | "depends_on" => "Dependency",
        "uml.usage" => "Usage",
        "uml.realization" => "Realization",
        "uml.interface_realization" => "InterfaceRealization",
        "uml.template_binding" => "TemplateBinding",
        _ => return None,
    })
}

pub fn object_metaclass(kind: &str) -> Option<&'static str> {
    if let Some(kind) = UmlType::from_kind(kind) {
        return kind.metaclass();
    }
    let mut name = String::new();
    for word in kind.strip_prefix("uml.")?.split('_') {
        let mut characters = word.chars();
        name.push(characters.next()?.to_ascii_uppercase());
        name.extend(characters);
    }
    super::metamodel::DIRECT_GENERALIZATIONS
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(name, _)| *name)
}

impl<'a> ElementRef<'a> {
    pub fn id(self) -> &'a str {
        match self {
            Self::Object(value) => &value.id,
            Self::Relationship(value) => &value.id,
        }
    }
    pub fn kind(self) -> &'a str {
        match self {
            Self::Object(value) => &value.kind,
            Self::Relationship(value) => &value.kind,
        }
    }
    pub fn metaclass(self) -> Option<&'static str> {
        match self {
            Self::Object(value) => object_metaclass(&value.kind),
            Self::Relationship(value) => relationship_metaclass(&value.kind),
        }
    }
    pub fn conforms_to(self, metaclass: &str) -> bool {
        self.metaclass()
            .is_some_and(|actual| super::metamodel::conforms_to(actual, metaclass))
    }
    pub fn relationship(self) -> Option<RelationshipRef<'a>> {
        match self {
            Self::Relationship(relation) if self.conforms_to("Relationship") => {
                Some(RelationshipRef(relation))
            }
            _ => None,
        }
    }
    pub fn directed_relationship(self) -> Option<DirectedRelationshipRef<'a>> {
        self.conforms_to("DirectedRelationship")
            .then(|| self.relationship().map(DirectedRelationshipRef))
            .flatten()
    }
    pub fn comment(self) -> Option<CommentRef<'a>> {
        match self {
            Self::Object(element) if self.conforms_to("Comment") => Some(CommentRef(element)),
            _ => None,
        }
    }
    pub fn named(self) -> Option<NamedElementRef<'a>> {
        self.conforms_to("NamedElement")
            .then_some(NamedElementRef(self))
    }
    pub fn namespace(self) -> Option<NamespaceRef<'a>> {
        self.conforms_to("Namespace")
            .then_some(NamespaceRef(NamedElementRef(self)))
    }
    pub fn properties(self) -> &'a std::collections::BTreeMap<String, Value> {
        match self {
            Self::Object(value) => &value.properties,
            Self::Relationship(value) => &value.properties,
        }
    }
    pub fn display_label(self) -> String {
        match self {
            Self::Object(value) => crate::element_display_label(value),
            Self::Relationship(value) => self
                .named()
                .and_then(NamedElementRef::name)
                .map(str::to_owned)
                .unwrap_or_else(|| format!("{} → {} ({})", value.from, value.to, value.kind)),
        }
    }
}

impl<'a> NamedElementRef<'a> {
    pub fn element(self) -> ElementRef<'a> {
        self.0
    }
    /// Empty editor text represents an absent UML name, not an invented default name.
    pub fn name(self) -> Option<&'a str> {
        let name = match self.0 {
            ElementRef::Object(value) => Some(value.name.as_str()),
            ElementRef::Relationship(value) => value.properties.get("name").and_then(Value::as_str),
        };
        name.filter(|name| !name.is_empty())
    }
    pub fn visibility(self) -> Option<VisibilityKind> {
        match self.0.properties().get("visibility") {
            None => self.0.metaclass()
                .and_then(|metaclass| super::schema::uml_property(metaclass, "visibility").ok().flatten())
                .and_then(|property| super::schema::effective_default(property).ok().flatten())
                .and_then(|default| match default {
                    super::schema::DefaultValue::Enumeration { name, .. } => VisibilityKind::from_name(name),
                    _ => None,
                }),
            value => value
                .and_then(Value::as_str)
                .and_then(VisibilityKind::from_name),
        }
    }
}

impl<'a> NamespaceRef<'a> {
    pub fn named(self) -> NamedElementRef<'a> {
        self.0
    }
    pub fn element(self) -> ElementRef<'a> {
        self.0.element()
    }
}

/// Queries reference canonical records; this view neither copies nor owns a model.
#[derive(Clone, Copy)]
pub struct Model<'a> {
    objects: &'a [ElementRecord],
    relationships: &'a [Relation],
    root_model: Option<&'a str>,
}

impl<'a> Model<'a> {
    pub fn from_project(project: &'a Project) -> Self {
        Self {
            objects: &project.elements,
            relationships: &project.relations,
            root_model: project.root_model.as_deref(),
        }
    }
    pub fn from_section(section: &'a ModelingSection) -> Self {
        Self {
            objects: &section.elements,
            relationships: &section.relations,
            root_model: section.root_model.as_deref(),
        }
    }
    pub fn elements(self) -> impl Iterator<Item = ElementRef<'a>> {
        self.objects.iter().map(ElementRef::Object).chain(
            self.relationships
                .iter()
                .filter(|relation| relationship_metaclass(&relation.kind).is_some())
                .map(ElementRef::Relationship),
        )
    }
    pub fn element(self, id: &str) -> Option<ElementRef<'a>> {
        self.elements().find(|element| element.id() == id)
    }
    pub fn root(self) -> Result<Option<NamespaceRef<'a>>, String> {
        self.root_model
            .map(|id| {
                self.element(id)
                    .filter(|element| element.conforms_to("Model"))
                    .and_then(ElementRef::namespace)
                    .ok_or_else(|| format!("root_model {id} must identify a UML Model"))
            })
            .transpose()
    }
    pub fn owner(self, element: impl Element<'a>) -> Result<Option<ElementRef<'a>>, String> {
        let element = element.as_element();
        let mut owners = self
            .relationships
            .iter()
            .filter(|relation| relation.kind == "owns" && relation.to == element.id())
            .map(|relation| relation.from.as_str())
            .collect::<HashSet<_>>();
        // The specialized association ends subset Element::owner in UML.xmi.
        if let ElementRef::Relationship(relation) = element
            && matches!(
                relation.kind.as_str(),
                "uml.generalization" | "uml.interface_realization" | "uml.template_binding"
            )
        {
            owners.insert(&relation.from);
        }
        if owners.len() > 1 {
            return Err(format!("{} has more than one UML owner", element.id()));
        }
        owners
            .into_iter()
            .next()
            .map(|id| {
                self.element(id)
                    .ok_or_else(|| format!("{} has unknown UML owner {id}", element.id()))
            })
            .transpose()
    }
    pub fn owned_elements(self, owner: impl Element<'a>) -> Result<Vec<ElementRef<'a>>, String> {
        let owner = owner.as_element();
        let mut owned = Vec::new();
        for element in self.elements() {
            if self
                .owner(element)?
                .is_some_and(|actual| actual.id() == owner.id())
            {
                owned.push(element);
            }
        }
        Ok(owned)
    }
    pub fn namespace(
        self,
        named: impl NamedElement<'a>,
    ) -> Result<Option<NamespaceRef<'a>>, String> {
        Ok(self
            .owner(named.as_element())?
            .and_then(ElementRef::namespace))
    }
    pub fn qualified_name(self, named: impl NamedElement<'a>) -> Result<Option<String>, String> {
        let Some(name) = named.name() else {
            return Ok(None);
        };
        let mut names = vec![name];
        let mut current = named.as_element();
        let mut visited = HashSet::from([current.id()]);
        while let Some(mut owner) = self.owner(current)? {
            // NamedElement::allNamespaces handles exposed template parameters specially.
            if owner.conforms_to("TemplateParameter") {
                let Some(signature) = self.owner(owner)? else {
                    break;
                };
                let Some(template) = self.owner(signature)? else {
                    break;
                };
                owner = template;
            }
            let Some(namespace) = owner.namespace() else {
                break;
            };
            if !visited.insert(owner.id()) {
                return Err("cycle in UML ownership".into());
            }
            let Some(name) = namespace.named().name() else {
                return Ok(None);
            };
            names.push(name);
            current = owner;
        }
        names.reverse();
        Ok(Some(names.join("::")))
    }
    pub fn validate(self) -> Vec<StructuralDiagnostic> {
        let mut issues = Vec::new();
        let mut error = |code, id: &str, message: String| {
            issues.push(StructuralDiagnostic {
                severity: StructuralSeverity::Error,
                code,
                subject_id: id.into(),
                message,
            })
        };
        if let Err(message) = self.root() {
            error("UML_ROOT_MODEL", self.root_model.unwrap_or(""), message);
        }
        for element in self
            .elements()
            .filter(|element| element.metaclass().is_some())
        {
            let owner = match self.owner(element) {
                Ok(owner) => owner,
                Err(message) => {
                    error("UML_ELEMENT_OWNER", element.id(), message);
                    continue;
                }
            };
            if owner.is_none() && !element.conforms_to("Package") {
                error(
                    "UML_ELEMENT_HAS_OWNER",
                    element.id(),
                    "Element::mustBeOwned requires a UML owner".into(),
                );
            }
            if self.root_model == Some(element.id()) && owner.is_some() {
                error(
                    "UML_ROOT_MODEL",
                    element.id(),
                    "The designated root Model must not have an owner".into(),
                );
            }
            let mut current = owner;
            let mut visited = HashSet::from([element.id()]);
            while let Some(parent) = current {
                if !visited.insert(parent.id()) {
                    error(
                        "UML_ELEMENT_NOT_OWN_SELF",
                        element.id(),
                        "An Element cannot directly or indirectly own itself".into(),
                    );
                    break;
                }
                current = match self.owner(parent) {
                    Ok(owner) => owner,
                    Err(message) => {
                        error("UML_ELEMENT_OWNER", parent.id(), message);
                        break;
                    }
                };
            }
            if let Some(named) = element.named() {
                if element.properties().get("visibility").is_some_and(|value| {
                    !value.is_null() && value.as_str().and_then(VisibilityKind::from_name).is_none()
                }) {
                    error(
                        "UML_NAMED_ELEMENT_VISIBILITY",
                        element.id(),
                        "visibility must be a VisibilityKind or absent".into(),
                    );
                }
                if named.visibility().is_some()
                    && owner.is_some_and(|owner| {
                        owner.metaclass().is_some() && !owner.conforms_to("Namespace")
                    })
                {
                    error(
                        "UML_VISIBILITY_NEEDS_OWNERSHIP",
                        element.id(),
                        "A NamedElement owned by a non-Namespace cannot have visibility".into(),
                    );
                }
                if element.conforms_to("PackageableElement")
                    && owner.is_some_and(|owner| owner.conforms_to("Namespace"))
                    && named.visibility().is_none()
                {
                    error(
                        "UML_NAMESPACE_NEEDS_VISIBILITY",
                        element.id(),
                        "A PackageableElement in a Namespace must have visibility".into(),
                    );
                }
                if owner.is_some_and(|owner| owner.conforms_to("Package"))
                    && element.conforms_to("PackageableElement")
                    && !matches!(
                        named.visibility(),
                        Some(VisibilityKind::Public | VisibilityKind::Private)
                    )
                {
                    error(
                        "UML_PACKAGE_MEMBER_VISIBILITY",
                        element.id(),
                        "Package members must have public or private visibility".into(),
                    );
                }
                if let Some(namespace) = owner.and_then(ElementRef::namespace) {
                    // BehavioralFeature defines signature-based distinguishability; Parameters are a later slice.
                    if !element.conforms_to("BehavioralFeature")
                        && let Some(name) = named.name()
                    {
                        let members = match self.owned_elements(namespace.element()) {
                            Ok(members) => members,
                            Err(message) => {
                                error("UML_NAMESPACE_MEMBERS", namespace.id(), message);
                                continue;
                            }
                        };
                        let duplicate = members.into_iter().any(|other| {
                            other.id() != element.id()
                                && other.named().and_then(NamedElementRef::name) == Some(name)
                                && (element
                                    .metaclass()
                                    .is_some_and(|metaclass| other.conforms_to(metaclass))
                                    || other
                                        .metaclass()
                                        .is_some_and(|metaclass| element.conforms_to(metaclass)))
                        });
                        if duplicate {
                            error(
                                "UML_NAMESPACE_MEMBERS_DISTINGUISHABLE",
                                element.id(),
                                format!(
                                    "Namespace members of compatible metaclasses cannot share the name {name}"
                                ),
                            );
                        }
                    }
                }
            }
        }
        for element in self.elements() {
            issues.extend(super::schema::validate_element(element));
        }
        issues
    }
}

pub(crate) fn validate_changes(before: &Project, after: &Project) -> Result<(), String> {
    let existing = Model::from_project(before).validate();
    for issue in Model::from_project(after).validate() {
        if !existing.iter().any(|previous| {
            previous.code == issue.code
                && previous.subject_id == issue.subject_id
                && previous.message == issue.message
        }) {
            return Err(format!("{}: {}", issue.code, issue.message));
        }
    }
    Ok(())
}

fn fresh_id(section: &ModelingSection, project_id: &str, prefix: &str) -> Result<String, String> {
    for sequence in 1_u64.. {
        let id = format!("{prefix}-{sequence}");
        if id != project_id
            && !section.elements.iter().any(|element| element.id == id)
            && !section.relations.iter().any(|relation| relation.id == id)
            && !section
                .diagrams
                .iter()
                .any(|diagram| diagram.id == id || diagram.views.iter().any(|view| view.id == id))
        {
            return Ok(id);
        }
    }
    Err("identifier overflow".into())
}

fn ownership_property(owner: ElementRef<'_>, child: ElementRef<'_>) -> Option<&'static str> {
    let has = |property| {
        owner
            .metaclass()
            .is_some_and(|metaclass| super::metamodel::has_property(metaclass, property))
    };
    if child.conforms_to("Comment") {
        return Some("ownedComment");
    }
    if owner.conforms_to("Package") && child.conforms_to("PackageableElement") {
        return Some("packagedElement");
    }
    for (metaclass, property) in [
        ("Classifier", "nestedClassifier"),
        ("Operation", "ownedOperation"),
        ("Reception", "ownedReception"),
        ("Property", "ownedAttribute"),
        ("Property", "ownedEnd"),
        ("Constraint", "ownedRule"),
        ("TemplateSignature", "ownedTemplateSignature"),
    ] {
        if child.conforms_to(metaclass) && has(property) {
            return Some(property);
        }
    }
    if owner.conforms_to("TemplateSignature") && child.conforms_to("TemplateParameter") {
        return Some("ownedParameter");
    }
    if (owner.conforms_to("Behavior") || owner.conforms_to("Operation"))
        && child.conforms_to("Parameter")
    {
        return Some("ownedParameter");
    }
    if owner.conforms_to("TemplateParameter") && child.conforms_to("ParameterableElement") {
        return Some("ownedParameteredElement");
    }
    None
}

pub(crate) fn set_owner(
    section: &mut ModelingSection,
    project_id: &str,
    element_id: &str,
    owner_id: &str,
) -> Result<(), String> {
    let model = Model::from_section(section);
    let element = model.element(element_id).ok_or("UML element not found")?;
    let owner = model.element(owner_id).ok_or("UML owner not found")?;
    if owner.metaclass().is_none() {
        return Err("UML ownership requires a supported owner metaclass".into());
    }
    if let ElementRef::Relationship(relation) = element
        && matches!(
            relation.kind.as_str(),
            "uml.generalization" | "uml.interface_realization" | "uml.template_binding"
        )
    {
        return if relation.from == owner_id {
            Ok(())
        } else {
            Err("The relationship's specialized source determines its owner".into())
        };
    }
    if ownership_property(owner, element).is_none() {
        return Err(
            "This owner/element pair has no supported UML composite ownership property".into(),
        );
    }
    if let Some(previous) = model.owner(element)? {
        return if previous.id() == owner_id {
            Ok(())
        } else {
            Err("Element already has a semantic owner; explicit reparenting is required".into())
        };
    }
    let id = fresh_id(section, project_id, "mde-owns")?;
    section.relations.push(Relation {
        id,
        kind: "owns".into(),
        from: owner_id.into(),
        to: element_id.into(),
        properties: Default::default(),
    });
    Ok(())
}

/// Designates an actual Model and adopts only unowned packageable elements/comments.
/// Existing names, IDs, ownership, views and extension records are never replaced.
pub(crate) fn adopt_root_model(
    section: &mut ModelingSection,
    project_id: &str,
    project_name: &str,
) -> Result<String, String> {
    for element in &mut section.elements {
        if object_metaclass(&element.kind) == Some("Comment")
            && !element.properties.contains_key("body")
        {
            // Earlier notes rendered name, not description; preserve that authored text and metadata.
            element
                .properties
                .insert("body".into(), Value::String(element.name.clone()));
        }
    }
    if let Some(root) = Model::from_section(section).root()? {
        return Ok(root.element().id().into());
    }
    let models = section
        .elements
        .iter()
        .filter(|element| {
            element.kind == "uml.model"
                && !section
                    .relations
                    .iter()
                    .any(|relation| relation.kind == "owns" && relation.to == element.id)
        })
        .map(|element| element.id.clone())
        .collect::<Vec<_>>();
    let root_id = if let [id] = models.as_slice() {
        id.clone()
    } else {
        let id = fresh_id(section, project_id, "mde-root-model")?;
        section.elements.push(ElementRecord {
            id: id.clone(),
            kind: "uml.model".into(),
            name: project_name.into(),
            label: None,
            description: None,
            properties: Default::default(),
            annotations: Vec::new(),
            implementations: Vec::new(),
            content: None,
        });
        id
    };
    section.root_model = Some(root_id.clone());
    let model = Model::from_section(section);
    let mut unowned = Vec::new();
    for element in model.elements().filter(|element| {
        element.id() != root_id
            && (element.conforms_to("PackageableElement") || element.conforms_to("Comment"))
    }) {
        if model.owner(element)?.is_none() {
            unowned.push(element.id().to_owned());
        }
    }
    for id in unowned {
        set_owner(section, project_id, &id, &root_id)?;
    }
    Ok(root_id)
}

pub(crate) fn reparent(
    section: &mut ModelingSection,
    project_id: &str,
    element_id: &str,
    owner_id: Option<&str>,
) -> Result<(), String> {
    let model = Model::from_section(section);
    let element = model.element(element_id).ok_or("UML element not found")?;
    if let ElementRef::Relationship(relation) = element
        && matches!(
            relation.kind.as_str(),
            "uml.generalization" | "uml.interface_realization" | "uml.template_binding"
        )
    {
        return Err(
            "Change the specialized relationship source instead of its derived owner".into(),
        );
    }
    if section.root_model.as_deref() == Some(element_id) && owner_id.is_some() {
        return Err("The root Model cannot acquire an owner".into());
    }
    if let Some(owner_id) = owner_id {
        let owner = model.element(owner_id).ok_or("UML owner not found")?;
        if owner.metaclass().is_none() {
            return Err("UML ownership requires a supported owner metaclass".into());
        }
        if ownership_property(owner, element).is_none() {
            return Err(
                "This owner/element pair has no supported UML composite ownership property".into(),
            );
        }
        if let Some(ownership) = section
            .relations
            .iter_mut()
            .find(|relation| relation.kind == "owns" && relation.to == element_id)
        {
            ownership.from = owner_id.into();
        } else {
            set_owner(section, project_id, element_id, owner_id)?;
        }
    } else {
        if !element.conforms_to("Package") {
            return Err("Only Package-conforming elements may be unowned in this UML slice".into());
        }
        section
            .relations
            .retain(|relation| relation.kind != "owns" || relation.to != element_id);
    }
    Ok(())
}
