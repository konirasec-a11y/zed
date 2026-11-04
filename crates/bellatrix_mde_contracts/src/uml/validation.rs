/// UML 2.5.1 section 7.2; semantic inheritance is distinct from visual geometry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootMetaclass {
    Element,
    Relationship,
    DirectedRelationship,
    Comment,
}
impl RootMetaclass {
    pub const fn parent(self) -> Option<Self> {
        match self {
            Self::Element => None,
            Self::Relationship | Self::Comment => Some(Self::Element),
            Self::DirectedRelationship => Some(Self::Relationship),
        }
    }
}
pub use crate::protocol::trace::DiagnosticSeverity as StructuralSeverity;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralDiagnostic {
    pub severity: StructuralSeverity,
    pub code: &'static str,
    pub subject_id: String,
    pub message: String,
}
