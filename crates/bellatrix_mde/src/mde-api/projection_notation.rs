use bellatrix_mde_protocol::command::{LineMode, TimingMode, TimingPresentation};
use bellatrix_mde_protocol::presentation::*;
use crate::mde_api_projection as projection;
use crate as mde;
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) fn project_view(
    section: &mde::ModelingSection,
    diagram: &mde::Diagram,
    view: &mde::View,
    references: &mde::uml::references::ReferenceIndex<'_>,
    paths: &BTreeMap<String, mde::Geometry>,
) -> Result<projection::View, String> {
    let element = view.subject.as_deref().and_then(|subject| {
        section.elements.iter().find(|element| element.id == subject)
    });
    let timing = view.timing.as_ref().map(projection::convert).transpose()?;
    let render_style = render_style(&view.style)?;
    let figure = figure_presentation(section, diagram, view, element, references, timing.clone())?;
    let classifier = element.map(|element| {
        let presentation = mde::classifier_presentation(
            element,
            mde::ClassifierPresentationOptions { show_active_class_sides: true },
        );
        ClassifierPresentation {
            name_is_italic: presentation.name_is_italic,
            double_sided: presentation.double_sided,
        }
    }).unwrap_or_default();
    let display_label = element.map(|element| figure_label(element, references, &view.style))
        .unwrap_or_else(|| view.label.clone().unwrap_or_default());
    let member_labels = element.map(|owner| {
        section.relations.iter()
            .filter(|relation| relation.kind == "owns" && relation.from == owner.id)
            .filter_map(|relation| section.elements.iter().find(|member| member.id == relation.to))
            .map(|member| (
                member.id.clone(),
                mde::format_element_label_for_view(member, references, &view.style),
            ))
            .collect()
    }).unwrap_or_default();
    let relationship = if let Some(resolved) = mde::resolve_relationship_view(section, view) {
        Some(projection::ResolvedRelationship {
            communication_message: resolved.kind == "uml.message"
                && diagram.interaction_kind.as_deref() == Some("communication"),
            kind: resolved.kind,
            presentation: relationship_labels(&resolved.presentation),
            style: relationship_style(resolved.presentation.style),
            markers: resolved.markers.as_ref().map(|(source, target)| (
                marker_composition(source), marker_composition(target),
            )),
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
                type_label: mde::semantic_identity(&draft.notation_kind).type_label.to_owned(),
            },
            style: relationship_style(mde::resolve_relationship_style(
                &draft.notation_kind, &draft.properties,
            )),
            markers: None,
            diagnostics: Vec::new(),
            communication_message: draft.notation_kind == "uml.message"
                && diagram.interaction_kind.as_deref() == Some("communication"),
        })
    } else {
        None
    };
    let routed_geometry = if view.style.routing == Some(mde::RelationshipRouting::Manual) {
        None
    } else {
        paths.get(&view.id).map(projection::convert).transpose()?
    };
    Ok(projection::View {
        id: view.id.clone(),
        subject: view.subject.clone(),
        draft: view.draft.as_ref().map(|draft| projection::VisualDraftSummary {
            notation_kind: draft.notation_kind.clone(),
            parent_view_id: draft.parent_view_id.clone(),
        }),
        endpoints: view.endpoints.as_ref().map(projection::convert).transpose()?,
        timing,
        label: view.label.clone(),
        geometry: projection::convert(&view.geometry)?,
        children: view.children.iter().map(|child| {
            project_view(section, diagram, child, references, paths)
        }).collect::<Result<_, _>>()?,
        presentation: projection::ViewPresentation {
            kind: view.presentation.kind.clone(),
            has_source: view.presentation.source.is_some(),
            visible: visible_in_preview(view),
        },
        style: projection::convert(&view.style)?,
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

fn visible_in_preview(view: &mde::View) -> bool {
    let Some(source) = view.presentation.source.as_ref().filter(|source| source.format == "mdj") else {
        return true;
    };
    match source.residual.get("visible") {
        None => true,
        Some(Value::Bool(visible)) => *visible,
        Some(_) => false,
    }
}

fn render_style(style: &mde::Style) -> Result<RenderStyle, String> {
    let custom_pattern = !style.dash_pattern.is_empty()
        && style.dash_pattern.iter().all(|value| value.is_finite() && *value > 0.0 && *value <= 1000.0);
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
        text_align: style.text_align.as_ref().map(projection::convert).transpose()?,
        text_vertical_align: style.text_vertical_align.as_ref().map(projection::convert).transpose()?,
        corner_radius: style.corner_radius,
        stroke_pattern: (style.line_mode.is_some() || custom_pattern)
            .then(|| style.resolve_stroke_pattern(mde::LineMode::Solid).to_vec()),
        word_wrap: style.word_wrap,
        show_shadow: style.show_shadow,
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
    let (kind, properties) = if let Some(element) = element {
        (element.kind.as_str(), &element.properties)
    } else if let Some(draft) = &view.draft {
        (draft.notation_kind.as_str(), &draft.properties)
    } else {
        ("", &empty)
    };
    let pseudostate_kind = properties.get("kind").and_then(Value::as_str).unwrap_or("initial");
    let diagram_context = view.subject.as_ref().is_some_and(|subject| diagram.model_elements.contains(subject));
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
        .then_some(diagram.interaction_kind.as_deref()).flatten();
    let notation = if context_frame.is_some() {
        mde::UmlNotation::Frame
    } else if kind == "uml.lifeline" && interaction == Some("communication") {
        mde::UmlNotation::Instance
    } else if kind == "uml.pseudostate" {
        mde::resolve_pseudostate_notation(pseudostate_kind)
    } else {
        mde::uml_notation(kind)
    };
    let applied = element.and_then(|element| mde::stereotype_presentation(section, element).label());
    let mut header = mde::uml_header(kind, applied);
    if !view.style.uml.should_show_stereotype() {
        header.applied_stereotype = None;
    }
    let header_height = header.classifier_header_height();
    let interface = if element.is_none() && kind == "uml.interface" {
        match properties.get("interface_presentation").and_then(Value::as_str) {
            Some("provided") => Some(InterfacePresentation::Provided),
            Some("required") => Some(InterfacePresentation::Required),
            _ => None,
        }
    } else {
        None
    };
    let timing = if kind == "uml.lifeline" && interaction == Some("timing") {
        Some(timing.unwrap_or(TimingPresentation { mode: TimingMode::State, points: Vec::new() }))
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
        frame_kind: context_frame.or_else(|| (kind == "uml.interaction").then_some("sd")).map(str::to_owned),
        title_underlined: notation == mde::UmlNotation::Instance && kind != "uml.lifeline",
        fixed_message_anchor_x: (kind == "uml.lifeline"
            && diagram.interaction_kind.as_deref().is_none_or(|kind| kind == "sequence"))
            .then_some(0.5),
        resizable: element.is_none() || mde::UmlType::from_kind(kind) != Some(mde::UmlType::Port),
        interface,
        timing,
        history_label: match pseudostate_kind {
            "deepHistory" => "H*",
            "entryPoint" => "",
            _ => "H",
        }.to_owned(),
        composite_role: properties.get("aggregation").and_then(Value::as_str) == Some("composite"),
        interaction_operator: if kind == "uml.combined_fragment" {
            interaction_operator(properties)?
        } else {
            None
        },
        slot_labels: element.map(|element| mde::collect_instance_slot_labels(element, references)).unwrap_or_default(),
    })
}

fn interaction_operator(properties: &BTreeMap<String, Value>) -> Result<Option<String>, String> {
    if let Some(value) = properties.get("interactionOperator") {
        return Ok(value.as_str().map(str::to_owned));
    }
    use mde::uml::schema;
    let property = schema::uml_property("CombinedFragment", "interactionOperator")
        .map_err(|error| error.to_string())?;
    let value = property.map(schema::effective_default).transpose()
        .map_err(|error| error.to_string())?.flatten()
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
        "uml.port" | "uml.part" | "uml.note" | "uml.comment"
        | "uml.instance_specification" | "uml.collaboration_use" => {
            mde::format_element_label_for_view(element, references, style)
        }
        _ => element.name.clone(),
    }
}

