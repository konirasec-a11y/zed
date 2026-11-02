//! Semantic capabilities are independent of notation and of the persisted kind spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UmlType {
    Class, Interface, Signal, DataType, PrimitiveType, Enumeration,
    AssociationClass, Template, Package, Model, Subsystem,
    Note, Constraint, Frame, NaryAssociation, Attribute, Operation, Reception,
    TemplateParameter, Port, Part,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UmlCapability {
    Annotation, Attribute, Operation, Reception, Specialization, RealizationTarget,
    TemplateParameter, ClassInterfaces, ClassAssociation, Port, Part,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UmlQuickCommand {
    AddNote, AddConstraint, AddReception, AddSubtype, AddSupertype, AddRealizingClass,
    AddTemplateParameter, AddProvidedInterface, AddRequiredInterface,
    AddAssociatedClass, AddAggregatedClass, AddCompositedClass, AddPort, AddPart,
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

    pub fn is_classifier(self) -> bool {
        matches!(self, Self::Class | Self::Interface | Self::Signal | Self::DataType
            | Self::PrimitiveType | Self::Enumeration | Self::AssociationClass | Self::Template)
    }

    pub fn supports(self, capability: UmlCapability) -> bool {
        match capability {
            UmlCapability::Annotation => !matches!(self, Self::Frame),
            UmlCapability::Attribute | UmlCapability::Operation => self.is_classifier(),
            UmlCapability::Reception => matches!(self, Self::Class | Self::Interface | Self::AssociationClass),
            UmlCapability::Specialization => matches!(self, Self::Class | Self::Interface),
            UmlCapability::RealizationTarget => self == Self::Interface,
            UmlCapability::TemplateParameter | UmlCapability::ClassInterfaces |
            UmlCapability::ClassAssociation | UmlCapability::Port | UmlCapability::Part => self == Self::Class,
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
            Self::AddProvidedInterface | Self::AddRequiredInterface => UmlCapability::ClassInterfaces,
            Self::AddAssociatedClass | Self::AddAggregatedClass | Self::AddCompositedClass => UmlCapability::ClassAssociation,
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
        for capability in [UmlCapability::Annotation, UmlCapability::Attribute,
            UmlCapability::Operation, UmlCapability::Reception, UmlCapability::Specialization] {
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
        assert_eq!(UmlType::from_kind("rust.trait"), UmlType::from_kind("uml.interface"));
        assert_eq!(UmlType::from_kind("rust.struct"), UmlType::from_kind("uml.class"));
        assert_eq!(UmlType::from_kind("uml.note"), UmlType::from_kind("uml.comment"));
    }
}
