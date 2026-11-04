use crate as mde;
use crate::mde_api_projection as projection;
use bellatrix_mde_contracts::protocol::command::{LineMode, TimingMode, TimingPresentation};
use bellatrix_mde_contracts::protocol::presentation::*;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[path = "projection_entity.rs"]
mod entity;
#[path = "projection_interface.rs"]
mod interface;
#[path = "projection_labels.rs"]
mod labels;

pub(crate) fn project_view(
    section: &mde::ModelingSection,
    diagram: &mde::Diagram,
    view: &mde::View,
    references: &mde::uml::references::ReferenceIndex<'_>,
    paths: &BTreeMap<String, mde::Geometry>,
    render_styles: &BTreeMap<String, ProjectedRenderStyle>,
) -> Result<projection::View, String> {
    let element = view.subject.as_deref().and_then(|subject| {
        section
            .elements
            .iter()
            .find(|element| element.id == subject)
    });
    let timing = view.timing.clone();
    let projected_style = render_styles
        .get(&view.id)
        .ok_or_else(|| format!("missing render style projection for view {}", view.id))?;
    let mut render_style = projected_style.style.clone();
    let mut rendered_label = labels::project(section, diagram, view, references, &mut render_style);
    let entity_box = entity::project(
        section,
        diagram,
        view,
        &mut rendered_label,
        &mut render_style,
    );
    let figure = figure_presentation(section, diagram, view, element, references, timing.clone())?;
    let classifier = element
        .map(|element| {
            let presentation = mde::classifier_presentation(
                element,
                mde::ClassifierPresentationOptions {
                    show_active_class_sides: true,
                },
            );
            ClassifierPresentation {
                name_is_italic: presentation.name_is_italic,
                double_sided: presentation.double_sided,
            }
        })
        .unwrap_or_default();
    let display_label = element
        .map(|element| figure_label(element, references, &view.style))
        .unwrap_or_else(|| view.label.clone().unwrap_or_default());
    let member_labels = element
        .map(|owner| {
            section
                .relations
                .iter()
                .filter(|relation| relation.kind == "owns" && relation.from == owner.id)
                .filter_map(|relation| {
                    section
                        .elements
                        .iter()
                        .find(|member| member.id == relation.to)
                })
                .map(|member| {
                    (
                        member.id.clone(),
                        mde::format_element_label_for_view(member, references, &view.style),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let relationship = if let Some(resolved) = mde::resolve_relationship_view(section, view) {
        Some(projection::ResolvedRelationship {
            communication_message: resolved.kind == "uml.message"
                && diagram.interaction_kind.as_deref() == Some("communication"),
            kind: resolved.kind,
            presentation: relationship_labels(&resolved.presentation),
            style: relationship_style(resolved.presentation.style),
            markers: resolved
                .markers
                .as_ref()
                .map(|(source, target)| (marker_composition(source), marker_composition(target))),
            diagnostics: resolved.diagnostics,
        })
    } else if view.subject.is_none()
        && matches!(&view.geometry, mde::Geometry::Path { .. })
        && let Some(draft) = &view.draft
    {
        let mut properties = draft.properties.clone();
        if let Some(label) = view.label.as_ref().filter(|label| !label.is_empty()) {
            properties.insert("name".into(), Value::String(label.clone()));
        }
        Some(projection::ResolvedRelationship {
            kind: draft.notation_kind.clone(),
            presentation: RelationshipPresentation {
                name: mde::format_relationship_label(&draft.notation_kind, &properties),
                source_labels: RelationshipEndpointLabels::default(),
                target_labels: RelationshipEndpointLabels::default(),
                type_label: mde::semantic_identity(&draft.notation_kind)
                    .type_label
                    .to_owned(),
            },
            style: relationship_style(mde::resolve_relationship_style(
                &draft.notation_kind,
                &draft.properties,
            )),
            markers: None,
            diagnostics: Vec::new(),
            communication_message: draft.notation_kind == "uml.message"
                && diagram.interaction_kind.as_deref() == Some("communication"),
        })
    } else {
        None
    };
    if view.presentation.relationship_notation
        == Some(mde::RelationshipNotation::EntityRelationship)
        && let Some(resolved) = &relationship
    {
        render_style.stroke_pattern = Some(match resolved.style.line_mode {
            LineMode::Solid => Vec::new(),
            LineMode::Dotted => vec![3.0],
            LineMode::Dashed => vec![6.0, 5.0],
        });
    }
    let visible = rendered_label
        .visible
        .unwrap_or_else(|| visible_on_canvas(view));
    let routed_geometry = if view.style.routing == Some(mde::RelationshipRouting::Manual) {
        None
    } else {
        paths
            .get(&view.id)
            .map(|geometry| {
                geometry.validate()?;
                Ok::<_, String>(geometry.clone())
            })
            .transpose()?
    };
    Ok(projection::View {
        id: view.id.clone(),
        subject: view.subject.clone(),
        draft: view
            .draft
            .as_ref()
            .map(|draft| projection::VisualDraftSummary {
                notation_kind: draft.notation_kind.clone(),
                parent_view_id: draft.parent_view_id.clone(),
            }),
        endpoints: view.endpoints.clone(),
        timing,
        label: view.label.clone(),
        rendered_label,
        interface_icon: interface::project(section, diagram, view),
        entity_box,
        geometry: view.geometry.clone(),
        children: view
            .children
            .iter()
            .map(|child| project_view(section, diagram, child, references, paths, render_styles))
            .collect::<Result<_, _>>()?,
        presentation: projection::ViewPresentation {
            kind: view.presentation.notation_kind.clone(),
            internal: view.presentation.internal,
            // Expose every title through one canvas role while retaining the
            // existing ERD binding as its sole persisted source of truth.
            name_label: view
                .presentation
                .name_label
                .as_ref()
                .or_else(|| {
                    view.presentation
                        .entity_box
                        .as_ref()
                        .and_then(|binding| binding.name_label.value())
                        .filter(|id| view.children.iter().any(|child| &child.id == *id))
                })
                .cloned(),
            divider: view.presentation.divider,
            has_source: view.presentation.source.is_some(),
            visible,
            style_inheritance: projected_style.inheritance.clone(),
        },
        style: view.style.clone(),
        z_index: view.z_index,
        render_style,
        figure,
        classifier,
        display_label,
        member_labels,
        relationship,
        routed_geometry,
    })
}

fn visible_on_canvas(view: &mde::View) -> bool {
    view.presentation.visible.unwrap_or(true)
}

pub(crate) struct ProjectedRenderStyle {
    style: RenderStyle,
    inheritance: Option<projection::StyleInheritance>,
}

#[derive(Clone)]
enum StylePropertyResolution<'a> {
    Origin(&'a str),
    Unresolved(projection::StyleInheritanceDiagnostic),
}

pub(crate) fn project_render_styles(
    section: &mde::ModelingSection,
    project_id: &str,
) -> Result<BTreeMap<String, ProjectedRenderStyle>, String> {
    let views = section
        .diagrams
        .iter()
        .flat_map(mde::Diagram::all_views)
        .map(|view| (view.id.as_str(), view))
        .collect::<BTreeMap<_, _>>();
    let non_views = section
        .elements
        .iter()
        .map(|element| element.id.as_str())
        .chain(
            section
                .relations
                .iter()
                .map(|relation| relation.id.as_str()),
        )
        .chain(section.diagrams.iter().map(|diagram| diagram.id.as_str()))
        .chain(std::iter::once(project_id))
        .collect::<BTreeSet<_>>();
    let mut resolved = BTreeMap::new();
    let mut projected_styles = BTreeMap::new();
    let fallback = RenderStyle::default();
    for view in views.values().copied() {
        let mut projected = authored_render_style(view)?;
        if let Some(authored) = view
            .presentation
            .style_inheritance
            .as_ref()
            .filter(|inheritance| {
                inheritance.source.value().is_some() && !inheritance.properties.is_empty()
            })
            && let Some(inheritance) = &mut projected.inheritance
        {
            if let Some(source_id) = authored
                .source
                .value()
                .filter(|source_id| non_views.contains(source_id.as_str()))
            {
                for property in &authored.properties {
                    inheritance.diagnostics.insert(
                        inherited_property(*property),
                        projection::StyleInheritanceDiagnostic::NonViewSource {
                            source_id: source_id.clone(),
                        },
                    );
                }
                projected_styles.insert(view.id.clone(), projected);
                continue;
            }
            for property in &authored.properties {
                let projected_property = inherited_property(*property);
                match resolve_style_property(view, *property, &views, &non_views, &mut resolved) {
                    StylePropertyResolution::Origin(origin_id) => {
                        let origin = views
                            .get(origin_id)
                            .ok_or_else(|| format!("missing style origin view {origin_id}"))?;
                        // None is inherited too. The child's authored value must
                        // not override a source default or an unresolved field.
                        let origin_style = authored_render_style(origin)?.style;
                        copy_inherited_property(&mut projected.style, &origin_style, *property);
                        inheritance
                            .origins
                            .insert(projected_property, origin_id.to_owned());
                    }
                    StylePropertyResolution::Unresolved(diagnostic) => {
                        copy_inherited_property(&mut projected.style, &fallback, *property);
                        inheritance
                            .diagnostics
                            .insert(projected_property, diagnostic);
                    }
                }
            }
            inheritance.status = if inheritance.diagnostics.is_empty() {
                projection::StyleInheritanceStatus::Resolved
            } else {
                projection::StyleInheritanceStatus::Unresolved
            };
        }
        projected_styles.insert(view.id.clone(), projected);
    }
    Ok(projected_styles)
}

fn resolve_style_property<'a>(
    view: &'a mde::View,
    property: mde::InheritedStyleProperty,
    views: &BTreeMap<&'a str, &'a mde::View>,
    non_views: &BTreeSet<&str>,
    resolved: &mut BTreeMap<(&'a str, mde::InheritedStyleProperty), StylePropertyResolution<'a>>,
) -> StylePropertyResolution<'a> {
    let mut pending = Vec::new();
    let mut visiting = BTreeSet::new();
    let mut current = view;
    let resolution = loop {
        if let Some(resolution) = resolved.get(&(current.id.as_str(), property)) {
            break resolution.clone();
        }
        if !visiting.insert(current.id.as_str()) {
            break StylePropertyResolution::Unresolved(
                projection::StyleInheritanceDiagnostic::Cycle {
                    view_id: current.id.clone(),
                },
            );
        }
        pending.push(current.id.as_str());
        let Some(source_id) = inheritance_source(current, property) else {
            break StylePropertyResolution::Origin(current.id.as_str());
        };
        // A known target without presentation Style cannot provide inheritance;
        // this view's own value remains the origin for its descendants.
        if non_views.contains(source_id) {
            break StylePropertyResolution::Origin(current.id.as_str());
        }
        let Some(source) = views.get(source_id).copied() else {
            break StylePropertyResolution::Unresolved(
                projection::StyleInheritanceDiagnostic::MissingSource {
                    source_id: source_id.to_owned(),
                },
            );
        };
        current = source;
    };
    for view_id in pending {
        resolved.insert((view_id, property), resolution.clone());
    }
    resolution
}

fn inheritance_source(view: &mde::View, property: mde::InheritedStyleProperty) -> Option<&str> {
    let inheritance = view.presentation.style_inheritance.as_ref()?;
    // A cycle in the reference graph is harmless when the masks do not
    // propagate this property around it.
    if !inheritance.properties.contains(&property) {
        return None;
    }
    inheritance.source.value().map(String::as_str)
}

fn inherited_property(property: mde::InheritedStyleProperty) -> projection::InheritedStyleProperty {
    property
}

fn authored_render_style(view: &mde::View) -> Result<ProjectedRenderStyle, String> {
    let mut style = render_style(&view.style)?;
    if let Some(defaults) = &view.presentation.default_style {
        style.fill = style.fill.or_else(|| defaults.fill.clone());
        style.stroke = style.stroke.or_else(|| defaults.stroke.clone());
        style.text_color = style.text_color.or_else(|| defaults.text_color.clone());
        style.stroke_width = style.stroke_width.or(defaults.stroke_width);
        style.text_align = style.text_align.or(defaults.text_align);
        style.text_vertical_align = style.text_vertical_align.or(defaults.text_vertical_align);
        style.show_shadow = style.show_shadow.or(defaults.show_shadow);
    }
    Ok(ProjectedRenderStyle {
        style,
        inheritance: view
            .presentation
            .style_inheritance
            .as_ref()
            .map(|inheritance| projection::StyleInheritance {
                source: match &inheritance.source {
                    mde::Authored::Missing => projection::StyleInheritanceSource::Missing,
                    mde::Authored::Null => projection::StyleInheritanceSource::Null,
                    mde::Authored::Value(source) => {
                        projection::StyleInheritanceSource::Reference(source.clone())
                    }
                },
                properties: inheritance
                    .properties
                    .iter()
                    .copied()
                    .map(inherited_property)
                    .collect(),
                status: projection::StyleInheritanceStatus::Inactive,
                origins: BTreeMap::new(),
                diagnostics: BTreeMap::new(),
            }),
    })
}

fn copy_inherited_property(
    destination: &mut RenderStyle,
    source: &RenderStyle,
    property: mde::InheritedStyleProperty,
) {
    match property {
        mde::InheritedStyleProperty::Fill => destination.fill.clone_from(&source.fill),
        mde::InheritedStyleProperty::Stroke => destination.stroke.clone_from(&source.stroke),
        mde::InheritedStyleProperty::TextColor => {
            destination.text_color.clone_from(&source.text_color)
        }
        mde::InheritedStyleProperty::FontFamily => {
            destination.font_family.clone_from(&source.font_family)
        }
        mde::InheritedStyleProperty::FontSize => destination.font_size = source.font_size,
        mde::InheritedStyleProperty::FontWeight => destination.font_weight = source.font_weight,
        mde::InheritedStyleProperty::FontItalic => destination.font_italic = source.font_italic,
    }
}

fn render_style(style: &mde::Style) -> Result<RenderStyle, String> {
    let custom_pattern = !style.dash_pattern.is_empty()
        && style
            .dash_pattern
            .iter()
            .all(|value| value.is_finite() && *value > 0.0 && *value <= 1000.0);
    Ok(RenderStyle {
        fill: style.fill.clone(),
        stroke: style.stroke.clone(),
        stroke_width: style.stroke_width,
        font_family: style.font_family.clone(),
        font_size: style.font_size,
        font_weight: style.font_weight,
        font_italic: style.font_italic,
        font_underline: style.font_underline,
        opacity: style.opacity,
        text_color: style.text_color.clone(),
        text_align: style.text_align,
        text_vertical_align: style.text_vertical_align,
        corner_radius: style.corner_radius,
        stroke_pattern: (style.line_mode.is_some() || custom_pattern)
            .then(|| style.resolve_stroke_pattern(mde::LineMode::Solid).to_vec()),
        word_wrap: style.word_wrap,
        show_shadow: style.show_shadow,
        shadow: None,
        show_stereotype: style.uml.should_show_stereotype(),
        show_attributes: style.uml.suppress_attributes != Some(true),
        show_operations: style.uml.suppress_operations != Some(true),
        show_receptions: style.uml.suppress_receptions != Some(true),
    })
}

fn figure_presentation(
    section: &mde::ModelingSection,
    diagram: &mde::Diagram,
    view: &mde::View,
    element: Option<&mde::Element>,
    references: &mde::uml::references::ReferenceIndex<'_>,
    timing: Option<TimingPresentation>,
) -> Result<FigurePresentation, String> {
    let empty = BTreeMap::new();
    let (semantic_kind, properties) = if let Some(element) = element {
        (element.kind.as_str(), &element.properties)
    } else if let Some(draft) = &view.draft {
        (draft.notation_kind.as_str(), &draft.properties)
    } else {
        ("", &empty)
    };
    let kind = view
        .presentation
        .notation_kind
        .as_deref()
        .unwrap_or(semantic_kind);
    let pseudostate_kind = properties
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("initial");
    let diagram_context = view
        .subject
        .as_ref()
        .is_some_and(|subject| diagram.model_elements.contains(subject));
    let context_frame = if diagram_context {
        match (kind, diagram.uml_di_type.as_str()) {
            ("uml.activity", "UMLActivityDiagram") => Some("act"),
            ("uml.state_machine", "UMLStateMachineDiagram") => Some("stm"),
            _ => None,
        }
    } else {
        None
    };
    let interaction = (diagram.uml_di_type == "UMLInteractionDiagram")
        .then_some(diagram.interaction_kind.as_deref())
        .flatten();
    let notation = if context_frame.is_some() {
        mde::UmlNotation::Frame
    } else if kind == "uml.lifeline" && interaction == Some("communication") {
        mde::UmlNotation::Instance
    } else if kind == "uml.pseudostate" {
        mde::resolve_pseudostate_notation(pseudostate_kind)
    } else {
        mde::uml_notation(kind)
    };
    let applied =
        element.and_then(|element| mde::stereotype_presentation(section, element).label());
    let mut header = mde::uml_header(kind, applied);
    if !view.style.uml.should_show_stereotype() {
        header.applied_stereotype = None;
    }
    let header_height = header.classifier_header_height();
    let interface = if element.is_none() && kind == "uml.interface" {
        match properties
            .get("interface_presentation")
            .and_then(Value::as_str)
        {
            Some("provided") => Some(InterfacePresentation::Provided),
            Some("required") => Some(InterfacePresentation::Required),
            _ => None,
        }
    } else {
        None
    };
    let timing = if kind == "uml.lifeline" && interaction == Some("timing") {
        Some(timing.unwrap_or(TimingPresentation {
            mode: TimingMode::State,
            points: Vec::new(),
        }))
    } else {
        None
    };
    Ok(FigurePresentation {
        notation: notation_kind(notation),
        header: UmlHeader {
            keyword: header.keyword.map(str::to_owned),
            applied_stereotype: header.applied_stereotype,
            classifier_header_height: header_height,
        },
        frame_kind: context_frame
            .or_else(|| (kind == "uml.interaction").then_some("sd"))
            .map(str::to_owned),
        title_underlined: notation == mde::UmlNotation::Instance && kind != "uml.lifeline",
        fixed_message_anchor_x: (kind == "uml.lifeline"
            && diagram
                .interaction_kind
                .as_deref()
                .is_none_or(|kind| kind == "sequence"))
        .then_some(0.5),
        resizable: element.is_none() || mde::UmlType::from_kind(kind) != Some(mde::UmlType::Port),
        interface,
        timing,
        history_label: match pseudostate_kind {
            "deepHistory" => "H*",
            "entryPoint" => "",
            _ => "H",
        }
        .to_owned(),
        composite_role: properties.get("aggregation").and_then(Value::as_str) == Some("composite"),
        interaction_operator: if kind == "uml.combined_fragment" {
            interaction_operator(properties)?
        } else {
            None
        },
        slot_labels: element
            .map(|element| mde::collect_instance_slot_labels(element, references))
            .unwrap_or_default(),
    })
}

fn interaction_operator(properties: &BTreeMap<String, Value>) -> Result<Option<String>, String> {
    if let Some(value) = properties.get("interactionOperator") {
        return Ok(value.as_str().map(str::to_owned));
    }
    use mde::uml::schema;
    let property = schema::uml_property("CombinedFragment", "interactionOperator")
        .map_err(|error| error.to_string())?;
    let value = property
        .map(schema::effective_default)
        .transpose()
        .map_err(|error| error.to_string())?
        .flatten()
        .and_then(schema::DefaultValue::scalar_json);
    Ok(value.and_then(|value| value.as_str().map(str::to_owned)))
}

fn figure_label(
    element: &mde::Element,
    references: &mde::uml::references::ReferenceIndex<'_>,
    style: &mde::Style,
) -> String {
    match element.kind.as_str() {
        "uml.interaction_operand" => mde::format_operand_guard(element, references),
        "uml.constraint" => format!("{{{}}}", element.name),
        "uml.port"
        | "uml.part"
        | "uml.note"
        | "uml.comment"
        | "uml.instance_specification"
        | "uml.collaboration_use" => mde::format_element_label_for_view(element, references, style),
        _ => element.name.clone(),
    }
}

pub(crate) fn relationship_labels(
    presentation: &mde::RelationshipPresentation,
) -> RelationshipPresentation {
    RelationshipPresentation {
        name: presentation.name.clone(),
        source_labels: RelationshipEndpointLabels {
            role: presentation.source_labels.role.clone(),
            multiplicity: presentation.source_labels.multiplicity.clone(),
        },
        target_labels: RelationshipEndpointLabels {
            role: presentation.target_labels.role.clone(),
            multiplicity: presentation.target_labels.multiplicity.clone(),
        },
        type_label: mde::semantic_identity(&presentation.notation)
            .type_label
            .to_owned(),
    }
}

fn relationship_style(style: mde::RelationshipStyle) -> RelationshipStyle {
    style
}

fn marker_composition(composition: &mde::EndpointMarkerComposition) -> EndpointMarkerComposition {
    composition.clone()
}

fn notation_kind(notation: mde::UmlNotation) -> UmlNotation {
    notation
}
