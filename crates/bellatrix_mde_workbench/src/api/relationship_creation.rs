use super::*;

impl MdeWorkbenchItem {
    pub(super) fn connect_relation_view(
        &mut self,
        kind: ModelingRelationKind,
        target: String,
        point: Option<bellatrix_mde_cordis::Point>,
        cx: &mut Context<Self>,
    ) {
        self.connect_relation_endpoint(kind, Some(target), point, cx);
    }

    pub(super) fn connect_free_message_endpoint(
        &mut self,
        kind: ModelingRelationKind,
        point: bellatrix_mde_cordis::Point,
        cx: &mut Context<Self>,
    ) {
        if self
            .source
            .snapshot()
            .ok()
            .and_then(|snapshot| {
                snapshot
                    .section
                    .tools
                    .relation(kind)
                    .map(|metadata| metadata.message_kind.is_some())
            })
            .unwrap_or(false)
        {
            self.connect_relation_endpoint(kind, None, Some(point), cx);
        }
    }

    fn connect_relation_endpoint(
        &mut self,
        kind: ModelingRelationKind,
        target: Option<String>,
        point: Option<bellatrix_mde_cordis::Point>,
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
        let Some(metadata) = snapshot.section.tools.relation(kind) else {
            self.feedback = "Relationship tool metadata unavailable".into();
            cx.notify();
            return;
        };
        let Some(diagram) = snapshot
            .section
            .diagrams
            .iter()
            .find(|diagram| Some(diagram.id.as_str()) == self.presentation.active_diagram())
        else {
            self.feedback = "Select a diagram before connecting Views".into();
            cx.notify();
            return;
        };
        let target_view = target
            .as_deref()
            .and_then(|id| diagram.views.iter().find(|view| view.id == id));
        if target.is_some() && target_view.is_none() {
            self.feedback = "The clicked View no longer exists in this diagram".into();
            cx.notify();
            return;
        }
        let target_subject = target_view.and_then(|view| view.subject.clone());
        let message_kind = metadata.message_kind.as_deref();
        if self.presentation.pending_relation_source().is_none() {
            let valid_start = if message_kind == Some("found") {
                target.is_none()
            } else {
                target.is_some()
            };
            if !valid_start {
                self.feedback = metadata.endpoint_instruction.clone();
                cx.notify();
                return;
            }
            let change = self.presentation.begin_relation(PendingRelationEndpoint {
                project_id: self.project_id().clone(),
                diagram_id: diagram.id.clone(),
                view_id: target.clone(),
                subject_id: target_subject.clone(),
                revision: snapshot.revision,
                point,
            });
            self.apply_selection_change(change, cx);
            self.feedback = format!(
                "{}: source selected. {}",
                metadata.label, metadata.endpoint_instruction
            );
            cx.notify();
            return;
        }
        let Some(source) = self.presentation.pending_relation_source().cloned() else {
            return;
        };
        if source.project_id != *self.project_id()
            || source.diagram_id != diagram.id
            || source.revision != snapshot.revision
        {
            self.presentation.clear_relation_source();
            self.feedback =
                "The diagram changed after the source click. Select the source View again".into();
            cx.notify();
            return;
        }
        let source_view = source
            .view_id
            .as_deref()
            .and_then(|id| diagram.views.iter().find(|view| view.id == id));
        if source.view_id.is_some()
            && source_view.is_none_or(|view| view.subject != source.subject_id)
        {
            self.feedback = "The original source View changed; select the source again".into();
            self.presentation.clear_relation_source();
            cx.notify();
            return;
        }
        let valid_end = if message_kind == Some("lost") {
            target.is_none()
        } else {
            target.is_some()
        };
        if !valid_end {
            self.feedback = metadata.endpoint_instruction.clone();
            cx.notify();
            return;
        }
        let from = source.view_id.clone();
        let change = self
            .presentation
            .select_relation_target(target_subject.clone(), target.clone());
        self.apply_selection_change(change, cx);
        if !self.presentation.visual_draft_mode() && !metadata.is_visual_only {
            let (Some(source_view_id), Some(target_view_id)) = (from.clone(), target.clone())
            else {
                return;
            };
            let owner_id = match kind {
                ModelingRelationKind::Generalization
                | ModelingRelationKind::InterfaceRealization
                | ModelingRelationKind::TemplateBinding
                | ModelingRelationKind::Include
                | ModelingRelationKind::Extend
                | ModelingRelationKind::ElementImport
                | ModelingRelationKind::PackageImport
                | ModelingRelationKind::PackageMerge
                | ModelingRelationKind::Manifestation
                | ModelingRelationKind::Deployment => source.subject_id.clone(),
                // Diagram context and project root are not a choice of semantic owner.
                // Other relationship tools remain incomplete until an owner is selected.
                _ => None,
            };
            self.submit(
                MdeUiCommand::CreateRelationAtViewsOrVisualDraft(
                    bellatrix_mde_cordis::ModelingCreateRelationAtViewsRequested {
                        project_id: self.project_id().clone(),
                        diagram_id: diagram.id.clone(),
                        owner_id,
                        kind,
                        source_view_id,
                        target_view_id,
                        from: source.subject_id.clone(),
                        to: target_subject,
                        expected_revision: source.revision,
                    },
                ),
                cx,
            );
            self.presentation.cancel_tool();
            cx.notify();
            return;
        }
        let locate = |id: Option<&str>, authored: Option<bellatrix_mde_cordis::Point>| match id {
            Some(id) => diagram
                .views
                .iter()
                .find(|view| view.id == id)
                .and_then(|view| super::visual_draft_canvas::resolve_visual_center(&view.geometry)),
            None => authored,
        };
        let (Some(start), Some(to)) = (
            locate(from.as_deref(), source.point),
            locate(target.as_deref(), point),
        ) else {
            self.feedback = "The selected endpoints do not have usable connection geometry".into();
            cx.notify();
            return;
        };
        let mut properties = std::collections::BTreeMap::new();
        let source_point = source.point;
        if metadata.uses_occurrence_anchors {
            for (key, view_id, point) in [
                ("source_anchor", from.as_deref(), source_point),
                ("target_anchor", target.as_deref(), point),
            ] {
                if diagram.interaction_kind.as_deref() != Some("communication")
                    && let Some(view_id) = view_id
                    && let Some(anchor) = point.and_then(|point| {
                        crate::workbench_ui::visual_draft_canvas::resolve_message_anchor(
                            &snapshot.section,
                            diagram,
                            view_id,
                            point,
                        )
                    })
                {
                    properties.insert(key.into(), serde_json::json!({"x":anchor.x,"y":anchor.y}));
                }
            }
        }
        if let Some(message_kind) = message_kind {
            properties.insert("messageKind".into(), serde_json::json!(message_kind));
        }
        let label = if let Some(sort) = metadata.message_sort.as_deref() {
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
                    notation_kind: metadata.model_kind.clone(),
                    label,
                    geometry: Geometry::Path {
                        start,
                        segments: vec![PathSegment::Line { to }],
                    },
                    properties,
                    parent_view_id: None,
                    source_view_id: from,
                    target_view_id: target,
                    expected_revision: snapshot.revision,
                },
            ),
            cx,
        );
        self.presentation.cancel_tool();
        cx.notify();
    }
}
