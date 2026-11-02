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
    Reception,
    TemplateParameter,
    Port,
    Part,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UmlCapability {
    Annotation,
    Attribute,
    Operation,
    Reception,
    Specialization,
    RealizationTarget,
    TemplateParameter,
    ClassInterfaces,
    ClassAssociation,
    Port,
    Part,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UmlQuickCreate {
    Note,
    Constraint,
    Reception,
    Subtype,
    Supertype,
    RealizingClass,
    TemplateParameter,
    ProvidedInterface,
    RequiredInterface,
    AssociatedClass,
    AggregatedClass,
    CompositedClass,
    Port,
    Part,
}

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
            "uml.template_parameter" => Self::TemplateParameter,
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
            Self::Reception => "Reception",
            Self::TemplateParameter => "TemplateParameter",
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

    pub fn supports(self, capability: UmlCapability) -> bool {
        match capability {
            // Profile registration enables notation only; command support must
            // be introduced deliberately with its ownership/application adapter.
            UmlCapability::Annotation => !matches!(self, Self::Frame | Self::Profile),
            // Preserve the editor's supported commands independently of the
            // broader semantic Classifier hierarchy used by validation.
            UmlCapability::Attribute | UmlCapability::Operation => matches!(
                self,
                Self::Class
                    | Self::Interface
                    | Self::Signal
                    | Self::DataType
                    | Self::PrimitiveType
                    | Self::Enumeration
                    | Self::AssociationClass
                    | Self::Template
            ),
            UmlCapability::Reception => {
                matches!(self, Self::Class | Self::Interface | Self::AssociationClass)
            }
            UmlCapability::Specialization => matches!(self, Self::Class | Self::Interface),
            UmlCapability::RealizationTarget => self == Self::Interface,
            UmlCapability::TemplateParameter
            | UmlCapability::ClassInterfaces
            | UmlCapability::ClassAssociation
            | UmlCapability::Port
            | UmlCapability::Part => self == Self::Class,
        }
    }
}

impl UmlQuickCreate {
    pub fn capability(self) -> UmlCapability {
        match self {
            Self::Note | Self::Constraint => UmlCapability::Annotation,
            Self::Reception => UmlCapability::Reception,
            Self::Subtype | Self::Supertype => UmlCapability::Specialization,
            Self::RealizingClass => UmlCapability::RealizationTarget,
            Self::TemplateParameter => UmlCapability::TemplateParameter,
            Self::ProvidedInterface | Self::RequiredInterface => UmlCapability::ClassInterfaces,
            Self::AssociatedClass | Self::AggregatedClass | Self::CompositedClass => {
                UmlCapability::ClassAssociation
            }
            Self::Port => UmlCapability::Port,
            Self::Part => UmlCapability::Part,
        }
    }

