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
            .and_then(|id| diagram.all_views().find(|view| view.id == id));
        if target.is_some() && target_view.is_none() {
            self.feedback = "The clicked View no longer exists in this diagram".into();
            cx.notify();
            return;
        }
        let target_subject = target_view.and_then(|view| view.subject.clone());
        if self.presentation.pending_relation_source().is_none() {
            let valid_start =
                MdeUiSource::accepts_relation_endpoint(metadata, true, target.is_some());
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
            .and_then(|id| diagram.all_views().find(|view| view.id == id));
        if source.view_id.is_some()
            && source_view.is_none_or(|view| view.subject != source.subject_id)
        {
            self.feedback = "The original source View changed; select the source again".into();
            self.presentation.clear_relation_source();
            cx.notify();
            return;
        }
        let valid_end = MdeUiSource::accepts_relation_endpoint(metadata, false, target.is_some());
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
        let result = self.source.connect_relation(
            &diagram.id,
            kind,
            (from.as_deref(), source.point),
            (target.as_deref(), point),
            self.presentation.visual_draft_mode(),
            source.revision,
        );
        let sent = result.is_ok();
        self.report_command_result(result, cx);
        if sent {
            self.presentation.cancel_tool();
        }
        cx.notify();
    }
}
