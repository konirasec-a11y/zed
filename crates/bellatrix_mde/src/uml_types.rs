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
pub enum UmlQuickCommand {
    AddNote,
    AddConstraint,
    AddReception,
    AddSubtype,
    AddSupertype,
    AddRealizingClass,
    AddTemplateParameter,
    AddProvidedInterface,
    AddRequiredInterface,
    AddAssociatedClass,
    AddAggregatedClass,
    AddCompositedClass,
    AddPort,
    AddPart,
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
        // Preserve the legacy Template capability without inventing its UML type.
        self == Self::Template
            || self
                .metaclass()
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
            UmlCapability::Annotation => !matches!(self, Self::Frame),
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

impl UmlQuickCommand {
    pub fn capability(self) -> UmlCapability {
        match self {
            Self::AddNote | Self::AddConstraint => UmlCapability::Annotation,
            Self::AddReception => UmlCapability::Reception,
            Self::AddSubtype | Self::AddSupertype => UmlCapability::Specialization,
            Self::AddRealizingClass => UmlCapability::RealizationTarget,
            Self::AddTemplateParameter => UmlCapability::TemplateParameter,
            Self::AddProvidedInterface | Self::AddRequiredInterface => {
                UmlCapability::ClassInterfaces
            }
            Self::AddAssociatedClass | Self::AddAggregatedClass | Self::AddCompositedClass => {
                UmlCapability::ClassAssociation
            }
            Self::AddPort => UmlCapability::Port,
            Self::AddPart => UmlCapability::Part,
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
        assert!(!UmlQuickCommand::AddNote.accepts("fs.file"));
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
            assert_eq!(kind.is_classifier(), kind == UmlType::Template);
        }
        assert_eq!(UmlType::from_kind("uml.unknown"), None);
        assert!(!UmlQuickCommand::AddSubtype.accepts("uml.unknown"));
    }
}
