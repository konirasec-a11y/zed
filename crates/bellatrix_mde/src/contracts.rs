pub(crate) mod composition;
pub use composition::{CompositionParent, CompositionPlacement, composition_paths};
pub mod endpoint_presentation;
pub mod presentation;
mod project;
mod provenance;
mod section;
pub mod semantic;
pub mod uml_presentation;
pub mod uml_authoring;

pub use endpoint_presentation::{
    EndpointMarkerBinding, EndpointMarkerComposition, EndpointMarkerGlyph, EndpointMarkerPart,
    EndpointMarkerPlacement, EndpointMarkerSource, EndpointMarkerTarget,
    RelationshipEndpointPresentation,
};

pub use presentation::{
    Authored, Diagram, Geometry, HierarchyPlacement, LabelOptionsSource, LabelPlacement, LayoutBounds, LayoutExtent, UnresolvedExtentReason, LineMode,
    PathSegment, Point, PresentationReference, PresentationResource, PresentationSource,
    RelationshipRouting, Style, StyleInheritance, InheritedStyleProperty, SourceFieldPresence,
    SubjectLabelBinding, SubjectLabelRole, TextAlignment, TextVerticalAlignment, TimingMode, TimingPoint,
    InterfaceIconBinding, InterfaceConnection, InterfaceConnectionRole, EntityBoxBinding, RelationshipNotation,
    TimingPresentation, View, ViewEndpoints, ViewOccurrence, ViewPresentation, VisualDraft,
};
pub use project::Project;
pub use uml_authoring::{AssociationKnowledge, AuthoringKnowledge, NavigationKnowledge};
pub use provenance::{MdjObjectResidual, MdjProjectionFingerprint, MdjProvenance};
pub use section::ModelingSection;
pub use semantic::{Annotation, Element, FileContent, Implementation, Relation, StereotypeValue};
pub use uml_presentation::{AssociationEndConvention, StereotypeDisplay, UmlStyle};