pub(crate) fn relationship_labels(presentation: &mde::RelationshipPresentation) -> RelationshipPresentation {
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
        type_label: mde::semantic_identity(&presentation.notation).type_label.to_owned(),
    }
}

fn relationship_style(style: mde::RelationshipStyle) -> RelationshipStyle {
    RelationshipStyle {
        source_marker: relationship_marker(style.source_marker),
        target_marker: relationship_marker(style.target_marker),
        middle_marker: relationship_marker(style.middle_marker),
        line_mode: match style.line_mode {
            mde::LineMode::Solid => LineMode::Solid,
            mde::LineMode::Dotted => LineMode::Dotted,
            mde::LineMode::Dashed => LineMode::Dashed,
        },
    }
}

fn relationship_marker(marker: mde::RelationshipMarker) -> RelationshipMarker {
    match marker {
        mde::RelationshipMarker::None => RelationshipMarker::None,
        mde::RelationshipMarker::OpenArrow => RelationshipMarker::OpenArrow,
        mde::RelationshipMarker::Triangle => RelationshipMarker::Triangle,
        mde::RelationshipMarker::Diamond => RelationshipMarker::Diamond,
        mde::RelationshipMarker::FilledDiamond => RelationshipMarker::FilledDiamond,
        mde::RelationshipMarker::FilledTriangle => RelationshipMarker::FilledTriangle,
        mde::RelationshipMarker::FilledCircle => RelationshipMarker::FilledCircle,
        mde::RelationshipMarker::OpenArrowAndCircle => RelationshipMarker::OpenArrowAndCircle,
        mde::RelationshipMarker::FilledTriangleAndCircle => RelationshipMarker::FilledTriangleAndCircle,
        mde::RelationshipMarker::Containment => RelationshipMarker::Containment,
    }
}

