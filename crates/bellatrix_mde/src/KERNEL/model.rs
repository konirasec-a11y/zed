#[path = "model/diagram.rs"]
mod diagram;
#[path = "model/graph.rs"]
mod graph;
#[path = "model/io.rs"]
mod io;
#[path = "model/project.rs"]
mod project;
#[path = "model/section.rs"]
mod section;
#[path = "model/wire.rs"]
pub(crate) mod wire;

pub use diagram::{
    Diagram, Geometry, LineMode, PathSegment, Point, RelationshipRouting, StereotypeDisplay, Style,
    TextAlignment, TextVerticalAlignment, TimingMode, TimingPoint, TimingPresentation, View, ViewEndpoints,
    VisualDraft,
};
pub use graph::{Annotation, Element, FileContent, Implementation, Relation};
pub use io::{Format, ModelIoError, decode, encode};
pub use project::Project;
pub use section::ModelingSection;
