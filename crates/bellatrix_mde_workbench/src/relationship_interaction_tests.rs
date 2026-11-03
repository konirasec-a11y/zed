use super::*;
#[test]
fn two_clicks_connect_distinct_model_elements() {
    let mut pending = None;
    assert_eq!(
        choose_relation_endpoint(&mut pending, "type-list"),
        RelationClick::First
    );
    assert_eq!(pending.as_deref(), Some("type-list"));
    assert_eq!(
        choose_relation_endpoint(&mut pending, "type-task"),
        RelationClick::Connect("type-list".to_owned())
    );
    assert_eq!(pending, None);
}
#[test]
fn clicking_source_again_does_not_drop_armed_tool() {
    let mut pending = None;
    assert_eq!(
        choose_relation_endpoint(&mut pending, "type-list"),
        RelationClick::First
    );
    assert_eq!(
        choose_relation_endpoint(&mut pending, "type-list"),
        RelationClick::Repeat
    );
    assert_eq!(pending.as_deref(), Some("type-list"));
    assert_eq!(
        choose_relation_endpoint(&mut pending, "type-task"),
        RelationClick::Connect("type-list".into())
    );
}

#[test]
fn switching_diagrams_clears_selection_but_reopening_same_diagram_preserves_it() {
    let mut active = Some("first".to_owned());
    let mut selected = Some("class".to_owned());
    switch_diagram_selection(&mut active, &mut selected, "first".to_owned());
    assert_eq!(selected.as_deref(), Some("class"));
    switch_diagram_selection(&mut active, &mut selected, "second".to_owned());
    assert_eq!(active.as_deref(), Some("second"));
    assert_eq!(selected, None);
}

#[test]
fn canvas_placement_inverts_window_origin_pan_and_zoom() {
    for zoom in [0.25_f32, 1.0, 3.0] {
        let origin = (246.0, 80.0);
        let pan = (-40.0_f32, 65.0_f32);
        let expected = (128.0, 256.0);
        let position = (
            origin.0 + f64::from(pan.0) + expected.0 * f64::from(zoom),
            origin.1 + f64::from(pan.1) + expected.1 * f64::from(zoom),
        );
        assert_eq!(canvas_model_position(position, origin, pan, zoom), expected);
    }
}

#[test]
fn diagram_creation_context_is_only_forwarded_for_composite_structure() {
    for kind in bellatrix_mde::StructuralDiagramKind::ALL {
        let context = diagram_creation_context(kind, Some("selected-model-element"));
        if kind == bellatrix_mde::StructuralDiagramKind::CompositeStructure {
            assert_eq!(context, vec!["selected-model-element".to_owned()]);
        } else {
            assert!(context.is_empty());
        }
        assert!(diagram_creation_context(kind, None).is_empty());
        assert!(!diagram_kind_label(kind).is_empty());
    }
}

#[test]
fn validation_requires_known_explicit_metadata() {
    for kind in bellatrix_mde::StructuralDiagramKind::ALL {
        assert_eq!(diagram_validation_kind(kind.uml_di_type()), Ok(kind));
    }
    assert!(diagram_validation_kind("").is_err());
    assert!(diagram_validation_kind("  ").is_err());
    assert!(diagram_validation_kind("UMLFutureDiagram").is_err());
}

#[test]
fn ordinary_delete_requires_an_explicit_view_and_diagram() {
    assert_eq!(
        selected_view_target(Some("diagram"), Some("view")),
        Some(("diagram".into(), "view".into()))
    );
    assert!(selected_view_target(Some("diagram"), None).is_none());
    assert!(selected_view_target(None, Some("view")).is_none());
}

#[test]
fn native_delete_actions_never_handle_editor_focus_or_inline_editing() {
    assert!(canvas_delete_action_allowed(true, false));
    assert!(!canvas_delete_action_allowed(false, false));
    assert!(!canvas_delete_action_allowed(true, true));
    assert!(!canvas_delete_action_allowed(false, true));
}

#[test]
fn show_element_reuses_existing_view_without_changing_diagram() -> serde_json::Result<()> {
    let diagram: bellatrix_mde::Diagram = serde_json::from_value(serde_json::json!({
        "id": "active", "name": "Active", "uml_di_type": "UMLClassDiagram", "views": [
            {"id": "existing", "subject": "element", "geometry": {"kind": "rect", "x": 0.0, "y": 0.0, "width": 180.0, "height": 112.0}}
        ]
    }))?;
    let before = diagram.clone();
    assert_eq!(
        element_view_placement(&diagram, "element"),
        ElementViewPlacement::Existing("existing".into())
    );
    assert_eq!(
        element_view_placement(&diagram, "other"),
        ElementViewPlacement::NewAt(224.0, 36.0)
    );
    assert_eq!(diagram, before);
    Ok(())
}

#[test]
fn new_view_grid_uses_active_diagram_view_count() {
    assert_eq!(next_diagram_grid_position(0), (24.0, 36.0));
    assert_eq!(next_diagram_grid_position(4), (24.0, 184.0));
    assert_eq!(next_diagram_grid_position(5), (224.0, 184.0));
}
#[test]
fn structural_creation_forwards_explicit_selection_as_owner() {
    for kind in ModelingElementKind::STRUCTURAL {
        assert_eq!(
            creation_owner(*kind, Some("hidden-package")),
            Some("hidden-package".into())
        );
        assert_eq!(creation_owner(*kind, None), None);
    }
    assert_eq!(
        creation_owner(ModelingElementKind::Class, Some("selected")),
        None
    );
}