    pub fn accepts(self, kind: &str) -> bool {
        UmlType::from_kind(kind).is_some_and(|kind| kind.supports(self.capability()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_and_interface_share_capabilities_not_symbols() {
        for capability in [
            UmlCapability::Annotation,
            UmlCapability::Attribute,
            UmlCapability::Operation,
            UmlCapability::Reception,
            UmlCapability::Specialization,
        ] {
            assert!(UmlType::Class.supports(capability));
            assert!(UmlType::Interface.supports(capability));
        }
        assert!(!UmlType::Class.supports(UmlCapability::RealizationTarget));
        assert!(UmlType::Interface.supports(UmlCapability::RealizationTarget));
        assert!(UmlType::Package.supports(UmlCapability::Annotation));
        assert!(!UmlType::Package.supports(UmlCapability::Operation));
        assert!(!UmlQuickCreate::Note.accepts("fs.file"));
    }

    #[test]
    fn wire_aliases_do_not_change_semantics() {
        assert_eq!(
            UmlType::from_kind("rust.trait"),
            UmlType::from_kind("uml.interface")
        );
        assert_eq!(
            UmlType::from_kind("rust.struct"),
            UmlType::from_kind("uml.class")
        );
        assert_eq!(
            UmlType::from_kind("uml.note"),
            UmlType::from_kind("uml.comment")
        );
    }

    #[test]
    fn semantic_classifiers_do_not_enable_unsupported_editor_commands() {
        for kind in [
            UmlType::Component,
            UmlType::Node,
            UmlType::Device,
            UmlType::ExecutionEnvironment,
            UmlType::Artifact,
            UmlType::Stereotype,
            UmlType::Collaboration,
            UmlType::NaryAssociation,
        ] {
            assert!(kind.is_classifier());
            assert!(!kind.supports(UmlCapability::Attribute));
            assert!(!kind.supports(UmlCapability::Operation));
        }
        assert_eq!(UmlType::Template.may_specialize_type(UmlType::Class), None);
        assert_eq!(UmlType::Class.may_specialize_type(UmlType::Template), None);
        assert_eq!(UmlType::Package.may_specialize_type(UmlType::Class), None);
    }

    #[test]
    fn bellatrix_kind_mappings_are_consistent_with_the_official_catalog() {
        use crate::uml::metamodel::{conforms_to, is_known_metaclass};

        for kind in [
            UmlType::Class,
            UmlType::Interface,
            UmlType::Signal,
            UmlType::DataType,
            UmlType::PrimitiveType,
            UmlType::Enumeration,
            UmlType::AssociationClass,
            UmlType::Component,
            UmlType::Node,
            UmlType::Device,
            UmlType::ExecutionEnvironment,
            UmlType::Artifact,
            UmlType::Stereotype,
            UmlType::Collaboration,
            UmlType::Package,
            UmlType::Profile,
            UmlType::Model,
            UmlType::Note,
            UmlType::Constraint,
            UmlType::NaryAssociation,
            UmlType::Attribute,
            UmlType::Operation,
            UmlType::Reception,
            UmlType::TemplateParameter,
            UmlType::Port,
            UmlType::Part,
        ] {
            let Some(name) = kind.metaclass() else {
                panic!("Expected a normative metaclass for {kind:?}");
            };
            assert!(
                is_known_metaclass(name),
                "Unknown metaclass for {kind:?}: {name}"
            );
            assert_eq!(kind.is_classifier(), conforms_to(name, "Classifier"));
            assert_eq!(
                kind.is_structured_classifier(),
                conforms_to(name, "StructuredClassifier")
            );
            assert_eq!(
                kind.may_specialize_type(kind),
                kind.is_classifier().then_some(true)
            );
        }

        for kind in [UmlType::Template, UmlType::Subsystem, UmlType::Frame] {
            assert_eq!(kind.metaclass(), None);
            assert_eq!(kind.may_specialize_type(kind), None);
            assert_eq!(kind.may_specialize_type(UmlType::Class), None);
            assert_eq!(UmlType::Class.may_specialize_type(kind), None);
            assert!(!kind.is_structured_classifier());
            assert!(!kind.is_classifier());
        }
        assert_eq!(UmlType::from_kind("uml.unknown"), None);
        assert!(!UmlQuickCreate::Subtype.accepts("uml.unknown"));
    }

    #[test]
    fn profile_is_a_package_not_a_classifier_or_a_new_command_surface() {
        assert_eq!(UmlType::from_kind("uml.profile"), Some(UmlType::Profile));
        assert_eq!(UmlType::Profile.metaclass(), Some("Profile"));
        assert!(crate::uml::metamodel::conforms_to("Profile", "Package"));
        assert!(!UmlType::Profile.is_classifier());
        for capability in [
            UmlCapability::Annotation,
            UmlCapability::Attribute,
            UmlCapability::Operation,
            UmlCapability::Reception,
            UmlCapability::Specialization,
            UmlCapability::RealizationTarget,
            UmlCapability::TemplateParameter,
            UmlCapability::ClassInterfaces,
            UmlCapability::ClassAssociation,
            UmlCapability::Port,
            UmlCapability::Part,
        ] {
            assert!(!UmlType::Profile.supports(capability));
        }
    }
}
