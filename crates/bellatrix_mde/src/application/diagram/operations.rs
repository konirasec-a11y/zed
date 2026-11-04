use crate::{Geometry, ModelingSection, RelationshipRouting, ViewEndpoints};

pub(crate) struct RoutingProjection {
    pub(crate) geometry: Geometry,
    pub(crate) endpoints: ViewEndpoints,
}

pub(crate) trait DiagramOperations {
    fn reanchor_manual_paths(
        &self,
        previous: &ModelingSection,
        current: &mut ModelingSection,
        diagram_id: &str,
    ) -> Result<(), String>;

    fn current_path(
        &self,
        section: &ModelingSection,
        diagram_id: &str,
        view_id: &str,
    ) -> Result<Geometry, String>;

    fn project_routing(
        &self,
        section: &ModelingSection,
        diagram_id: &str,
        view_id: &str,
        routing: RelationshipRouting,
    ) -> Result<RoutingProjection, String>;
}
