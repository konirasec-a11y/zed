use super::*;

impl MdeWorkbenchItem {
    pub fn select_element(&mut self, id: String, cx: &mut Context<Self>) {
        self.source.record_interaction(
            "selection.element",
            Some(&id),
            bellatrix_mde_cordis::TraceOutcome::Success,
        );
        let change = self.presentation.select_element(id);
        self.apply_selection_change(change, cx);
    }

    pub fn select_explorer_element(
        &mut self,
        id: String,
        key: String,
        occurrence: Option<bellatrix_mde_cordis::ViewOccurrence>,
        cx: &mut Context<Self>,
    ) {
        self.source.record_interaction(
            "selection.explorer_member",
            Some(&key),
            bellatrix_mde_cordis::TraceOutcome::Success,
        );
        let change = self
            .presentation
            .select_explorer_element(id, key, occurrence);
        self.apply_selection_change(change, cx);
    }

    pub(super) fn apply_selection_change(
        &mut self,
        change: SelectionChange,
        cx: &mut Context<Self>,
    ) {
        if !change.preserve_editor {
            self.inline_editor = None;
            self.inline_draft = None;
        }
        if change.selection_changed {
            self.gesture = None;
            cx.notify();
        }
    }

    pub(super) fn select_view(&mut self, subject: String, view_id: String, cx: &mut Context<Self>) {
        self.source.record_interaction(
            "selection.view",
            Some(&view_id),
            bellatrix_mde_cordis::TraceOutcome::Success,
        );
        let change = self.presentation.select_view(Some(subject), view_id);
        self.apply_selection_change(change, cx);
        cx.notify();
    }

    pub fn select_diagram_view(
        &mut self,
        diagram_id: String,
        view_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.source.is_presentation_suspended() {
            self.source.record_interaction(
                "selection.view.suspended",
                Some(&view_id),
                bellatrix_mde_cordis::TraceOutcome::Invalid,
            );
            return;
        }
        let selection = self.source.snapshot().and_then(|snapshot| {
            snapshot
                .section
                .diagrams
                .iter()
                .find(|diagram| diagram.id == diagram_id)
                .and_then(|diagram| diagram.all_views().find(|view| view.id == view_id))
                .map(|view| view.subject.clone())
                .ok_or_else(|| "The original diagram View no longer exists".to_owned())
        });
        let subject = match selection {
            Ok(subject) => subject,
            Err(error) => {
                self.source.record_interaction(
                    "selection.view.missing",
                    Some(&view_id),
                    bellatrix_mde_cordis::TraceOutcome::Invalid,
                );
                self.feedback = error;
                cx.notify();
                return;
            }
        };
        self.source.record_interaction(
            "selection.view",
            Some(&view_id),
            bellatrix_mde_cordis::TraceOutcome::Success,
        );
        self.select_diagram(diagram_id, cx);
        let change = self.presentation.select_view(subject, view_id);
        self.apply_selection_change(change, cx);
        self.feedback = if self.presentation.selected_element().is_none() {
            "Visual View selected; no semantic subject is required".into()
        } else {
            "Diagram View selected".into()
        };
        cx.notify();
    }

    pub(super) fn selected_view_is_inspection_only(&self) -> bool {
        let Some(view_id) = self.presentation.selected_view() else {
            return false;
        };
        self.source.snapshot().ok().is_none_or(|snapshot| {
            snapshot
                .section
                .diagrams
                .iter()
                .find(|diagram| Some(diagram.id.as_str()) == self.presentation.active_diagram())
                .is_none_or(|diagram| !selection::supports_view_editing(diagram, view_id))
        })
    }

    pub fn select_diagram(&mut self, id: String, cx: &mut Context<Self>) {
        self.source.record_interaction(
            "selection.diagram",
            Some(&id),
            bellatrix_mde_cordis::TraceOutcome::Success,
        );
        if self.presentation.select_diagram(id) {
            self.validation_status = "Not yet validated for this diagram".into();
        }
        self.gesture = None;
        self.inline_editor = None;
        self.inline_draft = None;
        cx.notify();
    }
}
