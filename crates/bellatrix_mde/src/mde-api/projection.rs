use crate::mde_api_projection_notation::{
    project_render_styles, project_view, relationship_labels,
};
use bellatrix_mde_contracts::protocol::command::*;
use bellatrix_mde_contracts::protocol::presentation::*;
pub(crate) use bellatrix_mde_contracts::protocol::projection::*;

pub(crate) fn decode_section(
    project_id: &str,
    name: &str,
    section_json: &str,
) -> Result<crate::ModelingSection, String> {
    let stored = serde_json::from_str(section_json)
        .map_err(|error| format!("invalid MDE section: {error}"))?;
    let section = crate::contract_document::project_section(stored)?;
    let project = section.to_model(project_id, name);
    crate::Graph::new(&project)
        .validate_identifiers()
        .map_err(|error| error.to_string())?;
    crate::validate_model_hierarchy(&project)?;
    Ok(section)
}

pub fn project_model_section(
    project_id: &str,
    name: &str,
    section_json: &str,
) -> Result<ModelingProjection, String> {
    let section = decode_section(project_id, name, section_json)?;
    let references = crate::uml::references::ReferenceIndex::new(
        crate::uml::root::Model::from_section(&section),
    );
    let elements = section
        .elements
        .iter()
        .map(|element| {
            let notation = crate::uml_notation(&element.kind);
            let header = crate::uml_header(
                &element.kind,
                crate::stereotype_presentation(&section, element).label(),
            );
            Element {
                id: element.id.clone(),
                kind: element.kind.clone(),
                name: element.name.clone(),
                display_label: crate::element_display_label(element),
                name_expression: crate::commands::name_expression_text(element),
                visibility: element
                    .properties
                    .get("visibility")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned),
                quick_actions: crate::mde_api_projection_inspector::project_quick_actions(
                    &element.kind,
                ),
                quick_create: [
                    UmlQuickCreate::Note,
                    UmlQuickCreate::Constraint,
                    UmlQuickCreate::Reception,
                    UmlQuickCreate::Subtype,
                    UmlQuickCreate::Supertype,
                    UmlQuickCreate::RealizingClass,
                    UmlQuickCreate::TemplateParameter,
                    UmlQuickCreate::ProvidedInterface,
                    UmlQuickCreate::RequiredInterface,
                    UmlQuickCreate::AssociatedClass,
                    UmlQuickCreate::AggregatedClass,
                    UmlQuickCreate::CompositedClass,
                    UmlQuickCreate::Port,
                    UmlQuickCreate::Part,
                ]
                .into_iter()
                .filter(|action| crate::uml_types::quick_create_accepts(*action, &element.kind))
                .collect(),
                is_classifier: crate::UmlType::from_kind(&element.kind)
                    .is_some_and(crate::UmlType::is_classifier),
                is_comment: crate::UmlType::from_kind(&element.kind) == Some(crate::UmlType::Note),
                template_parametered_id: crate::template_parametered_element(&section, element)
                    .map(|target| target.id.clone()),
                members: section
                    .relations
                    .iter()
                    .filter(|relation| relation.kind == "owns" && relation.from == element.id)
                    .map(|relation| relation.to.clone())
                    .collect(),
                template_parameters: crate::template_parameters(&section, &element.id)
                    .into_iter()
                    .map(|parameter| parameter.id.clone())
                    .collect(),
                template_label: crate::template_parameter_label_with_references(
                    &section,
                    element,
                    &references,
                ),
                structured_classifier: crate::uml::root::ElementRef::Object(element)
                    .conforms_to("StructuredClassifier"),
                member: MemberPresentation {
                    underlined: crate::member_presentation(element).underlined,
                },
                minimum_size: crate::resolve_element_notation(element).minimum_size(),
                minimum_header_height: header.minimum_height(notation),
                minimum_interface_height: (crate::UmlType::from_kind(&element.kind)
                    == Some(crate::UmlType::Interface))
                .then(|| {
                    crate::classifier_contents(&section, &element.id).minimum_interface_height()
                }),
            }
        })
        .collect();
    let relations = section
        .relations
        .iter()
        .map(|relation| {
            let presentation = crate::relationship_presentation(&section, relation);
            Relation {
                id: relation.id.clone(),
                kind: relation.kind.clone(),
                from: relation.from.clone(),
                to: relation.to.clone(),
                source_role: presentation.source_role.to_owned(),
                target_role: presentation.target_role.to_owned(),
                presentation: relationship_labels(&presentation),
            }
        })
        .collect();
    let render_styles = project_render_styles(&section, project_id)?;
    let diagrams = section
        .diagrams
        .iter()
        .map(|diagram| {
            let graph = crate::RelationshipGraph::new(&section, &diagram.id);
            let paths = diagram
                .all_views()
                .filter_map(|view| {
                    graph
                        .view_path(&view.id)
                        .map(|geometry| (view.id.clone(), geometry))
                })
                .collect();
            Ok(Diagram {
                id: diagram.id.clone(),
                name: diagram.name.clone(),
                width: diagram.width,
                height: diagram.height,
                type_label: crate::mde_api_projection_tools::diagram_type_label(diagram),
                owner_label: section
                    .composition
                    .get(&diagram.id)
                    .and_then(|placement| placement.parent_id(&diagram.id, &section.relations))
                    .and_then(|id| section.elements.iter().find(|element| element.id == id))
                    .map(|element| element.name.clone())
                    .filter(|name| !name.trim().is_empty()),
                uml_di_type: diagram.uml_di_type.clone(),
                interaction_kind: diagram.interaction_kind.clone(),
                model_elements: diagram.model_elements.clone(),
                kind: crate::StructuralDiagramKind::from_diagram(diagram),
                views: diagram
                    .views
                    .iter()
                    .map(|view| {
                        project_view(&section, diagram, view, &references, &paths, &render_styles)
                    })
                    .collect::<Result<_, String>>()?,
            })
        })
        .collect::<Result<_, String>>()?;
    let hierarchy = section.hierarchy.clone();
    let inspections = section
        .elements
        .iter()
        .map(|element| &element.id)
        .chain(section.relations.iter().map(|relation| &relation.id))
        .filter_map(|id| {
            crate::mde_api_projection_inspector::project_inspector(&section, id)
                .map(|inspection| (id.clone(), inspection))
        })
        .collect();
    let kinds = section
        .elements
        .iter()
        .map(|element| element.kind.as_str())
        .chain(
            section
                .relations
                .iter()
                .map(|relation| relation.kind.as_str()),
        )
        .chain(
            section
                .diagrams
                .iter()
                .flat_map(|diagram| diagram.all_views())
                .filter_map(|view| {
                    view.draft
                        .as_ref()
                        .map(|draft| draft.notation_kind.as_str())
                }),
        )
        .chain(["uml.composition", "uml.aggregation"]);
    let identities = kinds
        .map(|kind| {
            let identity = crate::semantic_identity(kind);
            (
                kind.to_owned(),
                SemanticIdentity {
                    type_label: crate::mde_api_projection_tools::imported_type_label(kind)
                        .unwrap_or(identity.type_label)
                        .to_owned(),
                    metaclass: identity.metaclass.map(str::to_owned),
                    relationship: kind == "mdj.ERDRelationship"
                        || identity.metaclass.is_some_and(|metaclass| {
                            crate::uml::metamodel::conforms_to(metaclass, "Relationship")
                        }),
                },
            )
        })
        .collect();
    let hierarchy_rows = crate::project_hierarchy(&section)
        .into_iter()
        .map(|row| HierarchyRow {
            element_id: row.element_id,
            label: row.label,
            kind: row.kind,
            depth: row.depth,
        })
        .collect();
    Ok(ModelingProjection {
        tools: crate::mde_api_projection_tools::project(&section.diagrams),
        root_model: section.root_model.clone(),
        elements,
        relations,
        diagrams,
        hierarchy,
        composed_subjects: section.composition.keys().cloned().collect(),
        inspections,
        identities,
        hierarchy_rows,
        explorer: crate::mde_api_projection_explorer::project(&section)?,
    })
}
