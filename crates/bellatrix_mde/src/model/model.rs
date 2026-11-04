#[path = "diagram.rs"]
mod diagram;
#[path = "graph.rs"]
mod graph;
#[path = "../persistence/io.rs"]
mod io;
#[path = "project.rs"]
mod project;
#[path = "section.rs"]
mod section;
#[path = "../persistence/wire.rs"]
pub(crate) mod wire;

pub use diagram::{
    Diagram, Geometry, LineMode, PathSegment, Point, RelationshipRouting, StereotypeDisplay, Style,
    TextAlignment, TextVerticalAlignment, TimingMode, TimingPoint, TimingPresentation, View, ViewEndpoints,
    VisualDraft,
};
pub use graph::{Annotation, Element, FileContent, Implementation, Relation};
pub use io::{Format, ModelIoError, decode, encode};
pub use project::{ExplorerDiagramOccurrence, Project};
pub use section::ModelingSection;
