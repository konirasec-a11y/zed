//! Toolbox intent and endpoint contracts; visual-only tools never claim semantic relationships.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModelingRelationKind {
    MessageSynchronous,
    MessageAsynchronous,
    MessageReply,
    AssociationClassAttachment,
    Association,
    DirectedAssociation,
    Aggregation,
    Composition,
    Dependency,
    Generalization,
    InterfaceRealization,
    TemplateBinding,
    Realization,
    Containment,
    Usage,
    Abstraction,
    Include,
    Extend,
    PackageImport,
    PackageMerge,
    Deployment,
    Manifestation,
    ControlFlow,
    ObjectFlow,
    Transition,
}
impl ModelingRelationKind {
    pub const MESSAGES: &'static [Self] = &[
        Self::MessageSynchronous,
        Self::MessageAsynchronous,
        Self::MessageReply,
    ];
    pub fn resolve_message_sort(self) -> Option<&'static str> {
        match self {
            Self::MessageSynchronous => Some("synchCall"),
            Self::MessageAsynchronous => Some("asynchCall"),
            Self::MessageReply => Some("reply"),
            _ => None,
        }
    }

    pub const BASIC: &'static [Self] = &[
        Self::Association,
        Self::DirectedAssociation,
        Self::Aggregation,
        Self::Composition,
        Self::Dependency,
        Self::Generalization,
        Self::InterfaceRealization,
    ];
    pub const ADVANCED: &'static [Self] = &[
        Self::TemplateBinding,
        Self::Realization,
        Self::AssociationClassAttachment,
    ];
    pub const BEHAVIOR: &'static [Self] = &[
        Self::Include,
        Self::Extend,
        Self::ControlFlow,
        Self::ObjectFlow,
        Self::Transition,
    ];
    pub const STRUCTURAL: &'static [Self] = &[
        Self::Usage,
        Self::Abstraction,
        Self::PackageImport,
        Self::PackageMerge,
        Self::Deployment,
        Self::Manifestation,
    ];
    pub const PACKAGES: &'static [Self] = &[Self::Containment];
    pub fn label(self) -> &'static str {
        match self {
            Self::MessageSynchronous => "Synchronous message",
            Self::MessageAsynchronous => "Asynchronous message",
            Self::MessageReply => "Reply message",
            Self::AssociationClassAttachment => "Association-class attachment",
            Self::Association => "Association",
            Self::DirectedAssociation => "Directed Association",
            Self::Aggregation => "Aggregation",
            Self::Composition => "Composition",
            Self::Dependency => "Dependency",
            Self::Generalization => "Generalization",
            Self::InterfaceRealization => "Interface Realization",
            Self::TemplateBinding => "Template Binding",
            Self::Realization => "Realization",
            Self::Containment => "Containment",
            Self::Usage => "Usage",
            Self::Abstraction => "Abstraction",
            Self::Include => "Include",
            Self::Extend => "Extend",
            Self::PackageImport => "Package Import",
            Self::PackageMerge => "Package Merge",
            Self::Deployment => "Deployment",
            Self::Manifestation => "Manifestation",
            Self::ControlFlow => "Control Flow",
            Self::ObjectFlow => "Object Flow",
            Self::Transition => "Transition",
        }
    }
    pub fn model_kind(self) -> &'static str {
        match self {
            Self::MessageSynchronous | Self::MessageAsynchronous | Self::MessageReply => {
                "uml.message"
            }
            Self::AssociationClassAttachment => "view.association_class_attachment",
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
            Self::Usage => "uml.usage",
            Self::Abstraction => "uml.abstraction",
            Self::Include => "uml.include",
            Self::Extend => "uml.extend",
            Self::PackageImport => "uml.package_import",
            Self::PackageMerge => "uml.package_merge",
            Self::Deployment => "uml.deployment",
            Self::Manifestation => "uml.manifestation",
            Self::ControlFlow => "uml.control_flow",
            Self::ObjectFlow => "uml.object_flow",
            Self::Transition => "uml.transition",
        }
    }
    /// The first click is Relation.from, not necessarily the arrowhead end.
    pub fn endpoint_instruction(self) -> &'static str {
        match self {
            Self::MessageSynchronous | Self::MessageAsynchronous | Self::MessageReply => {
                "click the source lifeline at send position, then the target at receive position"
            }
            Self::AssociationClassAttachment => {
                "click the association path, then its classifier box (visual attachment)"
            }
            Self::Association => "click either first class, then the second (no arrow)",
            Self::DirectedAssociation => "click source, then navigable target (arrow at target)",
            Self::Aggregation => "click the whole, then the part (hollow diamond at first)",
            Self::Composition => "click the whole, then the part (filled diamond at first)",
            Self::Dependency => "click dependent client, then supplier (arrow at target)",
            Self::Generalization => "click subtype, then supertype (triangle at target)",
            Self::InterfaceRealization => {
                "click implementing class, then interface (triangle at target)"
            }
            Self::TemplateBinding => "click bound element, then template (arrow at target)",
            Self::Realization => {
                "click implementing classifier, then specification (triangle at target)"
            }
            Self::Containment => "click container, then contained element (circle at first)",
            Self::Usage | Self::Abstraction => "click client, then supplier",
            Self::Include => "click including use case, then included use case",
            Self::Extend => "click extending use case, then extended use case",
            Self::PackageImport => "click importing namespace, then imported package",
            Self::PackageMerge => "click receiving package, then merged package",
            Self::Deployment => "click deployed artifact, then deployment target",
            Self::Manifestation => "click artifact, then manifested model element",
            Self::ControlFlow | Self::ObjectFlow => {
                "click source activity node, then target in the same activity"
            }
            Self::Transition => "click source vertex, then target in the same region",
        }
    }

    pub fn is_visual_only(self) -> bool {
        matches!(
            self,
            Self::AssociationClassAttachment
                | Self::MessageSynchronous
                | Self::MessageAsynchronous
                | Self::MessageReply
        )
    }
    pub fn validate_endpoints(self, source: &str, target: &str) -> Result<(), &'static str> {
        let conforms = |kind: &str, expected: &str| {
            crate::uml::root::object_metaclass(kind)
                .is_some_and(|actual| crate::uml::metamodel::conforms_to(actual, expected))
        };
        let valid = match self {
            Self::Include | Self::Extend => {
                Some(conforms(source, "UseCase") && conforms(target, "UseCase"))
            }
            Self::PackageImport => {
                Some(conforms(source, "Namespace") && conforms(target, "Package"))
            }
            Self::PackageMerge => Some(conforms(source, "Package") && conforms(target, "Package")),
            Self::Deployment => {
                Some(conforms(source, "DeployedArtifact") && conforms(target, "DeploymentTarget"))
            }
            Self::Manifestation => {
                Some(conforms(source, "Artifact") && conforms(target, "PackageableElement"))
            }
            Self::ControlFlow | Self::ObjectFlow => {
                Some(conforms(source, "ActivityNode") && conforms(target, "ActivityNode"))
            }
            Self::Transition => Some(conforms(source, "Vertex") && conforms(target, "Vertex")),
            Self::Usage | Self::Abstraction => {
                Some(conforms(source, "NamedElement") && conforms(target, "NamedElement"))
            }
            _ => None,
        };
        if let Some(valid) = valid {
            return if valid {
                Ok(())
            } else {
                Err("endpoints do not conform to this UML relationship's types")
            };
        }
        const PACKAGES: &[&str] = &["uml.package", "uml.model", "uml.subsystem"];
        if self == Self::Containment {
            return if PACKAGES.contains(&source) {
                Ok(())
            } else {
                Err("containment source must be a UML Package/Model/Subsystem")
            };
        }
        if self == Self::InterfaceRealization && target != "rust.trait" && target != "uml.interface"
        {
            return Err("interface realization must target an Interface");
        }
        if self == Self::Generalization
            && (!conforms(source, "Classifier") || !conforms(target, "Classifier"))
        {
            return Err("generalization requires classifier endpoints");
        }
        if self == Self::TemplateBinding && source != "uml.template" && target != "uml.template" {
            return Err("template binding requires a Template endpoint");
        }
        Ok(())
    }
}
