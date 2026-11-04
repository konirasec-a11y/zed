use super::mdj::Diagnostic;
use crate::{Geometry, Project, StereotypeDisplay, View};
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn normalize(project: &mut Project, diagnostics: &mut Vec<Diagnostic>) {
    let subjects = project
        .elements
        .iter()
        .map(|element| (element.id.clone(), element.kind.clone()))
        .collect::<BTreeMap<_, _>>();
    for diagram in &mut project.diagrams {
        normalize_views(&mut diagram.views, &subjects, false, diagnostics);
    }
}

fn normalize_views(
    views: &mut [View],
    subjects: &BTreeMap<String, String>,
    nested: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for view in views {
        let Some(source) = &mut view.presentation.source else {
            continue;
        };
        let source_kind = source.type_name.clone();
        if let Some(visible) = source.residual.get("visible").and_then(Value::as_bool) {
            view.presentation.visible = Some(visible);
            source.residual.remove("visible");
        } else if source.residual.contains_key("visible") {
            view.presentation.visible = Some(false);
            diagnostics.push(Diagnostic {
                path: view.id.clone(),
                message: "unsupported visibility value is preserved; the unresolved occurrence remains hidden".into(),
            });
        }
        let notation = notation_kind(&source_kind);
        let semantic_kind = view.subject.as_ref().and_then(|id| subjects.get(id));
        // A source View may have a supported silhouette while its model remains opaque.
        // Keep that distinction explicit rather than promoting unsupported UML semantics.
        if let Some(notation) = notation {
            if semantic_kind.is_none_or(|kind| kind != notation) {
                view.presentation.notation_kind = Some(notation.to_owned());
            }
        } else if !matches!(view.geometry, Geometry::Path { .. }) {
            view.presentation.notation_kind = Some("presentation.unsupported".into());
            diagnostics.push(Diagnostic {
                path: view.id.clone(),
                message: format!("{source_kind} has no native figure mapping; its object, layout and source data remain preserved"),
            });
        }
        view.presentation.internal = nested
            && matches!(
                notation,
                Some("presentation.label" | "presentation.compartment")
            );
        view.presentation.divider = matches!(
            source_kind.as_str(),
            "UMLAttributeCompartmentView"
                | "UMLOperationCompartmentView"
                | "UMLReceptionCompartmentView"
                | "UMLEnumerationLiteralCompartmentView"
                | "UMLSlotCompartmentView"
        );
        if let Geometry::Layout { bounds } = &view.geometry {
            let rectangle = bounds
                .x
                .value()
                .copied()
                .zip(bounds.y.value().copied())
                .zip(bounds.width.value().and_then(crate::LayoutExtent::resolved))
                .zip(
                    bounds
                        .height
                        .value()
                        .and_then(crate::LayoutExtent::resolved),
                );
            if let Some((((x, y), width), height)) = rectangle {
                if [x, y, width, height].iter().all(|value| value.is_finite())
                    && width >= 0.0
                    && height >= 0.0
                {
                    view.geometry = Geometry::Rect {
                        x,
                        y,
                        width,
                        height,
                    };
                }
            } else {
                diagnostics.push(Diagnostic {
                    path: view.id.clone(),
                    message: "native layout is unresolved; missing, null or unsupported source bounds were preserved instead of inventing coordinates".into(),
                });
            }
        }
        if source_kind == "UMLInterfaceView" {
            // The source constructor selects iconic notation when the field is absent.
            // A null or unsupported explicit value must not acquire that default.
            if view.style.uml.stereotype_display.is_none()
                && !source.residual.contains_key("stereotypeDisplay")
            {
                view.style.uml.stereotype_display = Some(StereotypeDisplay::Icon);
            } else if view.style.uml.stereotype_display.is_none() {
                view.presentation.notation_kind = Some("presentation.unsupported".into());
                diagnostics.push(Diagnostic {
                    path: view.id.clone(),
                    message: "unsupported interface form remains preserved; no icon or classifier form was substituted".into(),
                });
            }
        }
        let name_compartment = source
            .residual
            .get("nameCompartment")
            .and_then(reference)
            .map(str::to_owned)
            .or_else(|| {
                view.presentation
                    .interface_icon
                    .as_ref()
                    .and_then(|binding| binding.label_compartment.value())
                    .cloned()
            });
        let direct_name = source
            .residual
            .get("nameLabel")
            .and_then(reference)
            .map(str::to_owned);
        let entity_name_label = view
            .presentation
            .entity_box
            .as_ref()
            .and_then(|binding| binding.name_label.value())
            .cloned();
        let name_label = direct_name
            .or_else(|| {
                let compartment = view
                    .children
                    .iter()
                    .find(|child| Some(&child.id) == name_compartment.as_ref())?;
                reference(
                    compartment
                        .presentation
                        .source
                        .as_ref()?
                        .residual
                        .get("nameLabel")?,
                )
                .map(str::to_owned)
            })
            .or_else(|| entity_name_label.clone());
        if semantic_kind.is_some() {
            if let Some(name_label) = name_label {
                if let Some(label) = find_view(&mut view.children, &name_label) {
                    if let Some(text) = label.label.take() {
                        if let Some(source) = &mut label.presentation.source {
                            source.residual.insert("text".into(), Value::String(text));
                        }
                    }
                    if entity_name_label.as_ref() != Some(&name_label) {
                        view.presentation.name_label = Some(name_label);
                    }
                } else {
                    diagnostics.push(Diagnostic {
                        path: view.id.clone(),
                        message: "name label reference does not identify an owned occurrence; source reference was preserved".into(),
                    });
                }
            }
        }
        normalize_views(&mut view.children, subjects, true, diagnostics);
    }
}

