//! Toolbox intent and endpoint contracts; visual-only tools never claim semantic relationships.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModelingRelationKind {
    ElementImport,
    ProfileApplication,
    Extension,
    Connector,
    NoteLink,
    ConstraintLink,
    MessageSynchronous,
    MessageAsynchronous,
    MessageReply,
    MessageFound,
    MessageLost,
    GeneralOrdering,
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
        Self::MessageFound,
        Self::MessageLost,
        Self::GeneralOrdering,
    ];
    pub fn resolve_message_sort(self) -> Option<&'static str> {
        match self {
            Self::MessageSynchronous | Self::MessageFound | Self::MessageLost => Some("synchCall"),
            Self::MessageAsynchronous => Some("asynchCall"),
            Self::MessageReply => Some("reply"),
            _ => None,
        }
    }

    pub fn resolve_message_kind(self) -> Option<&'static str> {
        match self {
            Self::MessageFound => Some("found"),
            Self::MessageLost => Some("lost"),
            _ => None,
        }
    }
    pub fn uses_occurrence_anchors(self) -> bool {
        self.resolve_message_sort().is_some() || self == Self::GeneralOrdering
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
        Self::ElementImport,
        Self::ProfileApplication,
        Self::Extension,
        Self::Connector,
        Self::NoteLink,
        Self::ConstraintLink,
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
            Self::ElementImport => "Element import",
            Self::ProfileApplication => "Profile application · visual",
            Self::Extension => "Extension · visual",
            Self::Connector => "Connector · visual",
            Self::NoteLink => "Comment link · visual",
            Self::ConstraintLink => "Constraint link · visual",
            Self::MessageSynchronous => "Synchronous message",
            Self::MessageAsynchronous => "Asynchronous message",
            Self::MessageReply => "Reply message",
            Self::MessageFound => "Found message · visual",
            Self::MessageLost => "Lost message · visual",
            Self::GeneralOrdering => "General ordering · visual",
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
            Self::ElementImport => "uml.element_import",
            Self::ProfileApplication => "uml.profile_application",
            Self::Extension => "uml.extension",
            Self::Connector => "uml.connector",
            Self::NoteLink => "uml.note_link",
            Self::ConstraintLink => "uml.constraint_link",
            Self::MessageSynchronous
            | Self::MessageAsynchronous
            | Self::MessageReply
            | Self::MessageFound
            | Self::MessageLost => "uml.message",
            Self::GeneralOrdering => "uml.general_ordering",
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
            Self::ElementImport => "click importing namespace, then imported element",
            Self::ProfileApplication => "click applying package, then profile (visual only)",
            Self::Extension => "click stereotype, then metaclass (filled triangle at metaclass)",
            Self::Connector => "click a role or port, then another role or port (visual only)",
            Self::NoteLink => "click comment, then annotated view (visual only)",
            Self::ConstraintLink => "click constraint, then constrained view (visual only)",
            Self::MessageSynchronous | Self::MessageAsynchronous | Self::MessageReply => {
                "click the source lifeline at send position, then the target at receive position"
            }
            Self::MessageFound => "click an empty send position, then the receiving Lifeline",
            Self::MessageLost => "click the sending Lifeline, then an empty receive position",
            Self::GeneralOrdering => {
                "click the before occurrence position, then the after position (visual only)"
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
            Self::ProfileApplication
                | Self::Extension
                | Self::Connector
                | Self::NoteLink
                | Self::ConstraintLink
                | Self::AssociationClassAttachment
                | Self::MessageSynchronous
                | Self::MessageAsynchronous
                | Self::MessageReply
                | Self::MessageFound
                | Self::MessageLost
                | Self::GeneralOrdering
        )
    }
}
