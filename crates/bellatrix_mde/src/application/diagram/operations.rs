use crate::{Geometry, ModelingSection, RelationshipRouting, ViewEndpoints};

pub(crate) struct RoutingProjection {
    pub(crate) geometry: Geometry,
    pub(crate) endpoints: ViewEndpoints,
}

pub(crate) trait DiagramOperations {
    fn project_routing(
        &self,
        section: &ModelingSection,
        diagram_id: &str,
        view_id: &str,
        routing: RelationshipRouting,
    ) -> Result<RoutingProjection, String>;
}
