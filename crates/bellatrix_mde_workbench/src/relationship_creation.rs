use super::*;

#[derive(Clone)]
pub(super) struct PendingRelationEndpoint {
    project_id: ProjectId,
    diagram_id: String,
    view_id: String,
    subject_id: Option<String>,
    revision: u64,
    point: Option<bellatrix_mde::Point>,
}

impl MdeWorkbenchItem {
    pub(super) fn connect_relation_view(
        &mut self,
        kind: ModelingRelationKind,
        target: String,
        point: Option<bellatrix_mde::Point>,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_read_only() {
            return;
        }
        let snapshot = match self.source.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.feedback = error;
                cx.notify();
                return;
            }
        };
        let Some(diagram) = snapshot
            .section
            .diagrams
            .iter()
            .find(|diagram| Some(diagram.id.as_str()) == self.active_diagram.as_deref())
        else {
            self.feedback = "Select a diagram before connecting Views".into();
            cx.notify();
            return;
        };
        let Some(target_view) = diagram.views.iter().find(|view| view.id == target) else {
            self.feedback = "The clicked View no longer exists in this diagram".into();
            cx.notify();
            return;
        };
        if self.pending_relation_source.is_none() {
            self.pending_relation_source = Some(PendingRelationEndpoint {
                project_id: self.project_id().clone(),
                diagram_id: diagram.id.clone(),
                view_id: target.clone(),
                subject_id: target_view.subject.clone(),
                revision: snapshot.revision,
                point,
            });
            self.selected_element = target_view.subject.clone();
            self.selected_view = Some(target);
            self.feedback = format!(
                "{}: source View selected. Click the target View",
                kind.label()
            );
            cx.notify();
            return;
        }
        let Some(source) = self.pending_relation_source.clone() else {
            return;
        };
        if source.project_id != *self.project_id()
            || source.diagram_id != diagram.id
            || source.revision != snapshot.revision
        {
            self.pending_relation_source = None;
            self.feedback =
                "The diagram changed after the source click. Select the source View again".into();
            cx.notify();
            return;
        }
        let Some(source_view) = diagram
            .views
            .iter()
            .find(|view| view.id == source.view_id && view.subject == source.subject_id)
        else {
            self.feedback = "The original source View changed; select the source again".into();
            self.pending_relation_source = None;
            cx.notify();
            return;
        };
        let from = source.view_id.clone();
        self.selected_element = target_view.subject.clone();
        self.selected_view = Some(target.clone());
        if !self.visual_draft_mode && !kind.is_visual_only() {
            self.submit(
                MdeUiCommand::CreateRelationAtViewsOrVisualDraft(
                    bellatrix_mde_cordis::ModelingCreateRelationAtViewsRequested {
                        project_id: self.project_id().clone(),
                        diagram_id: diagram.id.clone(),
                        kind,
                        source_view_id: from,
                        target_view_id: target,
                        from: source_view.subject.clone(),
                        to: target_view.subject.clone(),
                        expected_revision: source.revision,
                    },
                ),
                cx,
            );
            self.pending_relation_source = None;
            self.selected_tool = None;
            cx.notify();
            return;
        }
        let locate = |id: &str| {
            diagram
                .views
                .iter()
                .find(|view| view.id == id)
                .and_then(|view| bellatrix_mde::resolve_visual_center(&view.geometry))
        };
        let (Some(start), Some(to)) = (locate(&from), locate(&target)) else {
            self.feedback = "The selected Views do not have usable connection geometry".into();
            cx.notify();
            return;
        };
        let mut properties = std::collections::BTreeMap::new();
        let source_point = source.point;
        if kind.uses_occurrence_anchors() {
            for (key, view_id, point) in [
                ("source_anchor", from.as_str(), source_point),
                ("target_anchor", target.as_str(), point),
            ] {
                if diagram.interaction_kind.as_deref() != Some("communication")
                    && let Some(anchor) = point.and_then(|point| {
                        crate::visual_draft_canvas::resolve_message_anchor(&snapshot.section, diagram, view_id, point)
                    })
                {
                    properties.insert(key.into(), serde_json::json!({"x":anchor.x,"y":anchor.y}));
                }
            }
        }
        let label = if let Some(sort) = kind.resolve_message_sort() {
            properties.insert("messageSort".into(), serde_json::json!(sort));
            Some(
                if diagram.interaction_kind.as_deref() == Some("communication") {
                    "1: message()".to_owned()
                } else {
                    "message()".to_owned()
                },
            )
        } else {
            None
        };
        self.submit(
            MdeUiCommand::CreateVisualDraft(
                bellatrix_mde_cordis::ModelingCreateVisualDraftRequested {
                    project_id: self.project_id().clone(),
                    diagram_id: diagram.id.clone(),
                    notation_kind: kind.model_kind().into(),
                    label,
                    geometry: Geometry::Path {
                        start,
                        segments: vec![PathSegment::Line { to }],
                    },
                    properties,
                    parent_view_id: None,
                    source_view_id: Some(from),
                    target_view_id: Some(target),
                    expected_revision: snapshot.revision,
                },
            ),
            cx,
        );
        self.pending_relation_source = None;
        self.selected_tool = None;
        cx.notify();
    }
}
