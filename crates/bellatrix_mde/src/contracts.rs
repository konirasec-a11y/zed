pub(crate) mod composition;
pub use composition::{CompositionParent, CompositionPlacement, composition_paths};
pub mod endpoint_presentation;
pub mod presentation;
mod project;
mod provenance;
mod section;
pub mod semantic;
pub mod uml_presentation;

pub use endpoint_presentation::{
    EndpointMarkerBinding, EndpointMarkerComposition, EndpointMarkerGlyph, EndpointMarkerPart,
    EndpointMarkerPlacement, EndpointMarkerSource, EndpointMarkerTarget,
    RelationshipEndpointPresentation,
};

pub use presentation::{
    Authored, Diagram, Geometry, HierarchyPlacement, LabelPlacement, LayoutBounds, LineMode,
    PathSegment, Point, PresentationReference, PresentationResource, PresentationSource,
    RelationshipRouting, Style, TextAlignment, TextVerticalAlignment, TimingMode, TimingPoint,
    TimingPresentation, View, ViewEndpoints, ViewOccurrence, ViewPresentation, VisualDraft,
};
pub use project::Project;
pub use provenance::{MdjObjectResidual, MdjProjectionFingerprint, MdjProvenance};
pub use section::ModelingSection;
pub use semantic::{Annotation, Element, FileContent, Implementation, Relation, StereotypeValue};
pub use uml_presentation::{AssociationEndConvention, StereotypeDisplay, UmlStyle};
