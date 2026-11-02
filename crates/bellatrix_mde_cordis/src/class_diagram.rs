//! UML Class Diagram vocabulary. These are domain tool types; no GPUI or Zed dependency.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModelingElementKind {
    Class, Interface, Signal, DataType, PrimitiveType, Enumeration, Frame,
    NaryAssociation, AssociationClass, Package, Model, Subsystem, Comment, Template,
}
impl ModelingElementKind {
    pub const BASIC: &'static [Self] = &[Self::Class, Self::Interface];
    pub const ADVANCED: &'static [Self] = &[
        Self::Signal, Self::DataType, Self::PrimitiveType, Self::Enumeration,
        Self::Frame, Self::NaryAssociation, Self::AssociationClass, Self::Template,
    ];
    pub const PACKAGES: &'static [Self] = &[Self::Package, Self::Model, Self::Subsystem];
    pub const ANNOTATIONS: &'static [Self] = &[Self::Comment];
    pub fn label(self) -> &'static str {
        match self {
            Self::Class => "Class", Self::Interface => "Interface",
            Self::Signal => "Signal", Self::DataType => "DataType",
            Self::PrimitiveType => "PrimitiveType", Self::Enumeration => "Enumeration",
            Self::Frame => "Frame", Self::NaryAssociation => "N-ary Association Node",
            Self::AssociationClass => "Association Class", Self::Template => "Template",
            Self::Package => "Package", Self::Model => "Model", Self::Subsystem => "Subsystem",
            Self::Comment => "Comment",
        }
    }
    pub fn model_kind(self) -> &'static str {
        match self {
            // Preserve existing Rust/Cargo fixtures and generator compatibility.
            Self::Class => "rust.struct", Self::Interface => "rust.trait",
            Self::Signal => "uml.signal", Self::DataType => "uml.data_type",
            Self::PrimitiveType => "uml.primitive_type",
            Self::Enumeration => "uml.enumeration", Self::Frame => "uml.frame",
            Self::NaryAssociation => "uml.nary_association",
            Self::AssociationClass => "uml.association_class",
            Self::Template => "uml.template", Self::Package => "uml.package",
            Self::Model => "uml.model", Self::Subsystem => "uml.subsystem",
            Self::Comment => "uml.comment",
        }
    }
    pub fn default_name(self) -> &'static str {
        match self {
            Self::Class => "NewClass", Self::Interface => "NewInterface",
            Self::Enumeration => "NewEnumeration", Self::Signal => "NewSignal",
            Self::DataType => "NewDataType", Self::PrimitiveType => "NewPrimitiveType",
            Self::Frame => "NewFrame", Self::NaryAssociation => "NewAssociation",
            Self::AssociationClass => "NewAssociationClass",
            Self::Template => "NewTemplate", Self::Package => "NewPackage",
            Self::Model => "NewModel", Self::Subsystem => "NewSubsystem",
            Self::Comment => "NewComment",
        }
    }
    pub fn initial_size(self) -> (f64, f64) {
        match self {
            Self::Frame => (280.0, 205.0),
            Self::Package | Self::Model | Self::Subsystem => (190.0, 125.0),
            Self::Comment => (180.0, 92.0),
            Self::NaryAssociation => (80.0, 80.0),
            _ => (185.0, 112.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModelingRelationKind {
    Association, DirectedAssociation, Aggregation, Composition, Dependency,
    Generalization, InterfaceRealization, TemplateBinding, Realization, Containment,
}
impl ModelingRelationKind {
    pub const BASIC: &'static [Self] = &[
        Self::Association, Self::DirectedAssociation, Self::Aggregation, Self::Composition,
        Self::Dependency, Self::Generalization, Self::InterfaceRealization,
    ];
    pub const ADVANCED: &'static [Self] = &[Self::TemplateBinding, Self::Realization];
    pub const PACKAGES: &'static [Self] = &[Self::Containment];
    pub fn label(self) -> &'static str {
        match self {
            Self::Association => "Association", Self::DirectedAssociation => "Directed Association",
            Self::Aggregation => "Aggregation", Self::Composition => "Composition",
            Self::Dependency => "Dependency", Self::Generalization => "Generalization",
            Self::InterfaceRealization => "Interface Realization",
            Self::TemplateBinding => "Template Binding", Self::Realization => "Realization",
            Self::Containment => "Containment",
        }
    }
    pub fn model_kind(self) -> &'static str {
        match self {
            Self::Association => "uml.association",
            Self::DirectedAssociation => "uml.directed_association",
            Self::Aggregation => "uml.aggregation",
            Self::Composition => "uml.composition",
            Self::Dependency => "uml.dependency",
            Self::Generalization => "uml.generalization",
            Self::InterfaceRealization => "uml.interface_realization",
            Self::TemplateBinding => "uml.template_binding",
            Self::Realization => "uml.realization",
            Self::Containment => "uml.containment",
        }
    }
    pub fn validate_endpoints(self, source: &str, target: &str) -> Result<(), &'static str> {
        const CLASSIFIERS: &[&str] = &[
            "rust.struct", "rust.trait", "uml.class", "uml.interface",
            "uml.signal", "uml.data_type", "uml.primitive_type",
            "uml.enumeration", "uml.association_class", "uml.template",
        ];
        const PACKAGES: &[&str] = &["uml.package", "uml.model", "uml.subsystem"];
        if self == Self::Containment {
            return if PACKAGES.contains(&source) { Ok(()) }
                else { Err("containment source must be a UML Package/Model/Subsystem") };
        }
        if self == Self::InterfaceRealization && target != "rust.trait" && target != "uml.interface" {
            return Err("interface realization must target an Interface");
        }
        if self == Self::Generalization && (!CLASSIFIERS.contains(&source) || !CLASSIFIERS.contains(&target)) {
            return Err("generalization requires classifier endpoints");
        }
        if self == Self::TemplateBinding && source != "uml.template" && target != "uml.template" {
            return Err("template binding requires a Template endpoint");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModelingTool {
    Element(ModelingElementKind),
    Relation(ModelingRelationKind),
}
impl ModelingTool {
    pub fn label(self) -> &'static str {
        match self { Self::Element(k) => k.label(), Self::Relation(k) => k.label() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_starum_class_tool_has_a_unique_semantic_kind() {
        use std::collections::HashSet;
        let nodes = ModelingElementKind::BASIC.iter()
            .chain(ModelingElementKind::ADVANCED)
            .chain(ModelingElementKind::PACKAGES)
            .chain(ModelingElementKind::ANNOTATIONS);
        let relations = ModelingRelationKind::BASIC.iter()
            .chain(ModelingRelationKind::ADVANCED)
            .chain(ModelingRelationKind::PACKAGES);
        let mut kinds = HashSet::new();
        for kind in nodes { assert!(kinds.insert(kind.model_kind())); }
        for kind in relations { assert!(kinds.insert(kind.model_kind())); }
        assert_eq!(kinds.len(), 24);
    }
    #[test]
    fn generalization_and_realization_validate_targets() {
        assert!(ModelingRelationKind::Generalization.validate_endpoints("rust.struct", "rust.trait").is_ok());
        assert!(ModelingRelationKind::InterfaceRealization.validate_endpoints("rust.struct", "uml.package").is_err());
        assert!(ModelingRelationKind::Containment.validate_endpoints("rust.struct", "rust.trait").is_err());
    }
}
