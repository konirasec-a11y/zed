//! Semantic capabilities are independent of notation and of the persisted kind spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UmlType {
    Class,
    Interface,
    Signal,
    DataType,
    PrimitiveType,
    Enumeration,
    AssociationClass,
    Component,
    Node,
    Device,
    ExecutionEnvironment,
    Artifact,
    Stereotype,
    Collaboration,
    Template,
    Package,
    Profile,
    Model,
    Subsystem,
    Note,
    Constraint,
    Frame,
    NaryAssociation,
    Attribute,
    Operation,
    Parameter,
    Reception,
    TemplateParameter,
    ClassifierTemplateParameter,
    TemplateSignature,
    RedefinableTemplateSignature,
    Port,
    Part,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UmlCapability {
    Annotation,
    Attribute,
    Operation,
    Parameter,
    Reception,
    Specialization,
    RealizationTarget,
    TemplateParameter,
    ClassInterfaces,
    ClassAssociation,
    Port,
    Part,
}

pub use crate::contracts::protocol::command::UmlQuickCreate;

impl UmlType {
    pub fn from_kind(kind: &str) -> Option<Self> {
        Some(match kind {
            "rust.struct" | "uml.class" => Self::Class,
            "rust.trait" | "uml.interface" => Self::Interface,
            "uml.signal" => Self::Signal,
            "uml.data_type" => Self::DataType,
            "uml.primitive_type" => Self::PrimitiveType,
            "uml.enumeration" => Self::Enumeration,
            "uml.association_class" => Self::AssociationClass,
            "uml.component" => Self::Component,
            "uml.node" => Self::Node,
            "uml.device" => Self::Device,
            "uml.execution_environment" => Self::ExecutionEnvironment,
            "uml.artifact" => Self::Artifact,
            "uml.stereotype" => Self::Stereotype,
            "uml.collaboration" => Self::Collaboration,
            "uml.template" => Self::Template,
            "uml.package" => Self::Package,
            "uml.profile" => Self::Profile,
            "uml.model" => Self::Model,
            "uml.subsystem" => Self::Subsystem,
            "uml.comment" | "uml.note" => Self::Note,
            "uml.constraint" => Self::Constraint,
            "uml.frame" => Self::Frame,
            "uml.nary_association" => Self::NaryAssociation,
            "rust.field" | "uml.attribute" => Self::Attribute,
            "rust.function" | "uml.operation" => Self::Operation,
            "uml.reception" => Self::Reception,
            "uml.parameter" => Self::Parameter,
            "uml.template_parameter" => Self::TemplateParameter,
            "uml.classifier_template_parameter" => Self::ClassifierTemplateParameter,
            "uml.template_signature" => Self::TemplateSignature,
            "uml.redefinable_template_signature" => Self::RedefinableTemplateSignature,
            "uml.port" => Self::Port,
            "uml.part" => Self::Part,
            _ => return None,
        })
    }

