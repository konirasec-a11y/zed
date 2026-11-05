pub mod composition;
pub use composition::{CompositionParent, CompositionPlacement, composition_paths};
pub mod endpoint_presentation;
pub mod presentation;
mod project;
mod provenance;
mod section;
pub mod semantic;
pub mod uml_authoring;
pub mod uml_presentation;

pub use endpoint_presentation::{
    EndpointMarkerBinding, EndpointMarkerComposition, EndpointMarkerGlyph, EndpointMarkerPart,
    EndpointMarkerPlacement, EndpointMarkerSource, EndpointMarkerTarget,
    RelationshipEndpointPresentation,
};

pub use presentation::{
    Authored, Diagram, EntityBoxBinding, Geometry, HierarchyPlacement, InheritedStyleProperty,
    InterfaceConnection, InterfaceConnectionRole, InterfaceIconBinding, LabelOptionsSource,
    LabelPlacement, LayoutBounds, LayoutExtent, LineMode, PathSegment, Point,
    PresentationReference, PresentationResource, PresentationSource, RelationshipLabelBindings,
    RelationshipNotation, RelationshipRouting, SourceFieldPresence, Style, StyleInheritance,
    SubjectLabelBinding, SubjectLabelRole, TextAlignment, TextVerticalAlignment, TimingMode,
    TimingPoint, TimingPresentation, UnresolvedExtentReason, View, ViewEndpoints, ViewOccurrence,
    ViewPresentation, ViewStyleDefaults, VisualDraft,
};
pub use project::Project;
pub use provenance::{MdjObjectResidual, MdjProjectionFingerprint, MdjProvenance};
pub use section::{ModelingSection, StereotypeLabel, SubjectPresentation};
pub use semantic::{Annotation, FileContent, Implementation, StereotypeValue};
pub use uml_authoring::{AssociationKnowledge, AuthoringKnowledge, NavigationKnowledge};
pub use uml_presentation::{AssociationEndConvention, StereotypeDisplay, UmlStyle};