fn find_view<'a>(views: &'a mut [View], id: &str) -> Option<&'a mut View> {
    for view in views {
        if view.id == id {
            return Some(view);
        }
        if let Some(found) = find_view(&mut view.children, id) {
            return Some(found);
        }
    }
    None
}

fn reference(value: &Value) -> Option<&str> {
    value
        .as_object()
        .filter(|object| object.len() == 1)?
        .get("$ref")?
        .as_str()
}

fn notation_kind(source: &str) -> Option<&'static str> {
    Some(match source {
        "RectangleView" => "presentation.rectangle",
        "LabelView"
        | "NodeLabelView"
        | "EdgeLabelView"
        | "UMLAttributeView"
        | "UMLOperationView"
        | "UMLReceptionView"
        | "UMLTemplateParameterView"
        | "UMLEnumerationLiteralView"
        | "UMLSlotView" => "presentation.label",
        "UMLCompartmentView"
        | "UMLNameCompartmentView"
        | "UMLAttributeCompartmentView"
        | "UMLOperationCompartmentView"
        | "UMLReceptionCompartmentView"
        | "UMLTemplateParameterCompartmentView"
        | "UMLEnumerationLiteralCompartmentView"
        | "UMLQualifierCompartmentView"
        | "UMLSlotCompartmentView" => "presentation.compartment",
        "UMLClassView" | "UMLMetaClassView" => "uml.class",
        "UMLInterfaceView" => "uml.interface",
        "UMLStereotypeView" => "uml.stereotype",
        "UMLModelView" => "uml.model",
        "UMLPackageView" => "uml.package",
        "UMLProfileView" => "uml.profile",
        "UMLActorView" => "uml.actor",
        "UMLUseCaseView" => "uml.use_case",
        "UMLComponentView" => "uml.component",
        "UMLArtifactView" => "uml.artifact",
        "UMLNodeView" => "uml.node",
        "UMLSignalView" => "uml.signal",
        "UMLDataTypeView" => "uml.data_type",
        "UMLPrimitiveTypeView" => "uml.primitive_type",
        "UMLEnumerationView" => "uml.enumeration",
        "UMLNoteView" => "uml.note",
        "UMLConstraintView" => "uml.constraint",
        "UMLFrameView" => "uml.frame",
        "UMLPortView" => "uml.port",
        "UMLPartView" => "uml.part",
        "UMLObjectView" => "uml.instance_specification",
        "UMLCollaborationView" => "uml.collaboration",
        "UMLStateView" => "uml.state",
        "UMLPseudostateView" => "uml.pseudostate",
        "UMLFinalStateView" => "uml.final_state",
        "UMLActionView" => "uml.opaque_action",
        "UMLInitialNodeView" => "uml.initial_node",
        "UMLActivityFinalNodeView" => "uml.activity_final_node",
        "UMLFlowFinalNodeView" => "uml.flow_final_node",
        "UMLDecisionNodeView" => "uml.decision_node",
        "UMLMergeNodeView" => "uml.merge_node",
        "UMLForkNodeView" => "uml.fork_node",
        "UMLJoinNodeView" => "uml.join_node",
        "UMLLifelineView" => "uml.lifeline",
        "UMLActivationView" => "uml.behavior_execution_specification",
        "UMLCombinedFragmentView" => "uml.combined_fragment",
        "UMLInteractionOperandView" => "uml.interaction_operand",
        "UMLInteractionUseView" => "uml.interaction_use",
        "UMLAssociationView" => "uml.association",
        "UMLGeneralizationView" => "uml.generalization",
        "UMLDependencyView" => "uml.dependency",
        "UMLRealizationView" => "uml.realization",
        "UMLInterfaceRealizationView" => "uml.interface_realization",
        "UMLExtensionView" => "uml.extension",
        "ERDEntityView" => "erd.entity",
        "ERDRelationshipView" => "erd.relationship",
        _ => return None,
    })
}
