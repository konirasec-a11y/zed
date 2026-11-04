use crate::diagram_operations::{DiagramOperations, RoutingProjection};
use crate::{Geometry, ModelingSection, RelationshipRouting, View};

pub(crate) struct ModelingDiagramOperations;

fn view<'a>(
    section: &'a ModelingSection,
    diagram_id: &str,
    view_id: &str,
) -> Result<&'a View, String> {
    section
        .diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?
        .views
        .iter()
        .find(|view| view.id == view_id)
        .ok_or_else(|| "view not found in diagram".into())
}

impl DiagramOperations for ModelingDiagramOperations {
    fn project_routing(
        &self,
        section: &ModelingSection,
        diagram_id: &str,
        view_id: &str,
        routing: RelationshipRouting,
    ) -> Result<RoutingProjection, String> {
        let current = view(section, diagram_id, view_id)?;
        if !matches!(current.geometry, Geometry::Path { .. }) {
            return Err("routing requires a relationship Path View".into());
        }
        let relation = section
            .relations
            .iter()
            .find(|relation| Some(relation.id.as_str()) == current.subject.as_deref())
            .ok_or("view does not represent a relationship")?;
        let endpoints = match &current.endpoints {
            Some(endpoints) => endpoints.clone(),
            None => crate::relationships::resolve_view_endpoints(
                section,
                diagram_id,
                &relation.from,
                &relation.to,
            )?,
        };
        let geometry = crate::relationships::route_for_routing(
            section,
            diagram_id,
            relation,
            Some(&endpoints),
            routing,
        )?;
        Ok(RoutingProjection {
            geometry,
            endpoints,
        })
    }
}