fn marker_composition(composition: &mde::EndpointMarkerComposition) -> EndpointMarkerComposition {
    EndpointMarkerComposition {
        glyphs: composition.glyphs.iter().map(|part| EndpointMarkerPart {
            glyph: match part.glyph {
                mde::EndpointMarkerGlyph::OpenArrow => EndpointMarkerGlyph::OpenArrow,
                mde::EndpointMarkerGlyph::SolidArrow => EndpointMarkerGlyph::SolidArrow,
                mde::EndpointMarkerGlyph::BlankArrow => EndpointMarkerGlyph::BlankArrow,
                mde::EndpointMarkerGlyph::Cross => EndpointMarkerGlyph::Cross,
                mde::EndpointMarkerGlyph::OpenDiamond => EndpointMarkerGlyph::OpenDiamond,
                mde::EndpointMarkerGlyph::FilledDiamond => EndpointMarkerGlyph::FilledDiamond,
                mde::EndpointMarkerGlyph::OpenTriangle => EndpointMarkerGlyph::OpenTriangle,
                mde::EndpointMarkerGlyph::FilledTriangle => EndpointMarkerGlyph::FilledTriangle,
                mde::EndpointMarkerGlyph::OpenCircle => EndpointMarkerGlyph::OpenCircle,
                mde::EndpointMarkerGlyph::FilledCircle => EndpointMarkerGlyph::FilledCircle,
                mde::EndpointMarkerGlyph::Dot => EndpointMarkerGlyph::Dot,
                mde::EndpointMarkerGlyph::Containment => EndpointMarkerGlyph::Containment,
            },
            placement: match part.placement {
                mde::EndpointMarkerPlacement::Endpoint => EndpointMarkerPlacement::Endpoint,
                mde::EndpointMarkerPlacement::AfterPrevious => EndpointMarkerPlacement::AfterPrevious,
            },
        }).collect(),
    }
}

fn notation_kind(notation: mde::UmlNotation) -> UmlNotation {
    match notation {
        mde::UmlNotation::ClassifierRectangle => UmlNotation::ClassifierRectangle,
        mde::UmlNotation::PackageTab => UmlNotation::PackageTab,
        mde::UmlNotation::FoldedNote => UmlNotation::FoldedNote,
        mde::UmlNotation::Frame => UmlNotation::Frame,
        mde::UmlNotation::AssociationDiamond => UmlNotation::AssociationDiamond,
        mde::UmlNotation::Port => UmlNotation::Port,
        mde::UmlNotation::Part => UmlNotation::Part,
        mde::UmlNotation::Actor => UmlNotation::Actor,
        mde::UmlNotation::UseCase => UmlNotation::UseCase,
        mde::UmlNotation::DeploymentNode => UmlNotation::DeploymentNode,
        mde::UmlNotation::Collaboration => UmlNotation::Collaboration,
        mde::UmlNotation::Instance => UmlNotation::Instance,
        mde::UmlNotation::Action => UmlNotation::Action,
        mde::UmlNotation::State => UmlNotation::State,
        mde::UmlNotation::InitialNode => UmlNotation::InitialNode,
        mde::UmlNotation::FinalNode => UmlNotation::FinalNode,
        mde::UmlNotation::FlowFinal => UmlNotation::FlowFinal,
        mde::UmlNotation::Decision => UmlNotation::Decision,
        mde::UmlNotation::Fork => UmlNotation::Fork,
        mde::UmlNotation::Lifeline => UmlNotation::Lifeline,
        mde::UmlNotation::Execution => UmlNotation::Execution,
        mde::UmlNotation::InteractionUse => UmlNotation::InteractionUse,
        mde::UmlNotation::CombinedFragment => UmlNotation::CombinedFragment,
        mde::UmlNotation::InteractionOperand => UmlNotation::InteractionOperand,
        mde::UmlNotation::ObjectNode => UmlNotation::ObjectNode,
        mde::UmlNotation::SendSignal => UmlNotation::SendSignal,
        mde::UmlNotation::AcceptEvent => UmlNotation::AcceptEvent,
        mde::UmlNotation::Partition => UmlNotation::Partition,
        mde::UmlNotation::TimingState => UmlNotation::TimingState,
        mde::UmlNotation::History => UmlNotation::History,
        mde::UmlNotation::Terminate => UmlNotation::Terminate,
        mde::UmlNotation::Unsupported => UmlNotation::Unsupported,
    }
}