    /// Persisted compatibility names map to normative UML metaclasses here.
    /// Presentation-only concepts and unresolved legacy aliases have no mapping.
    pub fn metaclass(self) -> Option<&'static str> {
        Some(match self {
            Self::Class => "Class",
            Self::Interface => "Interface",
            Self::Signal => "Signal",
            Self::DataType => "DataType",
            Self::PrimitiveType => "PrimitiveType",
            Self::Enumeration => "Enumeration",
            Self::AssociationClass => "AssociationClass",
            Self::Component => "Component",
            Self::Node => "Node",
            Self::Device => "Device",
            Self::ExecutionEnvironment => "ExecutionEnvironment",
            Self::Artifact => "Artifact",
            Self::Stereotype => "Stereotype",
            Self::Collaboration => "Collaboration",
            Self::Package => "Package",
            Self::Profile => "Profile",
            Self::Model => "Model",
            Self::Note => "Comment",
            Self::Constraint => "Constraint",
            Self::NaryAssociation => "Association",
            Self::Attribute | Self::Part => "Property",
            Self::Port => "Port",
            Self::Operation => "Operation",
            Self::Parameter => "Parameter",
            Self::Reception => "Reception",
            Self::TemplateParameter => "TemplateParameter",
            Self::ClassifierTemplateParameter => "ClassifierTemplateParameter",
            Self::TemplateSignature => "TemplateSignature",
            Self::RedefinableTemplateSignature => "RedefinableTemplateSignature",
            Self::Template | Self::Subsystem | Self::Frame => return None,
        })
    }

    pub fn is_classifier(self) -> bool {
        self.metaclass()
            .is_some_and(|name| crate::uml::metamodel::conforms_to(name, "Classifier"))
    }

    pub fn is_structured_classifier(self) -> bool {
        self.metaclass()
            .is_some_and(|name| crate::uml::metamodel::conforms_to(name, "StructuredClassifier"))
    }

    /// UML.xmi Classifier-maySpecializeType-spec uses metaclass conformance.
    pub fn may_specialize_type(self, general: Self) -> Option<bool> {
        if !self.is_classifier() || !general.is_classifier() {
            return None;
        }
        Some(crate::uml::metamodel::conforms_to(
            self.metaclass()?,
            general.metaclass()?,
        ))
    }

    pub fn has_property(self, property: &str) -> bool {
        self.metaclass()
            .is_some_and(|name| crate::uml::metamodel::has_property(name, property))
    }

    pub fn supports(self, capability: UmlCapability) -> bool {
        match capability {
            // Profile registration enables notation only; command support must
            // be introduced deliberately with its ownership/application adapter.
            UmlCapability::Annotation => !matches!(self, Self::Frame | Self::Profile),
            UmlCapability::Attribute => {
                self.has_property("ownedAttribute") || self == Self::Template
            }
            UmlCapability::Operation => {
                self.has_property("ownedOperation") || self == Self::Template
            }
            UmlCapability::Parameter => self == Self::Operation,
            UmlCapability::Reception => self.has_property("ownedReception"),
            UmlCapability::Port => self.has_property("ownedPort"),
            UmlCapability::Part => self.has_property("part"),
            UmlCapability::Specialization => self.has_property("generalization"),
            UmlCapability::RealizationTarget => self == Self::Interface,
            UmlCapability::ClassInterfaces => self.has_property("interfaceRealization"),
            UmlCapability::ClassAssociation => self
                .metaclass()
                .is_some_and(|name| crate::uml::metamodel::conforms_to(name, "Type")),
            UmlCapability::TemplateParameter => self.has_property("ownedTemplateSignature"),
        }
    }
}

pub fn quick_create_capability(action: UmlQuickCreate) -> UmlCapability {
    match action {
        UmlQuickCreate::Note | UmlQuickCreate::Constraint => UmlCapability::Annotation,
        UmlQuickCreate::Reception => UmlCapability::Reception,
        UmlQuickCreate::Subtype | UmlQuickCreate::Supertype => UmlCapability::Specialization,
        UmlQuickCreate::RealizingClass => UmlCapability::RealizationTarget,
        UmlQuickCreate::TemplateParameter => UmlCapability::TemplateParameter,
        UmlQuickCreate::ProvidedInterface | UmlQuickCreate::RequiredInterface => {
            UmlCapability::ClassInterfaces
        }
        UmlQuickCreate::AssociatedClass
        | UmlQuickCreate::AggregatedClass
        | UmlQuickCreate::CompositedClass => UmlCapability::ClassAssociation,
        UmlQuickCreate::Port => UmlCapability::Port,
        UmlQuickCreate::Part => UmlCapability::Part,
    }
}

pub fn quick_create_accepts(action: UmlQuickCreate, kind: &str) -> bool {
    UmlType::from_kind(kind).is_some_and(|kind| kind.supports(quick_create_capability(action)))
}
