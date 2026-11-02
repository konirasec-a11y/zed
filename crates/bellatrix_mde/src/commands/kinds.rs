use crate::ModelingRelationKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModelingElementKind {
    Class,
    Interface,
    Component,
    Artifact,
    Signal,
    DataType,
    PrimitiveType,
    Enumeration,
    Frame,
    NaryAssociation,
    AssociationClass,
    Package,
    Model,
    Subsystem,
    Comment,
    Template,
}
impl ModelingElementKind {
    pub const STRUCTURAL: &'static [Self] = &[Self::Component, Self::Artifact];
    pub fn requires_owner(self) -> bool {
        matches!(self, Self::Component | Self::Artifact)
    }
    pub const BASIC: &'static [Self] = &[Self::Class, Self::Interface];
    pub const ADVANCED: &'static [Self] = &[
        Self::Signal,
        Self::DataType,
        Self::PrimitiveType,
        Self::Enumeration,
        Self::Frame,
        Self::NaryAssociation,
        Self::AssociationClass,
        Self::Template,
    ];
    pub const PACKAGES: &'static [Self] = &[Self::Package, Self::Model, Self::Subsystem];
    pub const ANNOTATIONS: &'static [Self] = &[Self::Comment];
    pub fn label(self) -> &'static str {
        match self {
            Self::Component => "Component",
            Self::Artifact => "Artifact",
            Self::Class => "Class",
            Self::Interface => "Interface",
            Self::Signal => "Signal",
            Self::DataType => "DataType",
            Self::PrimitiveType => "PrimitiveType",
            Self::Enumeration => "Enumeration",
            Self::Frame => "Frame",
            Self::NaryAssociation => "N-ary Association Node",
            Self::AssociationClass => "Association Class",
            Self::Template => "Template",
            Self::Package => "Package",
            Self::Model => "Model",
            Self::Subsystem => "Subsystem",
            Self::Comment => "Comment",
        }
    }
    pub fn model_kind(self) -> &'static str {
        match self {
            // Existing wire identities and Rust generator profiles remain readable.
            Self::Component => "uml.component",
            Self::Artifact => "uml.artifact",
            Self::Class => "rust.struct",
            Self::Interface => "rust.trait",
            Self::Signal => "uml.signal",
            Self::DataType => "uml.data_type",
            Self::PrimitiveType => "uml.primitive_type",
            Self::Enumeration => "uml.enumeration",
            Self::Frame => "uml.frame",
            Self::NaryAssociation => "uml.nary_association",
            Self::AssociationClass => "uml.association_class",
            Self::Template => "uml.template",
            Self::Package => "uml.package",
            Self::Model => "uml.model",
            Self::Subsystem => "uml.subsystem",
            Self::Comment => "uml.comment",
        }
    }
    pub fn default_name(self) -> &'static str {
        match self {
            Self::Component => "NewComponent",
            Self::Artifact => "NewArtifact",
            Self::Class => "NewClass",
            Self::Interface => "NewInterface",
            Self::Enumeration => "NewEnumeration",
            Self::Signal => "NewSignal",
            Self::DataType => "NewDataType",
            Self::PrimitiveType => "NewPrimitiveType",
            Self::Frame => "NewFrame",
            Self::NaryAssociation => "NewAssociation",
            Self::AssociationClass => "NewAssociationClass",
            Self::Template => "NewTemplate",
            Self::Package => "NewPackage",
            Self::Model => "NewModel",
            Self::Subsystem => "NewSubsystem",
            Self::Comment => "NewComment",
        }
    }
    pub fn initial_size(self) -> (f64, f64) {
        match self {
            Self::Interface => (120.0, 76.0),
            Self::Frame => (280.0, 205.0),
            Self::Package | Self::Model | Self::Subsystem => (190.0, 125.0),
            Self::Comment => (180.0, 92.0),
            Self::NaryAssociation => (80.0, 80.0),
            _ => (185.0, 112.0),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModelingTool {
    Element(ModelingElementKind),
    Relation(ModelingRelationKind),
}
impl ModelingTool {
    pub fn label(self) -> &'static str {
        match self {
            Self::Element(kind) => kind.label(),
            Self::Relation(kind) => kind.label(),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_class_tool_has_a_unique_semantic_kind() {
        let mut kinds = std::collections::HashSet::new();
        for kind in ModelingElementKind::BASIC
            .iter()
            .chain(ModelingElementKind::ADVANCED)
            .chain(ModelingElementKind::STRUCTURAL)
            .chain(ModelingElementKind::PACKAGES)
            .chain(ModelingElementKind::ANNOTATIONS)
        {
            assert!(kinds.insert(kind.model_kind()));
        }
        for kind in ModelingRelationKind::BASIC
            .iter()
            .chain(ModelingRelationKind::ADVANCED)
            .chain(ModelingRelationKind::PACKAGES)
        {
            assert!(kinds.insert(kind.model_kind()));
        }
        assert_eq!(kinds.len(), 26);
    }
    #[test]
    fn generalization_and_realization_validate_targets() {
        assert!(
            ModelingRelationKind::Generalization
                .validate_endpoints("rust.struct", "rust.trait")
                .is_ok()
        );
        assert!(
            ModelingRelationKind::InterfaceRealization
                .validate_endpoints("rust.struct", "uml.package")
                .is_err()
        );
        assert!(
            ModelingRelationKind::Containment
                .validate_endpoints("rust.struct", "rust.trait")
                .is_err()
        );
    }
}
